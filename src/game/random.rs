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

#[cfg(not(target_arch = "wasm32"))]
use rand::Rng;

pub fn random_range(max: usize) -> usize {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut rng = rand::thread_rng();
        rng.gen_range(0..max)
    }

    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Math::random() * max as f64).floor() as usize
    }
}

pub fn shuffle<T>(slice: &mut [T]) {
    for i in (1..slice.len()).rev() {
        let j = random_range(i + 1);
        if i != j {
            let (a, b) = slice.split_at_mut(i);
            std::mem::swap(&mut a[j], &mut b[0]);
        }
    }
}
