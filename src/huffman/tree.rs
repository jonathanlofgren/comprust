use std::collections::{BinaryHeap, HashMap};

pub fn build_huffman_tree(text: &str) -> Option<HuffmanTree> {
    let counts = count_chars(text);

    // Insert the leaf nodes with the character counts in the heap
    let mut heap = BinaryHeap::new();
    for (ch, weight) in counts {
        heap.push(Link::Leaf(weight, ch))
    }

    // Build the tree
    while heap.len() > 1 {
        let right = heap.pop().unwrap();
        let left = heap.pop().unwrap();

        heap.push(Link::Node(Box::new(Node {
            weight: left.weight() + right.weight(),
            left: left,
            right: right,
        })))
    }

    heap.pop().map(|link| HuffmanTree { root: link }) // This may be None in the case of an empty string input
}

fn count_chars(source: &str) -> HashMap<char, i32> {
    source.chars().fold(HashMap::new(), |mut map, c| {
        *map.entry(c).or_insert(0) += 1;
        map
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuffmanTree {
    pub root: Link,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    weight: i32,
    pub left: Link,
    pub right: Link,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    Leaf(i32, char),
    Node(Box<Node>),
}

impl Link {
    pub fn weight(&self) -> i32 {
        match self {
            Link::Leaf(weight, _) => *weight,
            Link::Node(node) => node.weight,
        }
    }
}

impl Ord for Link {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.weight().cmp(&self.weight())
    }
}

impl PartialOrd for Link {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
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
    fn test_build_huffman_tree_for_simple_case() {
        let expected = build_correct_tree();
        let text = "aaaaaaaaaaaaaaabbbbbbbccccccdddddeeee";

        assert_eq!(build_huffman_tree(text), Option::Some(expected));
    }

    #[test]
    fn test_build_huffman_tree_for_edge_cases() {
        assert_eq!(
            build_huffman_tree("a"),
            Option::Some(HuffmanTree {
                root: Link::Leaf(1, 'a')
            })
        );
        assert_eq!(build_huffman_tree(""), None);
    }

    #[test]
    fn test_can_sort_links() {
        let d = Link::Leaf(3, 'd');
        let e = Link::Leaf(5, 'e');
        let de = Link::Node(Box::new(Node {
            weight: 11,
            left: Link::Leaf(3, 'd'),
            right: Link::Leaf(3, 'd'),
        }));

        let mut links = vec![de.clone(), e.clone(), d.clone()];
        links.sort();

        assert_eq!(links, vec![de, e, d]);
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
            left: Link::Node(Box::new(bc)),
            right: Link::Node(Box::new(de)),
        };

        HuffmanTree {
            root: Link::Node(Box::new(Node {
                weight: 37,
                left: Link::Node(Box::new(bcde)),
                right: a,
            })),
        }
    }
}
