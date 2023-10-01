use std::io::{prelude::*, Result};

pub enum Codes {
    Huffman,
}

pub trait Coder {
    fn encode<R: Read, W: Write>(reader: &mut R, writer: &mut W) -> Result<usize>;
    fn decode<R: Read, W: Write>(reader: &mut R, writer: &mut W) -> Result<usize>;
}

pub trait Serializable {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize>;
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self>
    where
        Self: Sized;
}
