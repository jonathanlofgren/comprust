use bitvec::prelude::*;
use std::collections::HashMap;
use std::io::{prelude::*, Result};

mod tree;

use self::tree::{HuffmanTree, Link, Serializable};

pub struct HuffmanCodec;

impl crate::codec::Codec for HuffmanCodec {
    fn encode(&self, data: &[u8], writer: &mut dyn Write) -> Result<u64> {
        encode(data, writer)
    }

    fn decode(&self, reader: &mut dyn Read, writer: &mut dyn Write) -> Result<usize> {
        decode(reader, writer)
    }
}

// Encodes the data using Huffman coding and writes it into the writer.
// Returns the number of bits in the compressed payload (excluding header/padding).
pub fn encode<W: Write + ?Sized>(data: &[u8], writer: &mut W) -> Result<u64> {
    let tree = HuffmanTree::build(data).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "cannot encode empty input")
    })?;
    let dict = build_dictionary(&tree);
    let mut bits = encode_with_dictionary(data, &dict);

    let num_bits = bits.len();
    let pad = if num_bits % 8 > 0 {
        8 - (num_bits % 8)
    } else {
        0
    };

    // Pad with 1's to reach a full number of bytes
    bits.extend(vec![true; pad]);

    // Convert the bitvec to bytes
    let mut buffer = vec![];
    bits.read_to_end(&mut buffer)?;

    // Should be nothing left in bits
    debug_assert!(bits.is_empty());

    tree.serialize(writer)?;
    writer.write_all(&[pad as u8])?; // First write how many useless bits were padded at the end
    writer.write_all(&buffer)?; //      Then write the compressed data

    Ok(num_bits as u64)
}

pub fn decode<R: Read + ?Sized, W: Write + ?Sized>(reader: &mut R, writer: &mut W) -> Result<usize> {
    // First read in the huffman tree
    let tree = HuffmanTree::deserialize(reader)?;

    // Read the number of padded bits at the end
    let bits_padded = {
        let mut num_padding_buffer = [0; 1];
        reader.read_exact(&mut num_padding_buffer)?;

        num_padding_buffer[0] as usize
    };

    let bits = {
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;

        BitVec::<_, Lsb0>::from_vec(buffer)
    };

    // Walk bit by bit through the tree. Navigate on each bit, and whenever
    // we land on a leaf, output that byte and hop back to the root.
    // Make sure the padded 1-bits at the end to reach a full byte are ignored.
    let num_data_bits = bits.len() - bits_padded;
    let mut current = &tree.root;
    let mut bytes_written: usize = 0;
    let mut at_root = true;

    for b in &bits[0..num_data_bits] {
        match current {
            Link::Node(node, _) => {
                current = if *b { &node.right } else { &node.left };
                at_root = false;
            }
            // Single-byte alphabet: root is a leaf, each bit represents one byte
            Link::Leaf(_, _) => {}
        }

        if let Link::Leaf(_, byte) = current {
            writer.write_all(&[*byte])?;
            bytes_written += 1;
            current = &tree.root;
            at_root = true;
        }
    }

    if !at_root {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Unexpected end of data: stopped at internal node instead of leaf",
        ));
    }

    Ok(bytes_written)
}

fn encode_with_dictionary(data: &[u8], dict: &HashMap<u8, BitVec>) -> BitVec {
    data.iter().flat_map(|b| dict[b].clone()).collect()
}

