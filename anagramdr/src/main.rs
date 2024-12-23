use itertools::Itertools;
use serde_json::Value;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead};
use std::ops::Range;
use std::path::Path;
use std::str::{self, FromStr};
use std::time::Instant;
use strum_macros::EnumString;
use unicode_normalization::char::{compose, decompose_canonical};
use urlencoding::decode;
use warp::Filter;
use warp::http::StatusCode;
const ALLOWED_CHARS: &str = "aàâäbcçdeéèêëfghiîïjklmnoôÔöÖpqrstuûüùvwxyz";
const MAX_EXPR_SIZE: usize = 6;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, EnumString, Hash, Copy, Clone)]
enum Gender {
    Fem,
    Masc,
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, EnumString, Hash, Copy, Clone)]
enum Number {
    Sing,
    Plur,
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, EnumString, Hash, Copy, Clone)]
enum Person {
    #[strum(serialize = "1")]
    One,
    #[strum(serialize = "2")]
    Two,
    #[strum(serialize = "3")]
    Three,
}

// https://universaldependencies.org/u/pos/
#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, EnumString, Hash, Copy, Clone)]
enum PosTag {
    ADJ,
    ADP,
    PUNC,
    ADV,
    AUX,
    SYM,
    INTJ,
    CCONJ,
    X,
    NOUN,
    DET,
    PROPN,
    NUM,
    VERB,
    PART,
    PRON,
    SCONJ,
    PUNCT,
    SPACE,
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, Hash, Copy, Clone)]
struct Morph {
    gender: Option<Gender>,
    number: Option<Number>,
    person: Option<Person>,
}

impl Morph {
    fn from_serde_map(serde_map: &serde_json::Map<String, serde_json::Value>) -> Morph {
        let mut morph = Morph {
            number: None,
            gender: None,
            person: None,
        };
        serde_map.iter().for_each(|(k, v)| match k.as_str() {
            "Number" => morph.number = Some(Number::from_str(v.as_str().unwrap()).unwrap()),
            "Gender" => morph.gender = Some(Gender::from_str(v.as_str().unwrap()).unwrap()),
            "Person" => morph.person = Some(Person::from_str(v.as_str().unwrap()).unwrap()),
            _ => unreachable!(),
        });
        morph
    }
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Copy, Clone)]
struct PosMorph {
    pos: PosTag,
    morph: Morph,
}

type Letters = Vec<u8>;
#[derive(Debug, Clone)]
struct Word {
    letters_sorted_range: Range<u32>,
    letters_original_range: Range<u32>,
    pos_tag: PosTag,
    morph_tags: Vec<Morph>,
    bloom_letters: u32,
    is_prio: bool,
}

// remove all elements from original that are in matched_words
fn remove_elems(original: &mut Vec<u8>, matched_word: &[u8], search_type: SearchType) {
    let lengths = (original.len(), matched_word.len()); // pool, searched
    let mut indexes = (0, 0); // pool, searched
    while indexes.0 < lengths.0 && indexes.1 < lengths.1 {
        if encoded_chars_equal(matched_word[indexes.1], original[indexes.0], search_type) {
            original.remove(indexes.0);
            indexes.1 += 1;
        } else if matched_word[indexes.1] > original[indexes.0] {
            indexes.0 += 1;
        }
    }
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filepath: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file: File = File::open(filepath)?;
    Ok(io::BufReader::new(file).lines())
}

type PosTagNGram = (
    Option<PosTag>,
    Option<PosTag>,
    Option<PosTag>,
    Option<PosTag>,
);

static PRIORITY_WORDS:  &'static [&'static str] = &["ce", "cet", "cette", "un", "une", "le", "la", "de", "du", "sur"];

#[derive(Clone)]
struct Index {
    /**
     * Contains the word as found in the entry corpus, as positions in "chars". If vocab is ["bonjour", "toi"] it will contain
     * bonjourtoi (as indexes)
     */
    original_letters: Letters,
    /**
     * Contains sorted letters of each word, punctuation removed and to lowercase, i.e "Très-étrange" will be
     * tresetrange (as indexes)
     */
    sorted_letters: Letters,
    /** Contain all the words of the entry vocab */
    word_defs: Vec<Word>,
    mean_word_size: f32,
    tagging_stats: HashMap<PosMorph, HashMap<PosMorph, f32>>,
    pos_n_grams: HashMap<PosTagNGram, f32>,
}

