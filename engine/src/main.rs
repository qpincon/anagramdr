use itertools::Itertools;
use serde_json::Value;
use serde_derive::{Deserialize, Serialize};
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
use rustc_hash::FxHashMap;
use rayon::prelude::*;
use rand::seq::SliceRandom;
use rand::thread_rng;

const ALLOWED_CHARS: &str = "aàâäbcçdeéèêëfghiîïjklmnoôÔöÖpqrstuûüùvwxyz";
const MAX_EXPR_SIZE: usize = 6;
const MAX_MATCHABLE_WORDS: usize = 600;
const NB_BIG_WORDS_INCLUDED : usize = 30;
const MAX_QUERY_LETTERS: usize = 25;

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

// static PRIORITY_WORDS:  &'static [&'static str] = &["ce", "cet", "cette", "un", "une", "le", "la", "de", "du", "sur"];

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
    tagging_stats: FxHashMap<(PosMorph, PosMorph), f32>,
    pos_n_grams: FxHashMap<PosTagNGram, f32>,
}

#[derive(PartialEq, EnumString, Copy, Clone, Default, Serialize, Deserialize, Debug)]
enum SearchType {
    #[default]
    ROOT,
    EXACT,
}

#[derive(Serialize)]
struct AnagramResult {
    anagrams: Vec<(String, f32)>,
    was_truncated: bool
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
            tagging_stats: FxHashMap::default(),
            pos_n_grams: FxHashMap::default(),
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
                    // is_prio: PRIORITY_WORDS.iter().find(|&&x| x.eq(word)).is_some(),
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
                let key = (first, second);
                let occurences: f32 = (stat["nb"].as_u64().unwrap() as f32).sqrt();
                index.tagging_stats.insert(key, occurences);
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

    /**
     * Get matchable words, always including words containing the most letters
     * Returns true alongside vector if it was truncated randomly
     */
    fn get_matchable_words(&self, input_letters: &[u8], search_type: SearchType, word_to_include: &[u8],) -> Result<(Vec<&Word>, bool), String> {
        let mut words: Vec<&Word> = self.word_defs
            .iter()
            .filter(| w| {
                Index::check_contains_all_letters(
                    input_letters,
                    &self.sorted_letters[w.letters_sorted_range.start as usize
                        ..w.letters_sorted_range.end as usize],
                    search_type,
                )
                
            })
            .collect();

        // We will find the word, put it at the end of the array, and during the finding of anagrams,
        // this will remain the only root of the search tree
        if word_to_include.len() > 0 {
            let index = words
            .iter()
            .find_position(|w| {
                let searched = &self.original_letters[w.letters_sorted_range.start as usize..w.letters_sorted_range.end as usize];
                word_to_include.len() == searched.len() && searched.iter().zip(word_to_include.iter())
                .all(|(a, b)| encoded_chars_equal(*a, *b, search_type))
            });
            if index.is_some() {
                // println!("Word found! at index {}", index.unwrap().0);
                let removed = words.remove(index.unwrap().0);
                words.push(removed);
            } else {
                return Err(String::from("Le mot à inclure n'est pas contenu dans l'index"))
            }
            return Ok((words, false));
        }
        if words.len() <= MAX_MATCHABLE_WORDS {
            return Ok((words, false));
        }
        // println!("{} words before truncate", words.len());
        /* Always include NB_BIG_WORDS_INCLUDED bigger words */
        let mut rng = thread_rng();
        let mut result = Vec::new();

        let suffix_size = NB_BIG_WORDS_INCLUDED.min(words.len());
        let start_suffix = words.len().saturating_sub(suffix_size);
        let suffix = &words[start_suffix..];
        result.extend_from_slice(suffix);
        let remaining = &words[..start_suffix];
        let additional_size = (MAX_MATCHABLE_WORDS - suffix_size).min(remaining.len());
        let additional_elements = remaining.choose_multiple(&mut rng, additional_size);
        result.extend(additional_elements.cloned());
        result.sort_by(|a, b| {
            let length_a: u32 = a.letters_sorted_range.end - a.letters_sorted_range.start ;
            let length_b: u32 = b.letters_sorted_range.end - b.letters_sorted_range.start;
            return length_a.partial_cmp(&length_b).unwrap();
        });
        Ok((result, true))

    }

