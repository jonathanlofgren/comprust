use std::io::{self, prelude::*};

pub enum CompressionMethod {
    HuffmanCoding,
}

pub trait Compressor {
    fn encode<R: Read, W: Write>(reader: &mut R, writer: &mut W) -> io::Result<usize>;
    fn decode<R: Read, W: Write>(reader: &mut R, writer: &mut W) -> io::Result<usize>;
}
