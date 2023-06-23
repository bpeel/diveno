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

mod dictionary;

use std::process::ExitCode;
use std::{fmt, io};
use io::Write;
use std::ffi::OsStr;
use dictionary::Node;

// The word list is stored as a list of u64’s. That way each word
// takes up the same amount of space and it’s easy to index to a
// random word. The bits of the u64 are split into 5-bit numbers. Each
// number represents the number of siblings to skip while traversing
// the dictionary graph before descending to a child. When the number
// points to descending to a '\0' character in the dictionary the word
// is finished. This means we can use a dictionary whose alphabet is
// at most 32 letters and each word is at most 64/5=12 letters long.

const BITS_PER_CHOICE: u32 = 5;

enum CompressError {
    TooManyBits,
    TooManySkips,
    NotInDictionary,
    DictionaryCorrupt,
}

impl fmt::Display for CompressError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompressError::TooManyBits => write!(f, "Too many bits"),
            CompressError::TooManySkips => write!(f, "Too many skips"),
            CompressError::NotInDictionary => write!(f, "Not in dictionary"),
            CompressError::DictionaryCorrupt => write!(f, "Dictionary corrupt"),
        }
    }
}

fn compress_word(dictionary: &[u8], word: &str) -> Result<u64, CompressError> {
    let mut n_choices = 0;
    let mut choices = 0u64;

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
    let mut skip_count = 0;

    loop {
        let Some(node) = Node::extract(data)
        else {
            return Err(CompressError::DictionaryCorrupt);
        };

        if node.letter == next_letter.unwrap_or('\0') {
            if (n_choices + 1) * BITS_PER_CHOICE > u64::BITS {
                return Err(CompressError::TooManyBits);
            } else {
                choices |= skip_count << (n_choices * BITS_PER_CHOICE);
                n_choices += 1;
                skip_count = 0;
            }

            if next_letter.is_none() {
                return Ok(choices);
            }

            if node.child_offset == 0 {
                return Err(CompressError::NotInDictionary);
            }

            next_letter = word.next();

            data = match node.remainder.get(node.child_offset..) {
                Some(d) => d,
                None => return Err(CompressError::DictionaryCorrupt),
            };
        } else {
            if node.sibling_offset == 0 {
                return Err(CompressError::NotInDictionary);
            }

            skip_count += 1;

            if u64::BITS - skip_count.leading_zeros() > BITS_PER_CHOICE {
                return Err(CompressError::TooManySkips);
            }

            data = match node.remainder.get(node.sibling_offset..) {
                Some(d) => d,
                None => return Err(CompressError::DictionaryCorrupt),
            };
        }
    }
}

fn write_words(words: &[u64], output_filename: &OsStr) -> io::Result<()> {
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

    let mut words = Vec::<u64>::new();
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
