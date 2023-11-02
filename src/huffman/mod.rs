use bitvec::prelude::*;
use std::collections::HashMap;
use std::io::{prelude::*, Result};
use std::ops::Deref;

mod tree;
use crate::types::Serializable;

use self::tree::{HuffmanTree, Link};

// Encodes the text data using Huffman coding and writes it into the writer
// Returns the number of bits
// TODO: probably should return the number of bytes written instead
pub fn encode<W: Write>(text: &str, writer: &mut W) -> Result<u64> {
    let tree = HuffmanTree::from(text).expect("Failed to build huffman tree.");
    let dict = build_dictionary(&tree);
    let mut data = encode_with_dictionary(text, &dict);

    let num_bits = data.len();
    let pad = if num_bits % 8 > 0 {
        8 - (num_bits % 8)
    } else {
        0
    };

    // Pad with 1's to reach a full number of bytes
    data.extend(vec![true; pad]);

    // Convert the bitvec to bytes
    // TODO: This is all in memory right now which is not good
    let mut buffer = vec![];
    data.read_to_end(&mut buffer)?;

    // Should be nothing left in data
    assert!(data.is_empty());

    tree.serialize(writer)?;
    writer.write_all(&[pad.try_into().unwrap()])?; //   First write how many useless bits were padded at the end
    writer.write_all(&buffer)?; //                      Then write the buffer

    Ok(num_bits as u64)
}

pub fn decode<R: Read, W: Write>(reader: &mut R, writer: &mut W) -> Result<usize> {
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

    dbg!(&tree);

    // Then walk bit by bit and keep track of where we are
    // As soon as we hit a leaf node
    // Output that character to writer
    // Make sure the padded 1-bits at the end to reach a full byte are ignored
    let num_data_bits = bits.len() - bits_padded;
    let mut current = &tree.root;

    for b in &bits[0..num_data_bits] {
        if let Link::Leaf(_, char) = current {
            // We are at a leaf node, just output the character (as bytes)
            write_char(writer, char)?;

            // Hop back to the root of the tree
            current = &tree.root;
        }

        if let Link::Node(node, _) = current {
            match *b {
                // Go right
                true => current = &node.right,
                // Go left
                false => current = &node.left,
            }
        }
    }

    // Now after the final bit we should be at a leaf,
    // otherwise something is really wrong with the code
    match current {
        Link::Leaf(_, char) => write_char(writer, char)?,
        Link::Node(_, _) => panic!("Invalid code"),
    }

    Ok(1)
}

// TODO: this is obviously really stupid
fn write_char<W: Write>(writer: &mut W, char: &char) -> Result<()> {
    let char_as_string = char.to_string();
    let bytes = char_as_string.as_bytes();

    writer.write_all(bytes)
}

// TODO: return Vec<u8> instead
fn encode_with_dictionary(text: &str, dict: &HashMap<char, BitVec>) -> BitVec {
    let bits: BitVec = text.chars().flat_map(|c| dict[&c].clone()).collect();

    bits
}

/// Depth first search to find the codes for each leaf node
fn build_dictionary(tree: &HuffmanTree) -> HashMap<char, BitVec> {
    let mut frontier = vec![(&tree.root, bitvec![])];
    let mut codes = HashMap::new();

    while let Some((link, code)) = frontier.pop() {
        match link {
            Link::Leaf(_, ch) => {
                codes.insert(*ch, code);
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
                ('a', bitvec![1]),
                ('b', bitvec![0, 0, 0]),
                ('c', bitvec![0, 0, 1]),
                ('d', bitvec![0, 1, 0]),
                ('e', bitvec![0, 1, 1]),
            ])
        )
    }

    #[test]
    fn encodes_correctly_with_dictionary() {
        let dict = build_dictionary(&build_correct_tree());

        assert_eq!(
            encode_with_dictionary("aabcd", &dict),
            bitvec![1, 1, 0, 0, 0, 0, 0, 1, 0, 1, 0]
        );
        assert_eq!(encode_with_dictionary("", &dict), bitvec![]);
        assert_eq!(
            encode_with_dictionary("ee", &dict),
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
    //  Codes
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
    fn encodes_simple_string_to_correct_buffer() {
        let mut buffer = Vec::new();

        let result = encode("aaaabbc", &mut buffer).expect("failed");

        assert_eq!(result, 10);
        assert_eq!(
            &buffer,
            &vec![0, 0, 0, 3, 97, 98, 99, 0, 0, 0, 4, 0, 0, 0, 2, 0, 0, 0, 1, 6, 80, 255]
        );
    }

    #[test]
    fn encodes_and_then_decodes_to_same_input() {
        let text = "aaaabbc";
        let mut encode_buffer: Vec<u8> = Vec::new();
        let mut decode_buffer: Vec<u8> = Vec::new();

        // Encode the test into encode_buffer
        let bytes = encode(text, &mut encode_buffer).expect("Failed to encode");

        // Decode back into the decode_buffer
        let something =
            decode(&mut encode_buffer.as_slice(), &mut decode_buffer).expect("Failed to decode");

        assert_eq!(
            String::from_utf8(decode_buffer)
                .expect("Failed to create text data from decoded data, probably invalid utf8"),
            text
        );
    }
}
