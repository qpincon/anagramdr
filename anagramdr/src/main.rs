
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
// use std::fmt::Display;

fn main() {
    let lines = read_lines("data/words.jsonl").expect("Words file not found");
    // Consumes the iterator, returns an (Optional) String
    for line in lines {
        if let Ok(word_def) = line {
            println!("{}", word_def);
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