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

use super::random;

pub const GRID_WIDTH: usize = 5;
pub const GRID_HEIGHT: usize = 5;
pub const N_SPACES: usize = GRID_WIDTH * GRID_HEIGHT;
pub const N_INITIAL_SPACES_COVERED: usize = 8;
// When generating the inital set of covered spaces, we won’t cover
// another space in a row, column or diagonal that already has this
// many spaces covered. That way we won’t generate a line that is
// already a bingo or that is too easy to complete.
const MAX_INITIAL_COVERED_SPACES_PER_LINE: usize = 2;

pub struct BingoGrid {
    spaces_covered: u32,
    spaces: [u8; N_SPACES],
}

impl BingoGrid {
    pub fn new() -> BingoGrid {
        assert!(N_SPACES <= u32::BITS as usize);

        let mut spaces = [0; N_SPACES];

        for (space_num, space) in spaces.iter_mut().enumerate() {
            *space = space_num as u8;
        }

        BingoGrid {
            spaces_covered: 0,
            spaces,
        }
    }

    pub fn reset(&mut self) {
        random::shuffle(&mut self.spaces);
        self.spaces_covered = generate_initial_spaces_covered();
    }

    pub fn spaces(&self) -> SpaceIter {
        SpaceIter {
            iter: self.spaces.iter().enumerate(),
            spaces_covered: self.spaces_covered,
        }
    }

    pub fn space(&self, index: usize) -> Space {
        Space {
            ball: self.spaces[index],
            covered: self.spaces_covered & (1 << index) != 0,
        }
    }
}

impl Default for BingoGrid {
    fn default() -> BingoGrid {
        BingoGrid::new()
    }
}

pub struct Space {
    pub ball: u8,
    pub covered: bool,
}

pub struct SpaceIter<'a> {
    iter: std::iter::Enumerate<std::slice::Iter<'a, u8>>,
    spaces_covered: u32,
}

impl<'a> Iterator for SpaceIter<'a> {
    type Item = Space;

    fn next(&mut self) -> Option<Space> {
        self.iter.next().map(|(index, &ball)| {
            Space {
                ball,
                covered: self.spaces_covered & (1 << index) != 0,
            }
        })
    }
}

fn mask_for_row(row: u32) -> u32 {
    ((1 << GRID_WIDTH) - 1) << (row * GRID_WIDTH as u32)
}

fn mask_for_column(column: u32) -> u32 {
    (0..GRID_HEIGHT as u32)
        .map(|y| 1 << (y * GRID_WIDTH as u32 + column))
        .fold(0, |a, b| a | b)
}

fn mask_for_diagonal_a() -> u32 {
    assert_eq!(GRID_WIDTH, GRID_HEIGHT);
    (0..GRID_WIDTH as u32)
        .map(|i| 1u32 << ((i * GRID_WIDTH as u32) + i))
        .fold(0, |a, b| a | b)
}

fn mask_for_diagonal_b() -> u32 {
    assert_eq!(GRID_WIDTH, GRID_HEIGHT);
    (0..GRID_WIDTH as u32)
        .map(|i| 1u32 << ((i * GRID_WIDTH as u32) + GRID_WIDTH as u32 - 1 - i))
        .fold(0, |a, b| a | b)
}

fn limit_for_mask(spaces_covered: u32, mask: u32, available: &mut u32) {
    if (spaces_covered & mask).count_ones() as usize
        >= MAX_INITIAL_COVERED_SPACES_PER_LINE
    {
        *available &= !mask;
    }
}

fn pick_nth_one_bit(mut bits: u32, mut n: u32) -> u32 {
    for i in 0..u32::BITS {
        if bits & 1 != 0 {
            match n.checked_sub(1) {
                Some(v) => n = v,
                None => return i,
            }
        }

        bits >>= 1;
    }

    unreachable!("Tried to pick bit {} but there wasn’t enough ones", n);
}

fn generate_initial_spaces_covered() -> u32 {
    let mut spaces_covered = 0;
    let mut available = (1u32 << N_SPACES as u32) - 1;

    for _ in 0..N_INITIAL_SPACES_COVERED {
        let n_available = available.count_ones() as usize;
        assert!(n_available > 0 && n_available <= N_SPACES);
        let chosen = pick_nth_one_bit(
            available,
            random::random_range(n_available) as u32,
        );

        assert!(spaces_covered & (1 << chosen) == 0);

        spaces_covered |= 1 << chosen;
        available &= !(1 << chosen);

        let row = chosen / GRID_WIDTH as u32;
        limit_for_mask(spaces_covered, mask_for_row(row), &mut available);

        let column = chosen % GRID_WIDTH as u32;
        limit_for_mask(spaces_covered, mask_for_column(column), &mut available);

        if row == column {
            limit_for_mask(
                spaces_covered,
                mask_for_diagonal_a(),
                &mut available,
            );
        }

        if GRID_HEIGHT as u32 - 1 - row == column {
            limit_for_mask(
                spaces_covered,
                mask_for_diagonal_b(),
                &mut available,
            );
        }
    }

    assert_eq!(spaces_covered.count_ones() as usize, N_INITIAL_SPACES_COVERED);

    spaces_covered
}
