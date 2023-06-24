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

#[derive(Clone, Copy)]
pub struct Timer {
    #[cfg(target_arch = "wasm32")]
    start_time: f64,
    #[cfg(not(target_arch = "wasm32"))]
    start_time: std::time::Instant,
}

impl Timer {
    #[cfg(target_arch = "wasm32")]
    fn now() -> f64 {
        web_sys::window().and_then(|w| {
            w.performance().map(|p| p.now())
        }).unwrap_or(0.0)
    }

    pub fn new() -> Timer {
        let start_time = {
            #[cfg(target_arch = "wasm32")]
            {
                Timer::now()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::time::Instant::now()
            }
        };

        Timer {
            start_time
        }
    }

    pub fn elapsed(&self) -> i64 {
        #[cfg(target_arch = "wasm32")]
        {
            (Timer::now() - self.start_time) as i64
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.start_time.elapsed().as_millis() as i64
        }
    }
}