    /**
     * This algorithm is similar to the construction of a powerset of all words containing provided letters. See https://en.wikipedia.org/wiki/Power_set
     * The size of a powerset is 2^n. Of course this size is never reached since we remove letters from candidates as we get going.
     * It can still get pretty large, that's why there is a hard limit of candidates to find to not iterate forever and return early. 
     * TODO:
     * - If we have a lot of words matching letters, rank them by occurence in some reference corpora 
     */
    fn find_anagrams_reverse(&self, input: String, search_type: SearchType, word_to_include: String) -> Result<AnagramResult, String> {
        let max_cand_to_find = 10000;
        let mut nb_found = 0;
        let sorted_input = self.process_input(input);
        if sorted_input.len() > MAX_QUERY_LETTERS {
            return Err(format!("Trop de lettres ({}, le maximum est {})", sorted_input.len(), MAX_QUERY_LETTERS));
        }
        let mut candidates: Vec<Matching> = vec![];
        let mut enough_found = false;
        let mut mode_include = false;
        let mut processed_to_include : Vec<u8> = vec![];
        if word_to_include.len() > 0 {
            mode_include = true;
            processed_to_include = word_to_include.to_lowercase().chars()
            .filter(|x| ALLOWED_CHARS.chars().any(|c| c == *x))
            .map(char_to_u8)
            .collect()
        }
        // let start = Instant::now();

        let matchable_words_res = self.get_matchable_words(&sorted_input, search_type, &processed_to_include);
        if matchable_words_res.is_err() {
            return Err(matchable_words_res.unwrap_err());
        }
        let (matchable_words, was_truncated) = matchable_words_res.unwrap();
        let nb_matchable_words = matchable_words.len();
        // println!("{} matchabled words", matchable_words.len());
        for (index, word) in matchable_words.iter().enumerate().rev() {
            // println!("{}, {}", index, u8_to_str(&self.sorted_letters[word.letters_sorted_range.start as usize..word.letters_sorted_range.end as usize]));
            let searched_word_letters = &self.sorted_letters
                [word.letters_sorted_range.start as usize..word.letters_sorted_range.end as usize];
            let nb_cand = candidates.len();
            /* Search new candidates among current ones */
            for cand_index in 0..nb_cand {
                let candidate: &Matching = &candidates[cand_index];
                if candidate.is_complete {
                    continue;
                }
                let bloom_ok: bool =
                    (candidate.bloom_letters & word.bloom_letters) == word.bloom_letters;
                /* Create new candidate with the matching letters removed from the pool */
                let check_pass = candidate.matched_size < MAX_EXPR_SIZE as u8 && bloom_ok
                    && Index::check_contains_all_letters(
                        &candidate.letter_pool,
                        searched_word_letters,
                        search_type,
                    );
                /* Create new candidate with the matching letters removed from the pool */
                if check_pass {
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
            let should_add_new_cand = !mode_include || index == nb_matchable_words - 1;
            /* Find new candidates from scratch */
            if should_add_new_cand && Index::check_contains_all_letters(
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
                    bloom_letters,
                };
                if new_candidate.is_complete {
                    nb_found += 1;
                }
                candidates.push(new_candidate);
            }
        }
        
        // let start_scoring = Instant::now();
        let mut str_with_scores: Vec<(String, f32)> = candidates
            .into_par_iter()
            .filter(|m| m.is_complete)
            .map(|m| m.best_permutation(&self, &matchable_words))
            .collect();
        str_with_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        // println!("Time to find best permutations: {:.2?}", start_scoring.elapsed());
        // println!("Found {} anagrams", str_with_scores.len());

        Ok(AnagramResult { anagrams: str_with_scores, was_truncated})
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
}



impl<'a> Matching {

    fn best_permutation(&self, index: &Index, matchable_words: &[&Word]) -> (String, f32) {
        let mut best_perm = vec![];
        let mut best_score = -1.0;
        if self.matched_size == 1 {
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
        let nb_small_words = self.matched[..self.matched_size as usize].iter()
            .filter(|word_index| {
                let word = matchable_words[**word_index as usize];
                word.letters_sorted_range.end - word.letters_sorted_range.start <= 4
            })
            .count();
        let mut best_perm_score = best_score / (self.matched.len().pow(2) as f32);
        /* Penalize expression with lots of small words */
        best_perm_score = best_perm_score / (1.0 + nb_small_words as f32).powf(1.5);
        (self._matched_to_string(&best_perm, index, matchable_words), best_perm_score)
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
                    let second_pos_morph = PosMorph {
                        morph: *second_morph,
                        pos: second.pos_tag,
                    };
                    let key = (first_pos_morph, second_pos_morph);
                    let value = index.tagging_stats.get(&key);
                    if value.is_none() {
                        continue;
                    }
                    let occ = value.unwrap();
                    if *occ > best_inner_score {
                        best_inner_score = *occ;
                    };
                }
            }
            score += best_inner_score;
        }
        let last = &matchable_words[**combination.last().unwrap() as usize];
        /*  If last word is ADP, DET, PRON, VERB penalize current combination */
        if last.pos_tag == PosTag::ADP || last.pos_tag == PosTag::DET || last.pos_tag == PosTag::PRON || last.pos_tag == PosTag::VERB
        {
            score /= 4.0;
        }
        score
    }
}


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
    #[serde(default)]
    word_to_include: String,
}

// use std::mem;
#[tokio::main]
async fn main() {
    // println!("Size of word: {}", mem::size_of::<Word>());
    // println!("Size of matching: {}", mem::size_of::<Matching>());
    // println!("Size of Letters: {}", mem::size_of::<Letters>());
    // bench_estimate();


    let index: Index = Index::new();
    let route = warp::path!("engine"/"query")
    .and(warp::query::<QueryParams>())
    .map(move |q: QueryParams| {
            let query_input: String = decode(&q.input).expect("UTF-8").into_owned();
            // let before = Instant::now();
            let results = index.find_anagrams_reverse(query_input, q.search_type, q.word_to_include);
            // println!("Elapsed time: {:.2?}", before.elapsed());
            match results {
                Ok(res) => return warp::reply::with_status(warp::reply::json(&res), StatusCode::OK),
                Err(msg) => {
                    let json = warp::reply::json(&ErrorMessage {
                        code: StatusCode::BAD_REQUEST.as_u16(),
                        message: msg.into(),
                    });
                
                    return warp::reply::with_status(json, StatusCode::BAD_REQUEST);
                }
            }
            
        });
    warp::serve(route).run(([127, 0, 0, 1], 3030)).await;
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
        let _words = index.find_anagrams_reverse(query, SearchType::ROOT, String::from(""));
        println!("{}: {:.2?}", copy, before.elapsed());
    }
            
}

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
