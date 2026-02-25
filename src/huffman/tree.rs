use crate::types::Serializable;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    io::{Error, ErrorKind, Read, Result, Write},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuffmanTree {
    pub root: Link,
    counts: HashMap<u8, u32>,
}

impl HuffmanTree {
    pub fn build(data: &[u8]) -> Option<Self> {
        let counts = count_bytes(data);

        Self::from_counts(&counts)
    }

    pub fn from_counts(counts: &HashMap<u8, u32>) -> Option<Self> {
        // Insert the leaf nodes with the byte counts in a heap
        let mut heap = BinaryHeap::new();
        for (byte, weight) in counts {
            heap.push(Link::Leaf(*weight, *byte))
        }

        // Build the tree
        while heap.len() > 1 {
            let right = heap.pop().unwrap(); // smaller weight goes to the right subtree
            let left = heap.pop().unwrap();

            let byte = left.byte();
            heap.push(Link::Node(
                Box::new(Node {
                    weight: left.weight() + right.weight(),
                    left,
                    right,
                }),
                byte,
            ))
        }

        heap.pop().map(|link| Self {
            root: link,
            counts: counts.clone(),
        }) // This may be None in the case of empty input
    }
}

impl Serializable for HuffmanTree {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize> {
        let mut bytes: Vec<u8> = self.counts.keys().copied().collect();
        bytes.sort();
        let counts: Vec<u32> = bytes.iter().map(|b| self.counts[b]).collect();
        let num_bytes = bytes.len() as u32;

        // Write the u32 describing how many unique bytes
        writer.write_all(&num_bytes.to_be_bytes())?;
        // Write the bytes
        writer.write_all(&bytes)?;
        // Write the counts
        for count in &counts {
            writer.write_all(&count.to_be_bytes())?;
        }

        Ok(4 + bytes.len() + counts.len() * 4)
    }

    fn deserialize<R: Read>(reader: &mut R) -> Result<Self>
    where
        Self: Sized,
    {
        // Read the u32 indicating how many unique bytes
        let mut num_bytes_buffer = [0; 4];
        reader.read_exact(&mut num_bytes_buffer)?;
        let num_bytes = u32::from_be_bytes(num_bytes_buffer) as usize;

        // Read the bytes
        let mut byte_values = vec![0u8; num_bytes];
        reader.read_exact(&mut byte_values)?;

        // For each byte value, read its count
        let mut counts = HashMap::new();
        for b in &byte_values {
            let mut count_buffer = [0; 4];
            reader.read_exact(&mut count_buffer)?;
            counts.insert(*b, u32::from_be_bytes(count_buffer));
        }

        HuffmanTree::from_counts(&counts).ok_or(Error::new(
            ErrorKind::Other,
            "failed to build tree from counts",
        ))
    }
}

fn count_bytes(source: &[u8]) -> HashMap<u8, u32> {
    source.iter().fold(HashMap::new(), |mut map, &b| {
        *map.entry(b).or_insert(0) += 1;
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
    Leaf(u32, u8),
    Node(Box<Node>, u8),
}

impl Link {
    pub fn weight(&self) -> u32 {
        match self {
            Link::Leaf(weight, _) => *weight,
            Link::Node(node, _) => node.weight,
        }
    }

    // The "representative" byte of a Leaf/Node. Needed to break ties in weight.
    pub fn byte(&self) -> u8 {
        match self {
            Link::Leaf(_, byte) => *byte,
            Link::Node(_, byte) => *byte,
        }
    }
}

impl Ord for Link {
    fn cmp(&self, other: &Self) -> Ordering {
        (other.weight(), other.byte()).cmp(&(self.weight(), self.byte()))
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
    fn test_count_bytes() {
        assert_eq!(
            count_bytes(b"mamma"),
            HashMap::from([(b'm', 3), (b'a', 2)])
        );
        assert_eq!(count_bytes(b""), HashMap::new());
        assert_eq!(
            count_bytes(b"abcd"),
            HashMap::from([(b'a', 1), (b'b', 1), (b'c', 1), (b'd', 1)])
        );
    }

    #[test]
    fn build_huffman_tree_for_simple_case() {
        let expected = build_correct_tree();
        let text = b"aaaaaaaaaaaaaaabbbbbbbccccccdddddeeee";

        assert_eq!(HuffmanTree::build(text), Option::Some(expected));
    }

    #[test]
    fn build_huffman_tree_for_edge_cases() {
        assert_eq!(
            HuffmanTree::build(b"a"),
            Option::Some(HuffmanTree {
                root: Link::Leaf(1, b'a'),
                counts: HashMap::from([(b'a', 1)])
            })
        );
        assert_eq!(HuffmanTree::build(b""), None);
    }

    #[test]
    fn build_from_counts_is_deterministic() {
        // Recreate the counts every time and make sure it always results in the same tree
        let get_counts = || (b'a'..=b'z').map(|b| (b, 100)).collect();
        let tree = HuffmanTree::from_counts(&get_counts()).unwrap();

        for _ in 0..20 {
            assert_eq!(tree, HuffmanTree::from_counts(&get_counts()).unwrap());
        }
    }

    #[test]
    fn can_sort_links() {
        let d = Link::Leaf(3, b'd');
        let e = Link::Leaf(5, b'e');
        let de = Link::Node(
            Box::new(Node {
                weight: 11,
                left: Link::Leaf(3, b'd'),
                right: Link::Leaf(3, b'd'),
            }),
            b'a',
        );

        let mut links = vec![de.clone(), e.clone(), d.clone()];
        links.sort();

        assert_eq!(links, vec![de, e, d]);
    }

    #[test]
    fn can_serialize_and_deserialize_to_eq_object() {
        let original = build_correct_tree();
        let mut buffer = Vec::<u8>::new();

        original.serialize(&mut buffer).unwrap();

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
        let a = Link::Leaf(15, b'a');
        let b = Link::Leaf(7, b'b');
        let c = Link::Leaf(6, b'c');
        let d = Link::Leaf(5, b'd');
        let e = Link::Leaf(4, b'e');
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
            left: Link::Node(Box::new(bc), b'b'),
            right: Link::Node(Box::new(de), b'd'),
        };

        HuffmanTree {
            root: Link::Node(
                Box::new(Node {
                    weight: 37,
                    left: Link::Node(Box::new(bcde), b'b'),
                    right: a,
                }),
                b'b',
            ),
            counts: HashMap::from([(b'e', 4), (b'd', 5), (b'c', 6), (b'b', 7), (b'a', 15)]),
        }
    }
}