/// Depth first search to find the codes for each leaf node
fn build_dictionary(tree: &HuffmanTree) -> HashMap<u8, BitVec> {
    let mut frontier = vec![(&tree.root, bitvec![])];
    let mut codes = HashMap::new();

    while let Some((link, code)) = frontier.pop() {
        match link {
            Link::Leaf(_, byte) => {
                // If the root itself is a leaf (single unique byte), assign
                // a 1-bit code so each occurrence actually produces output.
                let code = if code.is_empty() { bitvec![0] } else { code };
                codes.insert(*byte, code);
            }
            Link::Node(node, _) => {
                let mut left_code = code.clone();
                left_code.push(false);

                let mut right_code = code.clone();
                right_code.push(true);

                frontier.push((&node.left, left_code));
                frontier.push((&node.right, right_code));
            }
        };
    }

    codes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::huffman::tree::tests::build_correct_tree;

    #[test]
    fn builds_correct_dictionary_from_tree() {
        let tree = build_correct_tree();

        assert_eq!(
            build_dictionary(&tree),
            HashMap::from([
                (b'a', bitvec![1]),
                (b'b', bitvec![0, 0, 0]),
                (b'c', bitvec![0, 0, 1]),
                (b'd', bitvec![0, 1, 0]),
                (b'e', bitvec![0, 1, 1]),
            ])
        )
    }

    #[test]
    fn encodes_correctly_with_dictionary() {
        let dict = build_dictionary(&build_correct_tree());

        assert_eq!(
            encode_with_dictionary(b"aabcd", &dict),
            bitvec![1, 1, 0, 0, 0, 0, 0, 1, 0, 1, 0]
        );
        assert_eq!(encode_with_dictionary(b"", &dict), bitvec![]);
        assert_eq!(
            encode_with_dictionary(b"ee", &dict),
            bitvec![0, 1, 1, 0, 1, 1]
        );
    }

    // aaaa bb c
    // (a 4) (b 2) (c 1)
    // (a 4) (bc 3)
    //
    //     root
    //     / \
    //    4a / \
    //      2b  1c
    //
    //  a: 0
    //  b: 10
    //  c: 11
    //
    // bits:
    //  0000_1010_11
    //
    // padded:
    //  0000_1010_1111_1111 = 2 bytes
    //  Reverse order since Lsb first per byte
    // ======================
    // buffer = [80, 255]
    // num_bits = 10
    // Then the padding amount=6 bits as 1 byte just before that
    //
    // Then we have the actual huffman tree before that
    #[test]
    fn encodes_simple_data_to_correct_buffer() {
        let mut buffer = Vec::new();

        let result = encode(b"aaaabbc", &mut buffer).expect("failed");

        assert_eq!(result, 10);
        assert_eq!(
            &buffer,
            &vec![0, 0, 0, 3, 97, 98, 99, 0, 0, 0, 4, 0, 0, 0, 2, 0, 0, 0, 1, 6, 80, 255]
        );
    }

    #[test]
    fn encodes_and_then_decodes_to_same_input() {
        let data = b"aaaabbc";
        let mut encode_buffer: Vec<u8> = Vec::new();
        let mut decode_buffer: Vec<u8> = Vec::new();

        encode(data, &mut encode_buffer).expect("Failed to encode");
        decode(&mut encode_buffer.as_slice(), &mut decode_buffer).expect("Failed to decode");

        assert_eq!(decode_buffer, data);
    }

    #[test]
    fn encodes_and_decodes_single_byte_alphabet() {
        let data = b"aaa";
        let mut encode_buffer: Vec<u8> = Vec::new();
        let mut decode_buffer: Vec<u8> = Vec::new();

        encode(data, &mut encode_buffer).expect("Failed to encode");
        decode(&mut encode_buffer.as_slice(), &mut decode_buffer).expect("Failed to decode");

        assert_eq!(decode_buffer, data);
    }

    #[test]
    fn encodes_and_decodes_binary_data() {
        let data: Vec<u8> = (0..=255).cycle().take(1000).collect();
        let mut encode_buffer: Vec<u8> = Vec::new();
        let mut decode_buffer: Vec<u8> = Vec::new();

        encode(&data, &mut encode_buffer).expect("Failed to encode");
        decode(&mut encode_buffer.as_slice(), &mut decode_buffer).expect("Failed to decode");

        assert_eq!(decode_buffer, data);
    }
}
