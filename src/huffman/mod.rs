use bitvec::prelude::*;
use std::collections::HashMap;
use std::io::{prelude::*, Result};

mod tree;
use crate::types::Serializable;

use self::tree::{HuffmanTree, Link};

// Encodes the text data using Huffman coding and writes it into the writer
// Returns the number of bits
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

    // TODO: write the tree as well
    tree.serialize(writer)?;

    writer.write_all(&[pad.try_into().unwrap()])?; //   First write how many useless bits were padded at the end
    writer.write_all(&buffer)?; //                      Then write the buffer

    Ok(num_bits as u64)
}

// TODO
pub fn decode<R: Read, W: Write>(reader: &mut R, writer: &mut W) -> Result<usize> {
    unimplemented!()
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
    //  0000_1010_00
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
        let mut writer = Vec::new();

        let result = encode("aaaabbc", &mut writer).expect("failed");

        assert_eq!(result, 10);
        assert_eq!(
            &writer,
            &vec![0, 0, 0, 3, 97, 98, 99, 0, 0, 0, 4, 0, 0, 0, 2, 0, 0, 0, 1, 6, 80, 255]
        );
    }

    #[test]
    fn encodes_and_then_decodes_to_same_input() {}
}
