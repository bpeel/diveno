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
use std::mem::size_of;
use dictionary::Node;

const BITS_PER_CHOICE: u32 = 5;

fn append_word(buf: &mut String, dictionary: &[u8], mut word: u64) -> bool {
    // Skip the root node
    let Some(Node { remainder, child_offset, .. }) =
        Node::extract(dictionary)
    else {
        return false;
    };

    if child_offset == 0 {
        return false;
    }

    let mut data = &remainder[child_offset..];

    loop {
        let to_skip = word & ((1 << BITS_PER_CHOICE) - 1);
        word >>= BITS_PER_CHOICE;

        for _ in 0..to_skip {
            let Some(node) = Node::extract(data)
            else {
                return false;
            };

            if node.sibling_offset == 0 {
                return false;
            }

            data = match node.remainder.get(node.sibling_offset..) {
                Some(d) => d,
                None => return false,
            };
        }

        let Some(node) = Node::extract(data)
        else {
            return false;
        };

        if node.letter == '\0' {
            return true;
        }

        buf.push(node.letter);

        if node.child_offset == 0 {
            return false;
        }

        data = match node.remainder.get(node.child_offset..) {
            Some(d) => d,
            None => return false,
        };
    }
}

fn main() -> ExitCode {
    let mut args = std::env::args_os();

    let (Some(dictionary_filename), Some(word_list_filename)) =
        (args.nth(1), args.next())
    else {
        eprintln!("usage: dump_word_list <dictionary> <word_list>");
        return ExitCode::FAILURE;
    };

    let dictionary = match std::fs::read(&dictionary_filename) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", dictionary_filename.to_string_lossy(), e);
            return ExitCode::FAILURE;
        },
    };

    let word_list = match std::fs::read(&word_list_filename) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", word_list_filename.to_string_lossy(), e);
            return ExitCode::FAILURE;
        },
    };

    let mut buf = String::new();
    let mut ret = ExitCode::SUCCESS;

    for index in (0..word_list.len()).step_by(size_of::<u64>()) {
        let mut bytes = [0u8; size_of::<u64>()];
        bytes.copy_from_slice(&word_list[index..index + size_of::<u64>()]);
        let word = u64::from_le_bytes(bytes);

        buf.clear();

        if append_word(&mut buf, &dictionary, word) {
            println!("{}", buf);
        } else {
            eprintln!("couldn’t decode word at {}", index);
            ret = ExitCode::FAILURE;
        }
    }

    ret
}
