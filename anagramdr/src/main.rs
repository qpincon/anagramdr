// #[macro_use]
// extern crate lazy_static;

use std::time::Instant;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use serde_json::{Value, Map};
use std::collections::HashMap;
use std::str;
use std::fmt;

const MORPH_TYPES_LEN : usize = 3;
const MORPH_TYPES : [&'static str; MORPH_TYPES_LEN] = ["Gender", "Number", "Person"];
const ALLOWED_CHARS : &'static str = "AZERTYUIOPQSDFGHJKLMWXCVBNazertyuiopqsdfghjklmwxcvbn'àÀâÂäÄéèÈëÉîÎïôöÖ-çÇ";
type Letters = Vec<u8>;


fn find_or_insert<T> (list: &mut Vec<T>, elem: T) -> usize 
        where T: Clone + PartialOrd
{
    match list.iter().position(|e| e == &elem) {
        Some(pos) => (pos + 1),
        None => {
            list.push(elem.clone());
            list.len()
        }
    }
}


fn remove_elems<T>(original: &mut Vec<T>, matched_word: &Vec<T>) where T: PartialOrd {
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
    let file = File::open(filepath)?;
    Ok(io::BufReader::new(file).lines())
}

#[derive(Debug)]
struct Word<'a> {
    letters: &'a [u8],
    pos: u8,
    lol_score: u8,
    morph: [u8; MORPH_TYPES_LEN],
}

// impl fmt::Display for Word<'_> {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", str::from_utf8(&self.letters).unwrap())
//     }
// }


struct Index<'a> {
    // letters: Letters,
    char_mapping: HashMap<char, u8>,
    chars: Vec<char>,
    pos_tags: Vec<String>,
    morph_tags: [Vec<String>; MORPH_TYPES_LEN],
    word_defs: Vec<Word<'a>>,
}

impl<'a> Index<'a> {
    fn new() -> (Index<'a>, Vec<usize>, Letters) {
        let mut index = Index {
            char_mapping: HashMap::new(),
            chars: vec![],
            morph_tags: Default::default(),
            word_defs: vec![],
            pos_tags: vec![],
        };
        ALLOWED_CHARS.chars().for_each(|c| {
            index.char_mapping.insert(c, index.chars.len() as u8);
            index.chars.push(c);
        });
        let mut letters = vec![];
        let lines = read_lines("data/words.jsonl").expect("Words file not found");
        let mut ranges = vec![];
        for line in lines {
            if let Ok(word_def) = line {
                let word_def : Value = serde_json::from_str(&word_def).unwrap();
                let word = word_def["word"].as_str().unwrap();
                let pos_tag = word_def["pos"].as_str().unwrap();
                let pos_tag_index = find_or_insert(&mut index.pos_tags, String::from(pos_tag));
                let morphology = Index::get_morph_tags(&mut index.morph_tags, word_def["morph"].as_object().unwrap());
                let len_words = letters.len();
                ranges.push(len_words);
                if !word.chars().all(|x| index.char_mapping.contains_key(&x)) {
                    println!("{} not in character set: skipping", word);
                    continue;
                }
                letters.extend_from_slice(&index.str_to_u8(word));
                let new_word_def = Word {
                    letters: &[],
                    pos: pos_tag_index as u8,
                    lol_score: 0,
                    morph: morphology,
                };
                index.word_defs.push(new_word_def);
            }
        }
        (index, ranges, letters)
    }

    fn str_to_u8(&self, string: &str) -> Vec<u8> {
        string.chars().map(|c| *self.char_mapping.get(&c).unwrap()).collect()
    }

    fn u8_to_str(&self, indexes: &[u8]) -> String {
        indexes.iter().map(|&c| self.chars[c as usize]).collect()
    }
    
    // associate words to their slices
    fn associate(&mut self, letters: &'a Letters, ranges: Vec<usize>) {
        let mut prev_start = 0;
        for (def, end_index) in self.word_defs.iter_mut().zip(ranges) {
            def.letters = &letters[prev_start..end_index];
            prev_start = end_index;
        }
    }

    fn get_morph_tags(morph_tags: &mut [Vec<String>; MORPH_TYPES_LEN], morph: &Map<String, Value>) -> [u8; 3] {
        let mut tags : [u8; MORPH_TYPES_LEN] = [0; MORPH_TYPES_LEN];
        MORPH_TYPES.iter().enumerate().for_each(|(index, morph_type)| {
            if let Some(morph_value) = morph.get(*morph_type) {
                let morph_tag_str = morph_value.as_str().unwrap();
                let tags_for_morph = &mut morph_tags[index];
                tags[index] = find_or_insert(tags_for_morph, String::from(morph_tag_str)) as u8;
            }
        });
        tags
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

    fn new_vec_removed_letters(original: &Letters, matched_word: &Letters) -> Letters {
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


    fn find_anagrams(&self, mut input: String) {
        let before = Instant::now();

        input.retain(|c| !c.is_whitespace());
        let mut sorted_letters : Vec<u8> = input.drain(..).map(|c| *self.char_mapping.get(&c).unwrap()).collect();
        sorted_letters.sort();
        let mut candidates : Vec<Matching> = vec![];
        let mut sorted_letters_cand : Vec<u8> = Vec::with_capacity(100);
        for word in &self.word_defs {
            sorted_letters_cand.extend_from_slice(&word.letters);
            sorted_letters_cand.sort();
            let candidates_len = candidates.len();
            for candidate_index in 0..candidates_len {
                let candidate = &mut candidates[candidate_index];
                if candidate.letter_pool.len() == 0 { continue; }
                let check = Index::check_contains_all_letters(&candidate.letter_pool, &sorted_letters_cand);
                if check {
                    // remove letters from original pool
                    remove_elems(&mut candidate.letter_pool,  &sorted_letters_cand);
                    candidate.matched.push(&word);
                }
                
            }
            // add new candidate if match 
            if Index::check_contains_all_letters(&sorted_letters, &sorted_letters_cand) {
                // remove letters from original pool
                let remaining_letters = Index::new_vec_removed_letters(&sorted_letters, &sorted_letters_cand);
                let new_candidate = Matching { letter_pool: remaining_letters, matched: vec![word]};
                candidates.push(new_candidate);
            }
            sorted_letters_cand.clear();
        }
        println!("{} candidate group", candidates.len());
        for cand in &candidates {
            if cand.letter_pool.len() == 0 {
                self.print_matching(cand);
            }
        }
        println!("Elapsed time: {:.2?}", before.elapsed());
    }

    fn print_matching(&self, matching: &Matching) {
        if matching.letter_pool.len() > 0 {
            println!("Remaining letters: {}", self.u8_to_str(&matching.letter_pool));
        }
        println!("matched {} words:", matching.matched.len());
        for word in &matching.matched {
            println!("{} ", self.u8_to_str(word.letters));
        }
    }

    
}

#[derive(Debug)]
struct Matching<'a> {
    letter_pool: Letters,
    matched: Vec<&'a Word<'a>>
}

fn main() {
    let (index, ranges, letters) = Index::new();
    let mut index = index;
    index.associate(&letters, ranges);
    println!("Indexing over, {} letters, {} words", letters.len(), index.word_defs.len());
    // println!("Indexing over, {} letters, {} words", index.letters.len(), index.word_defs.len());
    loop {

        let mut sentence = String::new();
        println!("Please enter a sentence.");
        io::stdin().read_line(&mut sentence).expect("Failed to read input");
        index.find_anagrams(sentence);
    }
}