#[derive(PartialEq, EnumString, Copy, Clone, Default, Serialize, Deserialize, Debug)]
enum SearchType {
    #[default]
    ROOT,
    EXACT,
}

fn encoded_letters_to_bloom_u32(input: &[u8]) -> u32 {
    let mut bloom: u32 = 0;
    input.iter().for_each(|c| {
        bloom |= 1 << (c >> 3);
    });
    bloom
}

/** First (left) 5 bits are for the position, the rest 3 is for the identifer.
That leaves 8 possibilities for diacritics for the same char
 */
fn encode_char(ascii_pos: u8, diacritic_identifier: u8) -> u8 {
    ascii_pos << 3 | diacritic_identifier
}

fn diacritic_to_offset(diacritic_code: u32) -> u8 {
    match diacritic_code {
        768 => 1, // accent grave
        769 => 2, // accent aigu
        770 => 3, // circonflexe
        776 => 4, // trema
        807 => 5, // cedille
        _ => panic!("Unmatched diacritic"),
    }
}

fn offset_to_diacritic(diacritic_code: u8) -> Option<char> {
    match diacritic_code {
        0 => None,
        1 => Some(char::from_u32(768).unwrap()), // accent grave
        2 => Some(char::from_u32(769).unwrap()), // accent aigu
        3 => Some(char::from_u32(770).unwrap()), // circonflexe
        4 => Some(char::from_u32(776).unwrap()), // trema
        5 => Some(char::from_u32(807).unwrap()), // cedille
        _ => panic!("Unmatched offset"),
    }
}

fn u8_to_char(encoded: u8) -> char {
    let diacritic_identifier = encoded & 7;
    let ascii_pos = ((encoded >> 3) + 97) as u32;
    let diacritic = offset_to_diacritic(diacritic_identifier);
    if diacritic.is_none() {
        return char::from_u32(ascii_pos).unwrap();
    }
    compose(char::from_u32(ascii_pos).unwrap(), diacritic.unwrap()).unwrap()
}

fn char_to_u8(c: char) -> u8 {
    let mut base_char: char = ' ';
    let mut accent_index = 0;
    let mut decomposed_index = 0;
    decompose_canonical(c, |emitted| {
        // should always be 0 or 1, since we deal with simple latin values with accents
        if decomposed_index == 0 {
            base_char = emitted;
        } else {
            accent_index = diacritic_to_offset(emitted as u32);
        }
        decomposed_index += 1;
    });
    if (base_char as u32) < 97 || (base_char as u32) > 122 {
        panic!("Unsuported character {}", base_char);
    }
    // println!("{} {} {}", base_char, ((base_char as u32) - 97) as u8, accent_index);
    // 97 is unicode for 'a'
    encode_char(((base_char as u32) - 97) as u8, accent_index)
}

fn str_to_u8(string: &str) -> Vec<u8> {
    string.chars().map(char_to_u8).collect()
}

fn u8_to_str(u8: &[u8]) -> String {
    String::from_iter(u8.iter().map(|&encoded| u8_to_char(encoded)))
}

#[inline(always)]
fn encoded_chars_equal(a: u8, b: u8, search_type: SearchType) -> bool {
    if search_type == SearchType::EXACT {
        a == b
    } else {
        (a & 0b11111000) == (b & 0b11111000)
    }
}

