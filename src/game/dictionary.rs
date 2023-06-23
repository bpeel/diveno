// Diveno – A word game in Esperanto
// Copyright (C) 2023  Neil Roberts
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

const BITS_PER_CHOICE: u32 = 5;

pub struct Dictionary {
    data: Box<[u8]>,
}

impl Dictionary {
    pub fn new(data: Box<[u8]>) -> Dictionary {
        Dictionary {
            data
        }
    }

    pub fn contains(&self, word: &str) -> bool {
        // Skip the root node
        let Some(Node { remainder, child_offset, .. }) =
            Node::extract(&self.data)
        else {
            return false;
        };

        if child_offset == 0 {
            return false;
        }

        let mut data = &remainder[child_offset..];
        let mut word = word.chars().flat_map(|c| c.to_lowercase());
        let mut next_letter = word.next();

        loop {
            let Some(node) = Node::extract(data)
            else {
                return false;
            };

            if node.letter == next_letter.unwrap_or('\0') {
                if next_letter.is_none() {
                    return true;
                }

                if node.child_offset == 0 {
                    return false;
                }

                next_letter = word.next();

                data = match node.remainder.get(node.child_offset..) {
                    Some(d) => d,
                    None => return false,
                };
            } else {
                if node.sibling_offset == 0 {
                    return false;
                }

                data = match node.remainder.get(node.sibling_offset..) {
                    Some(d) => d,
                    None => return false,
                };
            }
        }
    }

    pub fn extract_word(&self, mut word: u64) -> Option<String> {
        // Skip the root node
        let Some(Node { remainder, child_offset, .. }) =
            Node::extract(&self.data)
        else {
            return None;
        };

        if child_offset == 0 {
            return None;
        }

        let mut data = &remainder[child_offset..];
        let mut buf = String::new();

        loop {
            let to_skip = word & ((1 << BITS_PER_CHOICE) - 1);
            word >>= BITS_PER_CHOICE;

            for _ in 0..to_skip {
                let Some(node) = Node::extract(data)
                else {
                    return None;
                };

                if node.sibling_offset == 0 {
                    return None;
                }

                data = match node.remainder.get(node.sibling_offset..) {
                    Some(d) => d,
                    None => return None,
                };
            }

            let Some(node) = Node::extract(data)
            else {
                return None;
            };

            if node.letter == '\0' {
                return Some(buf);
            }

            buf.push(node.letter);

            if node.child_offset == 0 {
                return None;
            }

            data = match node.remainder.get(node.child_offset..) {
                Some(d) => d,
                None => return None,
            };
        }
    }
}

fn read_offset(data: &[u8]) -> Option<(&[u8], usize)> {
    let mut offset = 0;

    for (byte_num, &byte) in data.iter().enumerate() {
        if (byte_num + 1) * 7 > usize::BITS as usize {
            return None;
        }

        offset |= ((byte & 0x7f) as usize) << (byte_num * 7);

        if byte & 0x80 == 0 {
            return Some((&data[byte_num + 1..], offset));
        }
    }

    None
}

struct Node<'a> {
    sibling_offset: usize,
    child_offset: usize,
    letter: char,
    remainder: &'a [u8],
}

impl<'a> Node<'a> {
    fn extract(data: &'a [u8]) -> Option<Node<'a>> {
        let (data, sibling_offset) = read_offset(data)?;
        let (data, child_offset) = read_offset(data)?;

        let utf8_len = std::cmp::max(data.first()?.leading_ones() as usize, 1);
        let letter = std::str::from_utf8(data.get(0..utf8_len)?).ok()?;

        Some(Node {
            sibling_offset,
            child_offset,
            letter: letter.chars().next().unwrap(),
            remainder: data,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn extract() {
        let node = Node::extract(&[7, 8, b'c']).unwrap();
        assert_eq!(node.sibling_offset, 7);
        assert_eq!(node.child_offset, 8);
        assert_eq!(node.letter, 'c');
        assert_eq!(node.remainder, &[b'c']);

        let node = Node::extract(&[7, 8, 0xc4, 0x89]).unwrap();
        assert_eq!(node.sibling_offset, 7);
        assert_eq!(node.child_offset, 8);
        assert_eq!(node.letter, 'ĉ');
        assert_eq!(node.remainder, &[0xc4, 0x89]);

        let node = Node::extract(&[7, 8, 0xc4, 0x89, 0xc4, 0xa5]).unwrap();
        assert_eq!(node.sibling_offset, 7);
        assert_eq!(node.child_offset, 8);
        assert_eq!(node.letter, 'ĉ');
        assert_eq!(node.remainder, &[0xc4, 0x89, 0xc4, 0xa5]);

        assert!(Node::extract(&[7, 8, 0xc4]).is_none());

        let node = Node::extract(&[0xff, 0x7f, 0x80, 0x40, b'c']).unwrap();
        assert_eq!(node.sibling_offset, 0b11111111111111);
        assert_eq!(node.child_offset, 0b10000000000000);
        assert_eq!(node.letter, 'c');
        assert_eq!(node.remainder, &[b'c']);
    }

    #[test]
    fn contains() {
        // Dictionary that contains “a”, “b”, “c”, “apple”, “app”, “ĉapelo”
        static DICTIONARY_BYTES: [u8; 52] = [
            0x00, 0x01, 0x2a, 0x01, 0x07, b'a', 0x01, 0x29, b'b', 0x04, 0x26,
            b'c', 0x08, 0x00, 0x00, 0x00, 0x02, 0xc4, 0x89, 0x00, 0x07, b'a',
            0x00, 0x01, b'p', 0x00, 0x04, b'p', 0x00, 0x04, b'p', 0x04, 0x00,
            0x00, 0x00, 0x04, b'e', 0x00, 0x04, b'l', 0x00, 0x04, b'l', 0x00,
            0x04, b'e', 0x00, 0x01, b'o', 0x00, 0x00, 0x00,
        ];

        let dictionary = Dictionary::new(Box::new(DICTIONARY_BYTES.clone()));

        assert!(dictionary.contains("a"));
        assert!(dictionary.contains("b"));
        assert!(dictionary.contains("c"));
        assert!(dictionary.contains("apple"));
        assert!(dictionary.contains("app"));
        assert!(dictionary.contains("ĉapelo"));

        assert!(!dictionary.contains(""));
        assert!(!dictionary.contains("d"));
        assert!(!dictionary.contains("appl"));
        assert!(!dictionary.contains("apples"));
        assert!(!dictionary.contains("eĥo"));

        assert!(dictionary.contains("APPLE"));
        assert!(dictionary.contains("ĈAPelo"));
    }
}
