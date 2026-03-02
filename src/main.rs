use std::fs::File;
use std::io::BufReader;
use std::time::Instant;
use std::{env, fs, process};

use comprust::codec::{self, Codec, DEFAULT_ALGORITHM};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    // Parse: comprust <command> [-a algorithm] <input> <output>
    let command = args[1].as_str();
    let (algorithm, rest) = parse_algorithm_flag(&args[2..]);

    match command {
        "encode" => {
            if rest.len() < 2 {
                eprintln!("Usage: comprust encode [-a algorithm] <input-file> <output-file>");
                process::exit(1);
            }
            let codec = resolve_codec(&algorithm);
            cmd_encode(codec.as_ref(), &rest[0], &rest[1]);
        }
        "decode" => {
            if rest.len() < 2 {
                eprintln!("Usage: comprust decode [-a algorithm] <input-file> <output-file>");
                process::exit(1);
            }
            let codec = resolve_codec(&algorithm);
            cmd_decode(codec.as_ref(), &rest[0], &rest[1]);
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
            process::exit(1);
        }
    }
}

/// Extract `-a <name>` or `--algorithm <name>` from args, return the rest.
fn parse_algorithm_flag(args: &[String]) -> (String, Vec<String>) {
    let mut algorithm = DEFAULT_ALGORITHM.to_string();
    let mut rest = Vec::new();
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        if arg == "-a" || arg == "--algorithm" {
            match iter.next() {
                Some(name) => algorithm = name.clone(),
                None => {
                    eprintln!("Missing value for {}", arg);
                    process::exit(1);
                }
            }
        } else {
            rest.push(arg.clone());
        }
    }

    (algorithm, rest)
}

fn resolve_codec(name: &str) -> Box<dyn Codec> {
    match codec::get_codec(name) {
        Some(c) => c,
        None => {
            eprintln!("Unknown algorithm: '{}'. Available: huffman, rle", name);
            process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("Usage: comprust <command> [-a algorithm] <input-file> <output-file>");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  encode    Compress a file");
    eprintln!("  decode    Decompress a file");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -a, --algorithm <name>    Compression algorithm (default: huffman)");
    eprintln!("                            Available: huffman, rle");
}

fn cmd_encode(codec: &dyn Codec, input_path: &str, output_path: &str) {
    let file = match File::open(input_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to read '{}': {}", input_path, e);
            process::exit(1);
        }
    };
    let input_size = match file.metadata() {
        Ok(m) => m.len(),
        Err(e) => {
            eprintln!("Failed to read metadata for '{}': {}", input_path, e);
            process::exit(1);
        }
    };
    let mut reader = BufReader::new(file);

    let start = Instant::now();
    let mut output = Vec::new();
    let num_bits = match codec.encode(&mut reader, &mut output) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Failed to encode: {}", e);
            process::exit(1);
        }
    };
    let elapsed = start.elapsed();

    if let Err(e) = fs::write(output_path, &output) {
        eprintln!("Failed to write '{}': {}", output_path, e);
        process::exit(1);
    }

    let ratio = output.len() as f64 / input_size as f64;
    println!("=> Raw: {} bytes", input_size);
    println!("=> Compressed: {} bytes", output.len());
    println!("=> Compressed: {} bits", num_bits);
    println!("=> Ratio: {:.2}%", ratio * 100.0);
    println!("=> Time: {:.3?}", elapsed);
    println!("=> Written to: {}", output_path);
}

fn cmd_decode(codec: &dyn Codec, input_path: &str, output_path: &str) {
    let file = match File::open(input_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to read '{}': {}", input_path, e);
            process::exit(1);
        }
    };
    let input_size = match file.metadata() {
        Ok(m) => m.len(),
        Err(e) => {
            eprintln!("Failed to read metadata for '{}': {}", input_path, e);
            process::exit(1);
        }
    };
    let mut reader = BufReader::new(file);

    let start = Instant::now();
    let mut output = Vec::new();
    let bytes_written = match codec.decode(&mut reader, &mut output) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Failed to decode: {}", e);
            process::exit(1);
        }
    };
    let elapsed = start.elapsed();

    if let Err(e) = fs::write(output_path, &output) {
        eprintln!("Failed to write '{}': {}", output_path, e);
        process::exit(1);
    }

    let ratio = input_size as f64 / bytes_written as f64;
    println!("=> Compressed: {} bytes", input_size);
    println!("=> Restored: {} bytes", bytes_written);
    println!("=> Ratio: {:.2}%", ratio * 100.0);
    println!("=> Time: {:.3?}", elapsed);
    println!("=> Written to: {}", output_path);
}