impl Index {
    // construct the index from a jsonl file.
    // ASSUMES that the words are sorted by increasing length of letters
    fn new() -> Index {
        let mut index = Index {
            word_defs: vec![],
            sorted_letters: vec![],
            original_letters: vec![],
            mean_word_size: 0.0,
            tagging_stats: HashMap::new(),
            pos_n_grams: HashMap::new(),
        };

        let vocab_lines: io::Lines<io::BufReader<File>> =
            read_lines("data/words.jsonl").expect("Words file not found");
        for line in vocab_lines {
            if let Ok(word_def) = line {
                let word_def: Value = serde_json::from_str(&word_def).unwrap();
                let word = &word_def["word"].as_str().unwrap().to_lowercase();
                let lengths = (index.original_letters.len(), index.sorted_letters.len());
                if !word
                    .chars()
                    .all(|x| ALLOWED_CHARS.chars().any(|c| c == x))
                {
                    println!("{} not in character set: skipping", word);
                    continue;
                }
                index.mean_word_size += word.len() as f32;
                index.original_letters.extend_from_slice(&str_to_u8(word));
                let sorted_range: Vec<u8> = index.original_letters
                    [lengths.0..index.original_letters.len()]
                    .iter()
                    .cloned()
                    .sorted()
                    .collect();
                let bloom_letters = encoded_letters_to_bloom_u32(&sorted_range);
                index.sorted_letters.extend(sorted_range);
                let new_word_def = Word {
                    letters_original_range: lengths.0 as u32..index.original_letters.len() as u32,
                    letters_sorted_range: lengths.1 as u32..index.sorted_letters.len() as u32,
                    pos_tag: PosTag::from_str(word_def["pos"].as_str().unwrap()).unwrap(),
                    morph_tags: Index::build_morph_tags(word_def["morph"].as_array().unwrap()),
                    bloom_letters,
                    is_prio: PRIORITY_WORDS.iter().find(|&&x| x.eq(word)).is_some(),
                };
                index.word_defs.push(new_word_def);
            }
        }
        index.mean_word_size /= index.word_defs.len() as f32;

        let tagging_lines: io::Lines<io::BufReader<File>> =
            read_lines("data/tagging_stats.jsonl").expect("Tagging stats file not found");
        for line in tagging_lines {
            if let Ok(stat) = line {
                let stat: Value = serde_json::from_str(&stat).unwrap();
                let tagging = stat["tagging"].as_array().unwrap();
                let pos_1 = PosTag::from_str(tagging[0].as_str().unwrap()).unwrap();
                let morph_1 = Morph::from_serde_map(tagging[1].as_object().unwrap());
                let pos_2 = PosTag::from_str(tagging[2].as_str().unwrap()).unwrap();
                let morph_2 = Morph::from_serde_map(tagging[3].as_object().unwrap());
                let first = PosMorph {
                    pos: pos_1,
                    morph: morph_1,
                };
                let second = PosMorph {
                    pos: pos_2,
                    morph: morph_2,
                };
                let occurences: f32 = (stat["nb"].as_u64().unwrap() as f32).sqrt();
                index
                    .tagging_stats
                    .entry(first)
                    .and_modify(|dest_map| {
                        dest_map.insert(second, occurences);
                    })
                    .or_insert({
                        let mut new_map = HashMap::new();
                        new_map.insert(second, occurences);
                        new_map
                    });
            }
        }

        let pos_n_gram_lines: io::Lines<io::BufReader<File>> =
            read_lines("data/pos_n_grams.jsonl").expect("pos_n_grams file not found");
        for line in pos_n_gram_lines {
            if let Ok(stat) = line {
                let stat: Value = serde_json::from_str(&stat).unwrap();
                let occurences: f32 = (stat["occ"].as_u64().unwrap() as f32).sqrt();
                let mut ngram: Vec<Option<PosTag>> = stat["pos"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| Some(PosTag::from_str(x.as_str().unwrap()).unwrap()))
                    .collect();
                while ngram.len() < 4 {
                    ngram.push(None);
                }
                let t: PosTagNGram = ngram.iter().cloned().collect_tuple().unwrap();
                index.pos_n_grams.insert(t, occurences);
            }
        }
        index
    }

    fn build_morph_tags(morph: &[Value]) -> Vec<Morph> {
        morph
            .iter()
            .map(|val| -> Morph { Morph::from_serde_map(val.as_object().unwrap()) })
            .collect()
    }

    fn check_contains_all_letters(
        letter_pool: &[u8],
        searched: &[u8],
        search_type: SearchType,
    ) -> bool {
        // println!("{:?} ({}) vs {:?} ({})", letter_pool, u8_to_str(letter_pool), searched, u8_to_str(searched));
        let lengths = (letter_pool.len(), searched.len()); // pool, searched
        if lengths.1 > lengths.0 {
            return false;
        }
        let mut indexes = (0, 0); // pool, searched
        while indexes.0 < lengths.0 && indexes.1 < lengths.1 {
            if encoded_chars_equal(searched[indexes.1], letter_pool[indexes.0], search_type) {
                indexes.1 += 1;
            } 
            indexes.0 += 1;
            if indexes.0 == lengths.0 && indexes.1 < lengths.1 {
                return false;
            }
        }
        indexes.1 == lengths.1
    }

