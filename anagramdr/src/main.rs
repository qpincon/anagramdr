use std::time::Instant;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use serde_json::{de, Value};
use std::collections::{HashMap, HashSet};
use std::str::{self, FromStr};
use std::ops::Range;
use std::fmt;
use strum_macros::EnumString;
use itertools::Itertools;

const CHARS_TO_REMOVE : &'static str = " ,-"; // chars to remove for processing, but keep for storing words
const ALLOWED_CHARS : &'static str = "aAàÀâÂäÄbBcCçÇdDeEéÉèÈêÊëËfFgGhHiIîÎïÏjJkKlLmMnNoOôÔöÖpPqQrRsStTuûüUvVwWxXyYzZ ',-"; // must contain CHARS_TO_REMOVE


#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, EnumString, Hash, Copy, Clone)]
enum Gender {
    Fem,
    Masc
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, EnumString, Hash, Copy, Clone)]
enum Number {
    Sing,
    Plur
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


#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, Hash,Copy, Clone)]
struct Morph {
    gender: Option<Gender>,
    number: Option<Number>,
    person: Option<Person>,
}

impl Morph {
    fn from_serde_map(serde_map: &serde_json::Map<String, serde_json::Value>) -> Morph {
        let mut morph = Morph{number: None, gender: None, person: None};
        serde_map.iter().for_each(|(k, v)| {
            match k.as_str() {
                "Number" => {morph.number = Some(Number::from_str(v.as_str().unwrap()).unwrap())},
                "Gender" => {morph.gender = Some(Gender::from_str(v.as_str().unwrap()).unwrap())},
                "Person" => {morph.person = Some(Person::from_str(v.as_str().unwrap()).unwrap())},
                _ => unreachable!()
            }
        });
        morph
    }
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Copy, Clone)]
struct PosMorph {
    pos: PosTag,
    morph: Morph
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
struct PosMorph2Gram {
    first: PosMorph,
    second: PosMorph,
    nb_occ: u32
}


type Letters = Vec<u8>;
#[derive(Debug)]
struct Word {
    letters_sorted_range: Range<u32>,
    letters_original_range: Range<u32>,
    pos_tag: PosTag,
    morph_tags: Vec<Morph>,
}


fn find_or_insert<T> (list: &mut Vec<T>, elem: T) -> usize 
        where T: Clone + PartialOrd
{
    match list.iter().position(|e| e == &elem) {
        Some(pos) => pos + 1,
        None => {
            list.push(elem.clone());
            list.len()
        }
    }
}

// remove all elements from original that are in matched_words
fn remove_elems<T>(original: &mut Vec<T>, matched_word: &[T]) where T: PartialOrd {
    let lengths = (original.len(), matched_word.len()); // pool, searched
    let mut indexes = (0, 0); // pool, searched
    while indexes.0 < lengths.0 && indexes.1 < lengths.1 {
        if matched_word[indexes.1] > original[indexes.0] { 
            indexes.0 += 1;
        }
        else if matched_word[indexes.1] == original[indexes.0] {
            original.remove(indexes.0);
            indexes.1 += 1; 
        }
    }
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filepath: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file: File = File::open(filepath)?;
    Ok(io::BufReader::new(file).lines())
}


struct Index {
    /** Character to position in "chars" */
    char_mapping: HashMap<char, u8>,
    /** All characters in index, from 0 to ALLOWED_CHARS length */
    chars: Vec<char>,
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
    /** Character to position in "chars" to remove */ 
    chars_to_remove: HashSet<u8>,
    /** Uppercase variant to lowercase */
    uppercase_mapping: HashMap<u8, u8>,
    /** Contain all the words of the entry vocab */
    word_defs: Vec<Word>,
    mean_word_size: f32,
    // tagging_stats: Vec<PosMorph2Gram>,
    tagging_stats: HashMap<PosMorph, HashMap<PosMorph, f32>>,
    tagging_stats_total: u64,
}

impl Index {
    // construct the index from a jsonl file.
    // ASSUMES that the words are sorted by increasing length of letters
    fn new() -> Index {
        let mut index = Index {
            char_mapping: HashMap::new(),
            chars: vec![],
            chars_to_remove: HashSet::new(),
            uppercase_mapping: HashMap::new(),
            word_defs: vec![],
            sorted_letters: vec![],
            original_letters: vec![],
            mean_word_size: 0.0,
            tagging_stats: HashMap::new(),
            tagging_stats_total: 0,
            // tagging_stats: vec![]
        };
        ALLOWED_CHARS.chars().for_each(|c| {
            index.char_mapping.insert(c, index.chars.len() as u8);
            index.chars.push(c);
        });
        ALLOWED_CHARS.chars().for_each(|c| {
            let mut lower = c.to_lowercase();
            let first_char = lower.next().unwrap();
            if first_char != c {
                index.uppercase_mapping.insert(*index.char_mapping.get(&c).unwrap(), *index.char_mapping.get(&first_char).unwrap());
            }
        });
        CHARS_TO_REMOVE.chars().for_each(|c| {
            index.chars_to_remove.insert(*index.char_mapping.get(&c).unwrap());
        });
        let vocab_lines: io::Lines<io::BufReader<File>> = read_lines("data/words.jsonl").expect("Words file not found");
        for line in vocab_lines {
            if let Ok(word_def) = line {
                let word_def : Value = serde_json::from_str(&word_def).unwrap();
                let word = word_def["word"].as_str().unwrap();
                let lengths = (index.original_letters.len(), index.sorted_letters.len());
                if !word.chars().all(|x| index.char_mapping.contains_key(&x)) {
                    println!("{} not in character set: skipping", word);
                    continue;
                }
                index.mean_word_size += word.len() as f32;
                index.original_letters.extend_from_slice(&index.str_to_u8(word));
                 // remove punct and map uppercase to lowercase
                let mut sorted_range : Vec<u8> = index.original_letters[lengths.0..index.original_letters.len()].iter()
                    .filter(|c| !index.chars_to_remove.contains(c))
                    .map(|c| {
                        match index.uppercase_mapping.get(c) {
                            Some(lower) => *lower,
                            None => *c
                        }
                    }).collect();
                sorted_range.sort();
                index.sorted_letters.extend(sorted_range);
                let new_word_def = Word {
                    letters_original_range: lengths.0 as u32..index.original_letters.len() as u32,
                    letters_sorted_range: lengths.1 as u32..index.sorted_letters.len() as u32,
                    pos_tag: PosTag::from_str(word_def["pos"].as_str().unwrap()).unwrap(),
                    morph_tags: Index::build_morph_tags(word_def["morph"].as_array().unwrap()),
                };
                index.word_defs.push(new_word_def);
            }
        }
        index.mean_word_size = index.mean_word_size / index.word_defs.len() as f32;

        let tagging_lines: io::Lines<io::BufReader<File>> = read_lines("data/tagging_stats.jsonl").expect("Tagging stats file not found");
        let mut total_occurences: u64 = 0;
        for line in tagging_lines {
            if let Ok(stat) = line {
                let stat: Value = serde_json::from_str(&stat).unwrap();
                let tagging = stat["tagging"].as_array().unwrap();
                let pos_1 = PosTag::from_str(tagging[0].as_str().unwrap()).unwrap();
                let morph_1 = Morph::from_serde_map(tagging[1].as_object().unwrap());
                let pos_2 = PosTag::from_str(tagging[2].as_str().unwrap()).unwrap();
                let morph_2 = Morph::from_serde_map(tagging[3].as_object().unwrap());
                let first = PosMorph{pos: pos_1, morph: morph_1};
                let second = PosMorph{pos: pos_2, morph: morph_2};
                let occurences = stat["nb"].as_u64().unwrap() as f32;
                total_occurences += occurences as u64;
                index.tagging_stats.entry(first).and_modify(|dest_map| {
                    dest_map.insert(second, occurences);
                }).or_insert( {
                    let mut new_map = HashMap::new();
                    new_map.insert(second, occurences);
                    new_map
                });
            }
        }
        index.tagging_stats_total = total_occurences;
        index
    }

