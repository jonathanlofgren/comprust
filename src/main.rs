use std::{env, fs, io::Cursor};

use comprust::huffman;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        panic!("Please specify an input file");
    }

    let file_path = &args[1];
    let contents = fs::read_to_string(file_path).expect("Failed to read file");

    let mut byte_buffer = Cursor::new(Vec::new());

    let num_bits = huffman::encode(&contents, &mut byte_buffer).expect("failed to encode");

    println!("=> Raw: {} bytes", contents.as_bytes().len());
    println!("=> Compressed: {} bytes", byte_buffer.position());
    println!("=> Compressed: {} bits", num_bits);
}