    fn new_vec_removed_letters(
        original: &[u8],
        matched_word: &[u8],
        search_type: SearchType,
    ) -> Letters {
        let lengths = (original.len(), matched_word.len()); // pool, searched
        let mut remaining: Vec<u8> = Vec::with_capacity(lengths.0 - lengths.1);
        let mut indexes = (0, 0); // pool, searched
        while indexes.0 < lengths.0 {
            if indexes.1 == lengths.1 {
                remaining.push(original[indexes.0]);
                indexes.0 += 1;
            } else if encoded_chars_equal(matched_word[indexes.1], original[indexes.0], search_type)
            {
                indexes.0 += 1;
                indexes.1 += 1;
            } else if matched_word[indexes.1] > original[indexes.0] {
                remaining.push(original[indexes.0]);
                indexes.0 += 1;
            }
        }
        remaining
    }

    fn process_input(&self, input: String) -> Letters {
        input
            .to_lowercase()
            .chars()
            .filter(|x| ALLOWED_CHARS.chars().any(|c| c == *x))
            .map(char_to_u8)
            .sorted()
            .collect()
    }

    // TODO: use is_prio to move up the letters in the array for us to be sure to include it in the search
    fn get_matchable_words(&self, input_letters: &[u8], search_type: SearchType) -> Vec<&Word> {
        self.word_defs
            .iter()
            .filter(|w: &&Word| {
                Index::check_contains_all_letters(
                    input_letters,
                    &self.sorted_letters[w.letters_sorted_range.start as usize
                        ..w.letters_sorted_range.end as usize],
                    search_type,
                )
            })
            .collect()
    }