    fn str_to_u8(&self, string: &str) -> Vec<u8> {
        string.chars().map(|c| *self.char_mapping.get(&c).unwrap()).collect()
    }

    fn u8_to_str(&self, indexes: &[u8]) -> String {
        indexes.iter().map(|&c| self.chars[c as usize]).collect()
    }

    fn build_morph_tags(morph: &Vec<Value>) -> Vec<Morph> {
        morph.iter().map(|val| -> Morph {
            Morph::from_serde_map(val.as_object().unwrap())
        }).collect()
    }

    fn check_contains_all_letters(letter_pool: &[u8], searched: &[u8]) -> bool {
        let lengths = (letter_pool.len(), searched.len()); // pool, searched
        if lengths.1 > lengths.0 { return false; }
        let mut indexes = (0, 0); // pool, searched
        while indexes.0 < lengths.0 && indexes.1 < lengths.1 {
            if searched[indexes.1] < letter_pool[indexes.0] { return false; }
            if searched[indexes.1] > letter_pool[indexes.0] { indexes.0 += 1; }
            else if searched[indexes.1] == letter_pool[indexes.0] {
                indexes.0 += 1; indexes.1 += 1; 
            }
        }
        indexes.1 == lengths.1
    }

    fn new_vec_removed_letters(original: &[u8], matched_word: &[u8]) -> Letters {
        let lengths = (original.len(), matched_word.len()); // pool, searched
        let mut remaining : Vec<u8> = Vec::with_capacity(lengths.0 - lengths.1);
        let mut indexes = (0, 0); // pool, searched
        while indexes.0 < lengths.0 {
            if indexes.1 == lengths.1 || matched_word[indexes.1] > original[indexes.0] { 
                remaining.push(original[indexes.0]); 
                indexes.0 += 1;
            }
            else if matched_word[indexes.1] == original[indexes.0] {
                indexes.0 += 1; indexes.1 += 1; 
            }
        }
        remaining
    }

