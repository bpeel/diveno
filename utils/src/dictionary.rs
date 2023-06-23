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

pub struct Node<'a> {
    pub sibling_offset: usize,
    pub child_offset: usize,
    pub letter: char,
    pub remainder: &'a [u8],
}

impl<'a> Node<'a> {
    pub fn extract(data: &'a [u8]) -> Option<Node<'a>> {
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