    /**
     * This algorithm is similar to the construction of a powerset of all words containing provided letters. See https://en.wikipedia.org/wiki/Power_set
     * The size of a powerset is 2^n. Of course this size is never reached since we remove letters from candidates as we get going.
     * It can still get pretty large, that's why there is a hard limit of candidates to find to not iterate forever and return early. Since we iterate words in the order of their lengths,
     * this means that the expressions with smaller words will be skipped if we return early
     * TODO:
     * - If we have a lot of words matching letters, rank them by occurence in some reference corpora 
     */
    fn find_anagrams_reverse(&self, input: String, search_type: SearchType) -> Vec<(String, f32)> {
        let max_cand_to_find = 10000;
        // let mut nb_iter = 0;
        let mut nb_found = 0;
        let sorted_input = self.process_input(input);
        let input_length = sorted_input.len();
        let mut candidates: Vec<Matching> = vec![];
        // let mut nb_added_cand_scratch = 0;
        // let mut nb_added_cand_cand = 0;
        let mut enough_found = false;
        // let start = Instant::now();

        let matchable_words = self.get_matchable_words(&sorted_input, search_type);
        // println!("letters = {}", u8_to_str(&sorted_input));
        println!("{} matchable words", matchable_words.len());
        for (index, word) in matchable_words.iter().enumerate().rev() {
            // println!("{}, {:?}", index, word.letters_original_range);
        // for word in matchable_words.iter().rev() {
            let searched_word_letters = &self.sorted_letters
                [word.letters_sorted_range.start as usize..word.letters_sorted_range.end as usize];
            let cur_word_length = word.letters_sorted_range.end - word.letters_sorted_range.start;
            // if index % 200 == 0 {
                // let searched_word_original = &self.original_letters[word.letters_sorted_range.start
                //     as usize
                //     ..word.letters_sorted_range.end as usize];
                // println!(
                //     "Added candidates {} (scratch) {} (cloned), {} found",
                //     nb_added_cand_scratch, nb_added_cand_cand, nb_found
                // );
                // nb_added_cand_scratch = 0;
                // nb_added_cand_cand = 0;
                // println!(
                //     "{} / {}, {} candidates, word = {}",
                //     index,
                //     matchable_words.len(),
                //     candidates.len(),
                //     u8_to_str(searched_word_original)
                // );
            // }
            // nb_iter += 1;
            let nb_cand = candidates.len();
            /* Search new candidates among current ones */
            for cand_index in 0..nb_cand {
                let candidate: &Matching = &candidates[cand_index];
                // nb_iter += 1;
                if candidate.is_complete {
                    continue;
                }
                /* Only add new if the potential total of words is small enough relative to input size  */
                let should_add_new = input_length < 20
                    || candidates.len() < 300
                    || cur_word_length > 4
                    || (candidate.min_nb_words(cur_word_length) / input_length as f32) < 0.3;
                if !should_add_new {
                    continue;
                }
                let bloom_ok: bool =
                    (candidate.bloom_letters & word.bloom_letters) == word.bloom_letters;
                /* Create new candidate with the matching letters removed from the pool */
                let check_pass = bloom_ok
                    && Index::check_contains_all_letters(
                        &candidate.letter_pool,
                        searched_word_letters,
                        search_type,
                    );
                /* Create new candidate with the matching letters removed from the pool */
                if check_pass {
                    // println!("{} vs {}", u8_to_str(&self.sorted_letters[word.letters_sorted_range.start as usize..word.letters_sorted_range.end as usize]), u8_to_str(&candidate.letter_pool));
                    // nb_added_cand_cand += 1;
                    let mut new_cand = candidate.clone();
                    remove_elems(
                        &mut new_cand.letter_pool,
                        searched_word_letters,
                        search_type,
                    );
                    new_cand.is_complete = new_cand.letter_pool.len() == 0;
                    new_cand.bloom_letters = encoded_letters_to_bloom_u32(&new_cand.letter_pool);
                    new_cand.matched[new_cand.matched_size as usize] = index as u16;
                    new_cand.matched_size += 1;
                    if new_cand.is_complete {
                        nb_found += 1;
                        // new_cand.best_permutation(self);
                    }
                    if nb_found == max_cand_to_find {
                        enough_found = true;
                        break;
                    }
                    candidates.push(new_cand);
                }
            }
            if enough_found {
                break;
            }
            let should_add_new = input_length < 20
                || candidates.len() < 300
                || cur_word_length > 4
                || (cur_word_length as f32 / input_length as f32) > 0.2;
            /* Find new candidates from scratch */
            if should_add_new
                && Index::check_contains_all_letters(
                    &sorted_input,
                    searched_word_letters,
                    search_type,
                )
            {
                // remove letters from original pool
                let remaining_letters = Index::new_vec_removed_letters(
                    &sorted_input,
                    searched_word_letters,
                    search_type,
                );
                let length = remaining_letters.len();
                let bloom_letters = encoded_letters_to_bloom_u32(&remaining_letters);
                let mut new_matched = [u16::MAX; MAX_EXPR_SIZE];
                new_matched[0] = index as u16;
                let new_candidate = Matching {
                    letter_pool: remaining_letters,
                    is_complete: length == 0,
                    matched: new_matched,
                    matched_size: 1,
                    // best_perm_score: 0.0,
                    bloom_letters,
                };
                if new_candidate.is_complete {
                    nb_found += 1;
                    // new_candidate.best_permutation(self);
                }
                candidates.push(new_candidate);
                // nb_added_cand_scratch += 1;
            }
        }
        // println!("Finished search in {:.2?}", start.elapsed());
        // println!("{} candidate group", candidates.len());
        
        let str_with_scores = candidates
            .into_iter()
            .filter(|m| m.is_complete)
            .map(|m| m.best_permutation(&self, &matchable_words))
            .sorted_by(|a, b| b.1.partial_cmp(&a.1).unwrap())
            .collect();
        // println!(
        //     "Added candidates {} (scratch) {} (cloned) ",
        //     nb_added_cand_scratch, nb_added_cand_cand
        // );
        // println!(
        //     "Found {} anagrams",
        //     completed.iter().filter(|c| c.is_complete()).count()
        // );

        // println!("{} iterations", nb_iter);
        // println!("Total time: {:.2?}", start.elapsed());
        str_with_scores
    }

}

impl fmt::Display for Index {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // for word in &self.word_defs {
        //     let original = &self.original_letters[word.letters_original_range.start as usize..word.letters_original_range.end as usize];
        //     let sorted = &self.sorted_letters[word.letters_sorted_range.start as usize..word.letters_sorted_range.end as usize];
        //     writeln!(f, "{}, sorted : {}", self.u8_to_str(original), self.u8_to_str(sorted))?
        // }
        writeln!(
            f,
            "{} letters ({} sorted), {} words",
            self.original_letters.len(),
            self.sorted_letters.len(),
            self.word_defs.len()
        )?;