    fn process_input(&self, input : String) -> Letters {
        let space = self.char_mapping.get(&' ').unwrap();
        let mut encoded : Letters = input.chars()
            .map(|c| {
                let c = self.char_mapping.get(&c).unwrap_or(space);
                match self.uppercase_mapping.get(c) {
                    Some(lowercase) => *lowercase,
                    None => *c
                }
            })
            .filter(|c| !self.chars_to_remove.contains(&c))
            .collect();
        encoded.sort();
        encoded
    }

    fn find_anagrams_reverse(&self, input: String) {
        let before = Instant::now();
        let max_cand_to_find = 300;
        let mut nb_iter = 0;
        let mut nb_found = 0;
        let sorted_input = self.process_input(input);
        let input_length = sorted_input.len();
        let mut candidates : Vec<Matching> = vec![];
        let mut index = 0;
        let mut nb_added_cand_scratch = 0;
        let mut nb_added_cand_cand = 0;
        let mut enough_found = false;
        println!("letters = {}",  self.u8_to_str(&sorted_input));
        for word in self.word_defs.iter().rev() {
            let searched_word_letters = &self.sorted_letters[word.letters_sorted_range.start as usize..word.letters_sorted_range.end as usize];
            let cur_word_length = word.letters_sorted_range.end - word.letters_sorted_range.start;
            if index % 200 == 0 {
                let searched_word_original = &self.original_letters[word.letters_sorted_range.start as usize..word.letters_sorted_range.end as usize];
                println!("Added candidates {} (scratch) {} (cloned), {} found", nb_added_cand_scratch, nb_added_cand_cand, nb_found);
                nb_added_cand_scratch = 0;
                nb_added_cand_cand = 0;
                println!("{} / {}, {} candidates, word = {}", index, self.word_defs.len(), candidates.len(), self.u8_to_str(&searched_word_original));
            }
            index += 1;
            nb_iter += 1;
            let nb_cand = candidates.len();
            /* Search new candidates among current ones */
            for cand_index in 0..nb_cand {
                let candidate = &candidates[cand_index];
                nb_iter += 1;
                if candidate.letter_pool.len() == 0 { continue; }
                /* Only add new if the potential total of words is small enough relative to input size  */
                let should_add_new = cur_word_length > 4 || (candidate.min_nb_words(cur_word_length) / input_length as f32) < 0.3;
                if !should_add_new { continue; }
                let check_pass = Index::check_contains_all_letters(&candidate.letter_pool, searched_word_letters);
                /* Create new candidate with the matching letters removed from the pool */
                if check_pass {
                    nb_added_cand_cand += 1;
                    let mut new_cand = candidate.clone();
                    remove_elems(&mut new_cand.letter_pool, searched_word_letters);
                    new_cand.matched.push(&word);
                    if new_cand.is_complete() { 
                        nb_found+= 1;
                        new_cand.best_permutation(&self);
                    }
                    if nb_found == max_cand_to_find {
                        enough_found = true;
                        break;
                    }
                    candidates.push(new_cand);
                }
            }
            if enough_found { break; }
            let should_add_new = cur_word_length > 4 || (cur_word_length as f32 / input_length as f32) > 0.2;
            /* Find new candidates from scratch */
            if should_add_new && Index::check_contains_all_letters(&sorted_input, searched_word_letters) {
                // remove letters from original pool
                let remaining_letters = Index::new_vec_removed_letters(&sorted_input, searched_word_letters);
                let mut new_candidate = Matching { letter_pool: remaining_letters, matched: vec![word], best_perm_score: 0.0};
                if new_candidate.is_complete() { 
                    nb_found+= 1;
                    new_candidate.best_permutation(&self);
                }
                candidates.push(new_candidate);
                nb_added_cand_scratch += 1;
            }
        }
        candidates.sort_by(|a, b| b.best_perm_score.partial_cmp(&a.best_perm_score).unwrap());
        for cand in &candidates {
            if cand.letter_pool.len() == 0 {
                self.print_matching(cand);
            }
        }
        println!("Added candidates {} (scratch) {} (cloned) ", nb_added_cand_scratch, nb_added_cand_cand);
        println!("Found {} anagrams", candidates.iter().filter(|c| c.is_complete()).count());
        println!("{} candidate group", candidates.len());
        println!("{} iterations", nb_iter);
        println!("Elapsed time: {:.2?}", before.elapsed());
    }

