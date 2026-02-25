use std::{env, fs, io::Cursor, process};

use comprust::huffman;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: comprust <input-file>");
        process::exit(1);
    }

    let file_path = &args[1];
    let contents = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read file '{}': {}", file_path, e);
            process::exit(1);
        }
    };

    let mut byte_buffer = Cursor::new(Vec::new());

    let num_bits = match huffman::encode(&contents, &mut byte_buffer) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Failed to encode: {}", e);
            process::exit(1);
        }
    };

    println!("=> Raw: {} bytes", contents.as_bytes().len());
    println!("=> Compressed: {} bytes", byte_buffer.position());
    println!("=> Compressed: {} bits", num_bits);
}