        // for two_gram in &self.tagging_stats {
        //     writeln!(f, "{:?} {:?}", two_gram.first, two_gram.second)?;
        // }
        // for (key_pos, dest_map) in self.tagging_stats.iter() {
        //     for (dest_pos, occ) in dest_map.iter() {
        //         writeln!(f, "{:?} {:?} {}", key_pos, dest_pos, occ)?;
        //     }
        // }

        writeln!(f, "Mean letter count per word: {}", self.mean_word_size)?;
        Ok(())
    }
}

fn pos_tuple_from_words(words_indexes: &Vec<&u16>, matchable_words: &[&Word]) -> PosTagNGram {
    let mut pos: Vec<Option<PosTag>> = words_indexes.iter().map(|x| Some(matchable_words[**x as usize ] .pos_tag)).collect();
    while pos.len() != 4 {
        if pos.len() < 4 {
            pos.push(None);
        } else if pos.len() > 4 {
            pos.pop();
        }
    }
    pos.into_iter().collect_tuple().unwrap()
}

#[derive(Debug, Clone)]
struct Matching {
    letter_pool: Letters,
    is_complete: bool,
    bloom_letters: u32,
    /** No more than MAX_EXPR_SIZE words can be matched. Indexes to "matchable_words" */
    matched: [u16; MAX_EXPR_SIZE],
    matched_size: u8,
    // matched: Vec<&'a Word>,
    // best_perm_score: f32,
}



impl<'a> Matching {

    fn min_nb_words(&self, word_length: u32) -> f32 {
        (self.matched_size as f32) + (self.letter_pool.len() as f32 / word_length as f32).ceil()
    }


    // fn iter_matches<'b>(
    //     &'b self,
    //     matchables_words: &'b[Word],
    // ) -> impl Iterator<Item = &'b Word> + 'b {
    //     self.matched.iter().map(move |&i| &matchables_words[i as usize])
    // }

    fn best_permutation(&self, index: &Index, matchable_words: &[&Word]) -> (String, f32) {
        let mut best_perm = vec![];
        let mut best_score = -1.0;
        if self.matched_size == 1 {
            // self.best_perm_score = f32::MAX;
            return (self.matched_to_string(&self.matched, index, matchable_words), f32::MAX);
        }
        self.matched[..self.matched_size as usize]
            .iter()
            .permutations(self.matched_size as usize)
            .for_each(|combination: Vec<&u16>| {
                let mut score = Matching::score_combination(&combination, index, matchable_words);
                let pos_n_gram: (Option<PosTag>, Option<PosTag>, Option<PosTag>, Option<PosTag>) = pos_tuple_from_words(&combination, &matchable_words);
                if let Some(occs) = index.pos_n_grams.get(&pos_n_gram) {
                    score *= occs;

                }
                if score > best_score {
                    best_score = score;
                    best_perm = combination;
                }
            });
        /* Boost 2-word expressions */
        if self.matched.len() == 2 {
            best_score *= 15.0;
        }
        
        let best_perm_score = best_score / (self.matched.len().pow(4) as f32);
        (self._matched_to_string(&best_perm, index, matchable_words), best_perm_score)
        // (best_perm_str, best_perm_score)
    }

    fn matched_to_string(&self, matched: &[u16], index: &Index, matchable_words: &[&Word]) -> String {
        matched.iter()
            .take(self.matched_size as usize)
            .map(|word_index| {
                let w: &Word = &matchable_words[*word_index as usize];
                u8_to_str(
                    &index.original_letters[w.letters_original_range.start as usize
                        ..w.letters_original_range.end as usize],
                )
            })
            .join(" ")

    }

    fn _matched_to_string(&self, matched: &[&u16], index: &Index, matchable_words: &[&Word]) -> String {
        let m: Vec<u16> = matched.iter().map(|&&x| x).collect();
        self.matched_to_string(&m, index, matchable_words)
    }
    /**
     * Expections:
     * - If last word is ADP, DET or PRON, penalize current combination
     */
    fn score_combination(combination: &Vec<&u16>, index: &Index, matchable_words: &[&Word]) -> f32 {
        let mut score = 0.0;
        for window in combination.windows(2) {
            let first = &matchable_words[*window[0] as usize];
            let second = &matchable_words[*window[1] as usize];
            let mut best_inner_score = 0.0;
            for first_morph in &first.morph_tags {
                for second_morph in &second.morph_tags {
                    let first_pos_morph = PosMorph {
                        morph: *first_morph,
                        pos: first.pos_tag,
                    };
                    let dest_map = index.tagging_stats.get(&first_pos_morph);
                    if dest_map.is_none() {
                        continue;
                    }
                    let second_pos_morph = PosMorph {
                        morph: *second_morph,
                        pos: second.pos_tag,
                    };
                    let stats_occ = dest_map.unwrap().get(&second_pos_morph);
                    if stats_occ.is_none() {
                        continue;
                    }
                    let occ = stats_occ.unwrap();
                    if *occ > best_inner_score {
                        best_inner_score = *occ;
                    };
                }
            }
            // println!("'{}' and '{}' scored {}",
            //     index.u8_to_str(&index.original_letters[first.letters_original_range.start as usize..first.letters_original_range.end as usize]),
            //     index.u8_to_str(&index.original_letters[second.letters_original_range.start as usize..second.letters_original_range.end as usize]),
            //     best_inner_score,
            // );
            score += best_inner_score;
        }
        let last = &matchable_words[**combination.last().unwrap() as usize];
        if last.pos_tag == PosTag::ADP
            || last.pos_tag == PosTag::DET
            || last.pos_tag == PosTag::PRON
        {
            score /= 2.0;
        }
        score
    }
}
use std::mem;

