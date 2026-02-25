use std::io::{prelude::*, Result};

pub trait Serializable {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize>;
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self>
    where
        Self: Sized;
}
