use std::{env, fs, process};

use comprust::huffman;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "encode" => {
            if args.len() < 4 {
                eprintln!("Usage: comprust encode <input-file> <output-file>");
                process::exit(1);
            }
            cmd_encode(&args[2], &args[3]);
        }
        "decode" => {
            if args.len() < 4 {
                eprintln!("Usage: comprust decode <input-file> <output-file>");
                process::exit(1);
            }
            cmd_decode(&args[2], &args[3]);
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
            process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("Usage: comprust <command> <input-file> <output-file>");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  encode    Compress a file using Huffman coding");
    eprintln!("  decode    Decompress a previously compressed file");
}

fn cmd_encode(input_path: &str, output_path: &str) {
    let data = match fs::read(input_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to read '{}': {}", input_path, e);
            process::exit(1);
        }
    };

    let mut output = Vec::new();
    let num_bits = match huffman::encode(&data, &mut output) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Failed to encode: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = fs::write(output_path, &output) {
        eprintln!("Failed to write '{}': {}", output_path, e);
        process::exit(1);
    }

    println!("=> Raw: {} bytes", data.len());
    println!("=> Compressed: {} bytes", output.len());
    println!("=> Compressed: {} bits", num_bits);
    println!("=> Written to: {}", output_path);
}

fn cmd_decode(input_path: &str, output_path: &str) {
    let data = match fs::read(input_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to read '{}': {}", input_path, e);
            process::exit(1);
        }
    };

    let mut output = Vec::new();
    let bytes_written = match huffman::decode(&mut data.as_slice(), &mut output) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Failed to decode: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = fs::write(output_path, &output) {
        eprintln!("Failed to write '{}': {}", output_path, e);
        process::exit(1);
    }

    println!("=> Compressed: {} bytes", data.len());
    println!("=> Restored: {} bytes", bytes_written);
    println!("=> Written to: {}", output_path);
}