/// An API error serializable to JSON.
#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct QueryParams {
    input: String,
    #[serde(default)]
    search_type: SearchType,
}
#[tokio::main]
async fn main() {
    println!("Size of word: {}", mem::size_of::<Word>());
    println!("Size of matching: {}", mem::size_of::<Matching>());
    println!("Size of Letters: {}", mem::size_of::<Letters>());
    // let before = Instant::now();
    // println!("{}", index);
    // println!("Took: {:.2?} to build index", before.elapsed());
    // let cors = warp::cors().allow_any_origin();
    // let route = warp::path!("query" / String / String)
    //     .map(move |input_sentence: String, search_type: String| {
    //         let search_type = SearchType::from_str(&search_type).unwrap();
    //         let input: String = decode(&input_sentence).expect("UTF-8").into_owned();
    //         let before = Instant::now();
    //         let words = index.find_anagrams_reverse(input, search_type);
    //         println!("Elapsed time: {:.2?}", before.elapsed());
    //         warp::reply::json(&words)
    //     })
    //     .with(cors);
    
    bench_estimate();


    // let index: Index = Index::new();
    // let route = warp::path!("query")
    // .and(warp::query::<QueryParams>())
    // .map(move |q: QueryParams| {
    //         let query_input: String = decode(&q.input).expect("UTF-8").into_owned();
    //         if query_input.len() > 20 {
    //             let json = warp::reply::json(&ErrorMessage {
    //                 code: StatusCode::BAD_REQUEST.as_u16(),
    //                 message: "Too many letters".into(),
    //             });
            
    //             return warp::reply::with_status(json, StatusCode::BAD_REQUEST);
    //         }
    //         let before = Instant::now();
    //         let words = index.find_anagrams_reverse(query_input, q.search_type);
    //         println!("Elapsed time: {:.2?}", before.elapsed());
    //         return warp::reply::with_status(warp::reply::json(&words), StatusCode::OK);
    //     });
    // warp::serve(route).run(([127, 0, 0, 1], 3030)).await;
}


