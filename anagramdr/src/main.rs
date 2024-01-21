use std::time::Instant;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use serde_json::{Value, Map};
use std::collections::{HashMap, HashSet};
use std::str;
use std::ops::Range;
use std::fmt;

const MORPH_TYPES_LEN : usize = 3;
const MORPH_TYPES : [&'static str; MORPH_TYPES_LEN] = ["Gender", "Number", "Person"];
const CHARS_TO_REMOVES : &'static str = " ,-"; // chars to remove for processing, but keep for storing words
const ALLOWED_CHARS : &'static str = "aAàÀâÂäÄbBcCçÇdDeEéÉèÈëËfFgGhHiIîÎïÏjJkKlLmMnNoOôÔöÖpPqQrRsStTuUvVwWxXyYz ',-"; // must contain CHARS_TO_REMOVES

const POS_TAGS_LEN : usize = 18;
// https://universaldependencies.org/u/pos/
const POS_TAGS : [&'static str; POS_TAGS_LEN] = ["ADJ","ADP","PUNC","ADV","AUX","SYM","INTJ","CCONJ","X","NOUN","DET","PROPN","NUM","VERB","PART","PRON","SCONJ", "PUNCT"];
type Letters = Vec<u8>;

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
    let file = File::open(filepath)?;
    Ok(io::BufReader::new(file).lines())
}

#[derive(Debug)]
struct Word {
    letters_sorted_range: Range<u32>,
    letters_original_range: Range<u32>,
    pos: u8,
    lol_score: u8,
    morph: [u8; MORPH_TYPES_LEN],
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
    morph_tags: [Vec<String>; MORPH_TYPES_LEN],
    /** Contain all the words of the entry vocab */
    word_defs: Vec<Word>,
    mean_word_size: f32,
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
            morph_tags: Default::default(),
            word_defs: vec![],
            sorted_letters: vec![],
            original_letters: vec![],
            mean_word_size: 0.0,
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
        CHARS_TO_REMOVES.chars().for_each(|c| {
            index.chars_to_remove.insert(*index.char_mapping.get(&c).unwrap());
        });
        let lines: io::Lines<io::BufReader<File>> = read_lines("data/words.jsonl").expect("Words file not found");
        for line in lines {
            if let Ok(word_def) = line {
                let word_def : Value = serde_json::from_str(&word_def).unwrap();
                let word = word_def["word"].as_str().unwrap();
                let pos_tag = word_def["pos"].as_str().unwrap();
                let pos_tag_index = POS_TAGS.iter().position(|&pos| pos == pos_tag).unwrap();
                let morphology = Index::get_morph_tags(&mut index.morph_tags, word_def["morph"].as_object().unwrap());
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
                    pos: pos_tag_index as u8,
                    lol_score: 0,
                    morph: morphology,
                };
                index.word_defs.push(new_word_def);
            }
        }
        index.mean_word_size = index.mean_word_size / index.word_defs.len() as f32;
        index
    }

    fn str_to_u8(&self, string: &str) -> Vec<u8> {
        string.chars().map(|c| *self.char_mapping.get(&c).unwrap()).collect()
    }

    fn u8_to_str(&self, indexes: &[u8]) -> String {
        indexes.iter().map(|&c| self.chars[c as usize]).collect()
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
        let max_cand_to_find = 100;
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
                    if new_cand.is_complete() { nb_found+= 1; }
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
                let new_candidate = Matching { letter_pool: remaining_letters, matched: vec![word]};
                candidates.push(new_candidate);
                nb_added_cand_scratch += 1;
            }
        }
        for cand in &candidates {
            if cand.letter_pool.len() == 0 {
                self.print_matching(cand);
            }
        }
        print!("Added candidates {} (scratch) {} (cloned) ", nb_added_cand_scratch, nb_added_cand_cand);
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
        println!();
    }

}

impl fmt::Display for Index {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for word in &self.word_defs {
            let original = &self.original_letters[word.letters_original_range.start as usize..word.letters_original_range.end as usize];
            let sorted = &self.sorted_letters[word.letters_sorted_range.start as usize..word.letters_sorted_range.end as usize];
            writeln!(f, "{}, sorted : {}", self.u8_to_str(original), self.u8_to_str(sorted))?
        }
        writeln!(f, "{} letters ({} sorted), {} words", self.original_letters.len(), self.sorted_letters.len(), self.word_defs.len())?;
        // for (key, val) in self.uppercase_mapping.iter() {
        //     writeln!(f, "{}: {}", self.chars[*key as usize], self.chars[*val as usize]);
        // }
        writeln!(f, "Mean letter count per word: {}", self.mean_word_size)?;
        Ok(())
    }
}


#[derive(Debug, Clone)]
struct Matching<'a> {
    letter_pool: Letters,
    matched: Vec<&'a Word>
}

impl<'a> Matching<'a> {
    fn is_complete(&self) -> bool {
        self.letter_pool.len() == 0
    }

    fn min_nb_words(&self, word_length: u32) -> f32 {
        (self.matched.len() as f32) + (self.letter_pool.len() as f32 / word_length as f32).ceil()
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
    // index.associate(&letters, ranges);
   
    // println!("Indexing over, {} letters, {} words", index.letters.len(), index.word_defs.len());
    loop {

        let mut sentence = String::new();
        println!("Please enter a sentence.");
        io::stdin().read_line(&mut sentence).expect("Failed to read input");
        // index.find_anagrams(sentence.clone());
        index.find_anagrams_reverse(sentence);
    }
}