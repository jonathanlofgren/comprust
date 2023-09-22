use std::{env, fs};

use comprust::huffman;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        panic!("Please specify an input file");
    }

    let file_path = &args[1];
    let contents = fs::read_to_string(file_path).expect("Failed to read file");

    dbg!(huffman::count_chars(&contents));
}
