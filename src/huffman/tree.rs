use crate::types::Serializable;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    io::{Error, ErrorKind, Read, Result, Write},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuffmanTree {
    pub root: Link,
    counts: HashMap<char, u32>,
}

impl HuffmanTree {
    pub fn from(text: &str) -> Option<Self> {
        let counts = count_chars(text);

        Self::from_counts(&counts)
    }

    pub fn from_counts(counts: &HashMap<char, u32>) -> Option<Self> {
        // Insert the leaf nodes with the character counts in a heap
        let mut heap = BinaryHeap::new();
        for (ch, weight) in counts {
            heap.push(Link::Leaf(*weight, *ch))
        }

        // Build the tree
        while heap.len() > 1 {
            let right = heap.pop().unwrap(); // smaller weight goes to the right subtree
            let left = heap.pop().unwrap();

            let char = left.char();
            heap.push(Link::Node(
                Box::new(Node {
                    weight: left.weight() + right.weight(),
                    left,
                    right,
                }),
                char,
            ))
        }

        heap.pop().map(|link| Self {
            root: link,
            counts: counts.clone(),
        }) // This may be None in the case of an empty string input
    }
}

impl Serializable for HuffmanTree {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize> {
        let chars: String = {
            let mut chars: Vec<&char> = self.counts.keys().collect();
            chars.sort();
            chars.into_iter().collect()
        };
        let counts: Vec<_> = chars.chars().map(|c| self.counts[&c]).collect();
        let chars_num_bytes = chars.as_bytes().len() as u32;

        // Write the u32 describing how many bytes of characters
        writer.write_all(&chars_num_bytes.to_be_bytes())?;
        // Write the characters
        writer.write_all(chars.as_bytes())?;
        // Write the counts
        for count in &counts {
            writer.write_all(&count.to_be_bytes())?;
        }

        Ok(4 + (chars_num_bytes as usize) + counts.len() * 4)
    }

    fn deserialize<R: Read>(reader: &mut R) -> Result<Self>
    where
        Self: Sized,
    {
        // Read the u32 indicating how many bytes of characters
        let mut num_bytes_buffer = [0; 4];
        reader.read_exact(&mut num_bytes_buffer)?;
        let num_bytes = u32::from_be_bytes(num_bytes_buffer) as usize;

        // Read the characters as a String
        let mut char_buffer = vec![0; num_bytes];
        reader.read_exact(&mut char_buffer)?;
        let chars = String::from_utf8(char_buffer).unwrap();

        // Over each characters in the string, read the count and collect to the HashMap
        let counts: HashMap<char, u32> = chars
            .chars()
            .map(|c| {
                let mut count_buffer = [0; 4];
                reader.read_exact(&mut count_buffer).unwrap();

                (c, u32::from_be_bytes(count_buffer))
            })
            .collect();

        HuffmanTree::from_counts(&counts).ok_or(Error::new(
            ErrorKind::Other,
            "failed to build tree from counts",
        ))
    }
}

fn count_chars(source: &str) -> HashMap<char, u32> {
    source.chars().fold(HashMap::new(), |mut map, c| {
        *map.entry(c).or_insert(0) += 1;
        map
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    weight: u32,
    pub left: Link,
    pub right: Link,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    Leaf(u32, char),
    Node(Box<Node>, char),
}

impl Link {
    pub fn weight(&self) -> u32 {
        match self {
            Link::Leaf(weight, _) => *weight,
            Link::Node(node, _) => node.weight,
        }
    }

    // The "representative" character of a Leaf/Node. Needed to break ties in weight
    pub fn char(&self) -> char {
        match self {
            Link::Leaf(_, char) => *char,
            Link::Node(_, char) => *char,
        }
    }
}

impl Ord for Link {
    fn cmp(&self, other: &Self) -> Ordering {
        (other.weight(), other.char()).cmp(&(self.weight(), self.char()))
    }
}

impl PartialOrd for Link {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_count_chars() {
        assert_eq!(count_chars("mamma"), HashMap::from([('m', 3), ('a', 2)]));
        assert_eq!(count_chars(""), HashMap::new());
        assert_eq!(
            count_chars("abcd"),
            HashMap::from([('a', 1), ('b', 1), ('c', 1), ('d', 1)])
        );
    }

    #[test]
    fn build_huffman_tree_for_simple_case() {
        let expected = build_correct_tree();
        let text = "aaaaaaaaaaaaaaabbbbbbbccccccdddddeeee";

        assert_eq!(HuffmanTree::from(text), Option::Some(expected));
    }

    #[test]
    fn build_huffman_tree_for_edge_cases() {
        assert_eq!(
            HuffmanTree::from("a"),
            Option::Some(HuffmanTree {
                root: Link::Leaf(1, 'a'),
                counts: HashMap::from([('a', 1)])
            })
        );
        assert_eq!(HuffmanTree::from(""), None);
    }

    #[test]
    fn build_from_counts_is_determinsitic() {
        // Recreate the counts every time and make sure it always results in the same tree
        let get_counts = || (b'a'..=b'z').map(|b| (b as char, 100)).collect();
        let tree = HuffmanTree::from_counts(&get_counts()).unwrap();

        for _ in 0..20 {
            assert_eq!(tree, HuffmanTree::from_counts(&get_counts()).unwrap());
        }
    }

    #[test]
    fn can_sort_links() {
        let d = Link::Leaf(3, 'd');
        let e = Link::Leaf(5, 'e');
        let de = Link::Node(
            Box::new(Node {
                weight: 11,
                left: Link::Leaf(3, 'd'),
                right: Link::Leaf(3, 'd'),
            }),
            'a',
        );

        let mut links = vec![de.clone(), e.clone(), d.clone()];
        links.sort();

        assert_eq!(links, vec![de, e, d]);
    }

    #[test]
    fn can_serialize_and_deserialize_to_eq_object() {
        let original = build_correct_tree();
        let mut buffer = Vec::<u8>::new();

        assert_eq!(original.serialize(&mut buffer).unwrap(), 29);

        let read = HuffmanTree::deserialize(&mut buffer.as_slice()).unwrap();
        assert_eq!(original, read);
    }

    /// Correct codes for this tree should be:
    ///     a: 1
    ///     b: 000
    ///     c: 001
    ///     d: 010
    ///     e: 011
    pub fn build_correct_tree() -> HuffmanTree {
        let a = Link::Leaf(15, 'a');
        let b = Link::Leaf(7, 'b');
        let c = Link::Leaf(6, 'c');
        let d = Link::Leaf(5, 'd');
        let e = Link::Leaf(4, 'e');
        let de = Node {
            weight: 9,
            left: d,
            right: e,
        };
        let bc = Node {
            weight: 13,
            left: b,
            right: c,
        };
        let bcde = Node {
            weight: 22,
            left: Link::Node(Box::new(bc), 'b'),
            right: Link::Node(Box::new(de), 'd'),
        };

        HuffmanTree {
            root: Link::Node(
                Box::new(Node {
                    weight: 37,
                    left: Link::Node(Box::new(bcde), 'b'),
                    right: a,
                }),
                'b',
            ),
            counts: HashMap::from([('e', 4), ('d', 5), ('c', 6), ('b', 7), ('a', 15)]),
        }
    }
}
