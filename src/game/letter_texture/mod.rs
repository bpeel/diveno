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

mod data;

pub use data::N_LETTERS;
pub use data::N_COLORS;
pub use data::COLORS;

pub struct Color {
    pub letters: [Letter; N_LETTERS],
}

pub struct Letter {
    pub ch: char,
    pub s1: u16,
    pub t1: u16,
    pub s2: u16,
    pub t2: u16,
}
