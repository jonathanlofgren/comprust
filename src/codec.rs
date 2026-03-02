use std::io::{self, Read, Write};

/// Trait for compression algorithms.
///
/// To add a new algorithm:
/// 1. Create a module (e.g. `src/bpe/mod.rs`) with your encode/decode logic
/// 2. Define a unit struct (e.g. `pub struct BpeCodec;`)
/// 3. Implement this trait for it
/// 4. Add a match arm in `get_codec` below
pub trait Codec {
    fn encode(&self, reader: &mut dyn Read, writer: &mut dyn Write) -> io::Result<u64>;
    fn decode(&self, reader: &mut dyn Read, writer: &mut dyn Write) -> io::Result<usize>;
}

/// Look up a codec by name. Returns None for unknown algorithms.
pub fn get_codec(name: &str) -> Option<Box<dyn Codec>> {
    match name {
        "huffman" => Some(Box::new(crate::huffman::HuffmanCodec)),
        "rle" => Some(Box::new(crate::rle::RleCodec)),
        _ => None,
    }
}

pub const DEFAULT_ALGORITHM: &str = "huffman";
