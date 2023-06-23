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

use std::process::ExitCode;
use std::{fmt, io};
use io::Write;
use std::ffi::OsStr;

enum CompressError {
    TooManyBits,
    NotInDictionary,
    DictionaryCorrupt,
}

impl fmt::Display for CompressError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompressError::TooManyBits => write!(f, "Too many bits"),
            CompressError::NotInDictionary => write!(f, "Not in dictionary"),
            CompressError::DictionaryCorrupt => write!(f, "Dictionary corrupt"),
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

fn compress_word(dictionary: &[u8], word: &str) -> Result<u32, CompressError> {
    let mut n_choices = 0;
    let mut choices = 0u32;

    // Skip the root node
    let Some(Node { remainder, child_offset, .. }) =
        Node::extract(dictionary)
    else {
        return Err(CompressError::DictionaryCorrupt);
    };

    if child_offset == 0 {
        return Err(CompressError::DictionaryCorrupt);
    }

    let mut data = &remainder[child_offset..];
    let mut word = word.chars().flat_map(|c| c.to_lowercase());
    let mut next_letter = word.next();

    loop {
        let Some(node) = Node::extract(data)
        else {
            return Err(CompressError::DictionaryCorrupt);
        };

        if node.letter == next_letter.unwrap_or('\0') {
            if next_letter.is_none() {
                if n_choices >= u32::BITS {
                    return Err(CompressError::TooManyBits);
                } else {
                    return Ok(choices | (1 << n_choices));
                }
            }

            if node.child_offset == 0 {
                return Err(CompressError::NotInDictionary);
            }

            next_letter = word.next();

            if n_choices >= u32::BITS {
                return Err(CompressError::TooManyBits);
            } else {
                choices |= 1 << n_choices;
                n_choices += 1;
            }

            data = match node.remainder.get(node.child_offset..) {
                Some(d) => d,
                None => return Err(CompressError::DictionaryCorrupt),
            };
        } else {
            if node.sibling_offset == 0 {
                return Err(CompressError::NotInDictionary);
            }

            if n_choices >= u32::BITS {
                return Err(CompressError::TooManyBits);
            } else {
                n_choices += 1;
            }

            data = match node.remainder.get(node.sibling_offset..) {
                Some(d) => d,
                None => return Err(CompressError::DictionaryCorrupt),
            };
        }
    }
}

fn write_words(words: &[u32], output_filename: &OsStr) -> io::Result<()> {
    let output = std::fs::File::create(output_filename)?;
    let mut output = io::BufWriter::new(output);

    for &word in words.iter() {
        output.write(&word.to_le_bytes())?;
    }

    output.flush()
}

fn main() -> ExitCode {
    let mut args = std::env::args_os();

    let (Some(dictionary_filename), Some(output_filename)) =
        (args.nth(1), args.next())
    else {
        eprintln!("usage: make_word_list <dictionary> <output>");
        return ExitCode::FAILURE;
    };

    let dictionary = match std::fs::read(&dictionary_filename) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", dictionary_filename.to_string_lossy(), e);
            return ExitCode::FAILURE;
        },
    };

    let mut words = Vec::<u32>::new();
    let mut ret = ExitCode::SUCCESS;

    for line in io::stdin().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("{}", e);
                return ExitCode::FAILURE;
            },
        };

        match compress_word(&dictionary, &line) {
            Ok(word) => words.push(word),
            Err(e) => {
                eprintln!("{}: {}", line, e);
                ret = ExitCode::FAILURE;
            },
        }
    }

    if let Err(e) = write_words(&words, &output_filename) {
        eprintln!("{}: {}", output_filename.to_string_lossy(), e);
        ret = ExitCode::FAILURE;
    }

    ret
}
