#[macro_use]
extern crate lazy_static;

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use serde_json::{Value, Map};
use std::collections::{HashMap, HashSet};

const MORPH_TYPES_LEN : usize = 3;
const MORPH_TYPES : [&'static str; MORPH_TYPES_LEN] = ["Gender", "Number", "Person"];
lazy_static! {
    static ref ALLOWED_CHARS : HashSet<u8> = HashSet::from_iter(String::from("AZERTYUIOPQSDFGHJKLMWXCVBNazertyuiopqsdfghjklmwxcvbn'àÀâÂäÄéèÈëÉîÎïôöÖ-çÇ").into_bytes());
}

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

struct Index<'a> {
    pos_tags: Vec<String>,
    morph_tags: [Vec<String>; MORPH_TYPES_LEN],
    word_defs: Vec<Word<'a>>,
}

impl<'a> Index<'a> {
    fn new(word_letters: &'a mut Vec<u8>) -> Index<'a> {
        let mut morph_tags : [Vec<String>; MORPH_TYPES_LEN] = Default::default();
        let mut word_defs = vec![];
        let mut pos_tags : Vec<String> = vec![];
        let mut ranges = vec![];
        let lines = read_lines("data/words.jsonl").expect("Words file not found");
        for line in lines {
            if let Ok(word_def) = line {
                let word_def : Value = serde_json::from_str(&word_def).unwrap();
                let word = word_def["word"].as_str().unwrap();
                let pos_tag = word_def["pos"].as_str().unwrap();
                let pos_tag_index = find_or_insert(&mut pos_tags, String::from(pos_tag));
                let morphology = Index::get_morph_tags(&mut morph_tags, word_def["morph"].as_object().unwrap());
                let len_words = word_letters.len();
                ranges.push(len_words);
                if !word.as_bytes().iter().all(|x| ALLOWED_CHARS.contains(x)) {
                    println!("{} not in character set: skipping", word);
                    continue;
                }
                word_letters.extend_from_slice(word.as_bytes());
                let new_word_def = Word {
                    letters: &[],
                    pos: pos_tag_index as u8,
                    lol_score: 0,
                    morph: morphology,
                };
                word_defs.push(new_word_def);
            }
        }
        let mut prev_start = 0;
        for (def, end_index) in word_defs.iter_mut().zip(ranges) {
            def.letters = &word_letters[prev_start..end_index];
            prev_start = end_index;
            // println!("{:?}, {}", def, end_index);
        }

        Index {
            morph_tags: morph_tags,
            word_defs: word_defs,
            pos_tags: pos_tags,
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

    
}

fn main() {
    let mut word_letters = vec![];
    let index = Index::new(&mut word_letters);
    println!("Indexing over, {} letters", word_letters.len());
    loop {

    }
}