fn bench_estimate() {
    let index: Index = Index::new();
    let queries = [
        String::from("montceau les mines"),
        String::from("alain chabat le meilleur"),
        String::from("le marquis de sade"),
        String::from("j'ai la belle vie madame"),
    ];

    // println!("{}", index);
    for query in queries {
        let before = Instant::now();
        let copy: String = query.clone();
        let words = index.find_anagrams_reverse(query, SearchType::ROOT);
        println!("{}: {:.2?}", copy, before.elapsed());
    }
            
}
// fn main() {
//     let index: Index = Index::new();
//     for _ in 0..20i64 {
//         let words = index.find_anagrams_reverse(String::from("bien le bonjour madame"), SearchType::ROOT);
//         assert_eq!(words.len(), 9999);
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    fn str_to_sorted_encoded(input: &str) -> Vec<u8> {
        let mut encoded = str_to_u8(input);
        encoded.sort();
        encoded
    }

    #[test]
    fn char_encoding_decoding() {
        let encoded = char_to_u8('e');
        assert_eq!(encoded, 0b00100000); // e is 5th position, so '4'
        assert_eq!(u8_to_char(0b00100000), 'e');
        let encoded = char_to_u8('é');
        assert_eq!(encoded, 0b00100010);
        assert_eq!(u8_to_char(0b00100010), 'é');
        let encoded = char_to_u8('è');
        assert_eq!(encoded, 0b00100001);
        assert_eq!(u8_to_char(0b00100001), 'è');
        let encoded = char_to_u8('ê');
        assert_eq!(encoded, 0b00100011);
        assert_eq!(u8_to_char(0b00100011), 'ê');
        let encoded = char_to_u8('ë');
        assert_eq!(encoded, 0b00100100);
        assert_eq!(u8_to_char(0b00100100), 'ë');
    }

    #[test]
    fn str_encoding_decoding() {
        let encoded = str_to_u8("montceaulesmines");
        assert_eq!(u8_to_str(&encoded),"montceaulesmines");
    }

    #[test]
    fn encoded_comparison() {
        assert_eq!(
            Index::check_contains_all_letters(
                &str_to_sorted_encoded("efforça"),
                &str_to_sorted_encoded("efforça"),
                SearchType::EXACT
            ),
            true
        );
        assert_eq!(
            Index::check_contains_all_letters(
                &str_to_sorted_encoded("abcdefg"),
                &str_to_sorted_encoded("efg"),
                SearchType::EXACT
            ),
            true
        );
        assert_eq!(
            Index::check_contains_all_letters(
                &str_to_sorted_encoded("abcdefg"),
                &str_to_sorted_encoded("abc"),
                SearchType::EXACT
            ),
            true
        );
        assert_eq!(
            Index::check_contains_all_letters(
                &str_to_sorted_encoded("abcdefg"),
                &str_to_sorted_encoded("abh"),
                SearchType::EXACT
            ),
            false
        );
        assert_eq!(
            Index::check_contains_all_letters(
                &str_to_sorted_encoded("efforça"),
                &str_to_sorted_encoded("efforça"),
                SearchType::ROOT
            ),
            true
        );
        assert_eq!(
            Index::check_contains_all_letters(
                &str_to_sorted_encoded("efforca"),
                &str_to_sorted_encoded("efforça"),
                SearchType::ROOT
            ),
            true
        );
        assert_eq!(
            Index::check_contains_all_letters(
                &str_to_sorted_encoded("efforca"),
                &str_to_sorted_encoded("efforça"),
                SearchType::EXACT
            ),
            false
        );
    }

    #[test]
    fn encoded_comparison_new_vec() {
        assert_eq!(
            Index::new_vec_removed_letters(
                &str_to_sorted_encoded("efforça"),
                &str_to_sorted_encoded("efforç"),
                SearchType::EXACT
            ),
            [0]
        );
        let missing = char_to_u8('ç');
        assert_eq!(
            Index::new_vec_removed_letters(
                &str_to_sorted_encoded("efforça"),
                &str_to_sorted_encoded("effora"),
                SearchType::EXACT
            ),
            [missing]
        );
    }

    #[test]
    fn bloom_filter_test() {
        let bloom1 = encoded_letters_to_bloom_u32(&str_to_sorted_encoded("abcdef"));
        assert_eq!(bloom1, (2 as u32).pow(6) - 1);
        let bloom2 = encoded_letters_to_bloom_u32(&str_to_sorted_encoded("abcdef"));
        assert!((bloom1 & bloom2) == bloom1);
        let bloom2 = encoded_letters_to_bloom_u32(&str_to_sorted_encoded("bonjou"));
        assert!((bloom1 & bloom2) != bloom2);
        let bloom1 = encoded_letters_to_bloom_u32(&str_to_sorted_encoded("bonjouuuuu"));
        assert!((bloom1 & bloom2) == bloom2);

        let bloom1 = encoded_letters_to_bloom_u32(&str_to_sorted_encoded("deéélsu"));
        let bloom2 = encoded_letters_to_bloom_u32(&str_to_sorted_encoded("deelqsu"));
        assert!((bloom1 & bloom2) != bloom2);
    }
}