    fn print_matching(&self, matching: &Matching) {
        if matching.letter_pool.len() > 0 {
            println!("Remaining letters: {}", self.u8_to_str(&matching.letter_pool));
        }
        println!("matched {} words:", matching.matched.len());
        for word in &matching.matched {
            print!("{} ", self.u8_to_str(&self.original_letters[word.letters_original_range.start as usize..word.letters_original_range.end as usize]));
        }
        print!("(score = {})", matching.best_perm_score);
        println!();
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
        writeln!(f, "{} letters ({} sorted), {} words", self.original_letters.len(), self.sorted_letters.len(), self.word_defs.len())?;
        // for (key, val) in self.uppercase_mapping.iter() {
        //     writeln!(f, "{}: {}", self.chars[*key as usize], self.chars[*val as usize]);
        // }
        
        // for two_gram in &self.tagging_stats {
        //     writeln!(f, "{:?} {:?}", two_gram.first, two_gram.second)?;
        // }
        for (key_pos, dest_map) in self.tagging_stats.iter() {
            for (dest_pos, occ) in dest_map.iter() {
                writeln!(f, "{:?} {:?} {}", key_pos, dest_pos, occ)?;
            }
        }
        
        writeln!(f, "Mean letter count per word: {}", self.mean_word_size)?;
        Ok(())
    }
}


#[derive(Debug, Clone)]
struct Matching<'a> {
    letter_pool: Letters,
    matched: Vec<&'a Word>,
    best_perm_score: f32,
}

impl<'a> Matching<'a> {
    fn is_complete(&self) -> bool {
        self.letter_pool.len() == 0
    }

    fn min_nb_words(&self, word_length: u32) -> f32 {
        (self.matched.len() as f32) + (self.letter_pool.len() as f32 / word_length as f32).ceil()
    }

    fn best_permutation(&mut self, index: &Index) {
        let mut best_perm = vec![];
        let mut best_score = 0.0;
        if self.matched.len() == 1 {
            self.best_perm_score = f32::MAX;
            return;
        }
        self.matched.iter().permutations(self.matched.len()).for_each(|combination| {
            let score = Matching::score_combination(&combination, index);
            if score > best_score {
                best_score = score;
                best_perm = combination;
            }
        });
        self.matched = best_perm.into_iter().cloned().collect();
        self.best_perm_score = best_score / (self.matched.len() as f32);
    }

    fn score_combination(combination: &Vec<&&Word>, index: &Index) -> f32 {
        let mut score = 0.0;
        for window in combination.windows(2) {
            let first = window[0];
            let second = window[1];
            let mut best_inner_score = 0.0;
            for first_morph in &first.morph_tags {
                for second_morph in &second.morph_tags {
                    let first_pos_morph = PosMorph{morph: *first_morph, pos: first.pos_tag};
                    let dest_map = index.tagging_stats.get(&first_pos_morph);
                    if dest_map.is_none() { continue; }
                    let second_pos_morph = PosMorph{morph: *second_morph, pos: second.pos_tag};
                    let stats_occ = dest_map.unwrap().get(&second_pos_morph);
                    if stats_occ.is_none() { continue; }
                    let occ = stats_occ.unwrap();
                    if *occ > best_inner_score {
                        best_inner_score = *occ;
                    };
                }
            }
            println!("'{}' and '{}' scored {}", 
                index.u8_to_str(&index.original_letters[first.letters_original_range.start as usize..first.letters_original_range.end as usize]),
                index.u8_to_str(&index.original_letters[second.letters_original_range.start as usize..second.letters_original_range.end as usize]),
                best_inner_score,
            );
            score += best_inner_score;
        }
        score
    }
}

use std::mem;

fn main() {
    println!("Size of word: {}", mem::size_of::<Word>());
    println!("Size of matching: {}", mem::size_of::<Matching>());
    println!("Size of Letters: {}", mem::size_of::<Letters>());
    let index = Index::new();
    println!("{}", index);
    // let mut index = index;
   
    // println!("Indexing over, {} letters, {} words", index.letters.len(), index.word_defs.len());
    loop {

        let mut sentence = String::new();
        println!("Please enter a sentence.");
        io::stdin().read_line(&mut sentence).expect("Failed to read input");
        // index.find_anagrams(sentence.clone());
        index.find_anagrams_reverse(sentence);
    }
}