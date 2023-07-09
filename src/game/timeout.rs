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

#[derive(Eq, PartialEq, PartialOrd, Ord, Debug, Clone, Copy)]
pub enum Timeout {
    Milliseconds(i64),
    Forever,
}

pub const IMMEDIATELY: Timeout = Timeout::Milliseconds(0);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn order() {
        assert!(Timeout::Forever > Timeout::Milliseconds(12));
        assert!(Timeout::Milliseconds(11) > Timeout::Milliseconds(10));

        assert_eq!(
            Timeout::Forever.min(Timeout::Milliseconds(12)),
            Timeout::Milliseconds(12),
        );
    }
}
