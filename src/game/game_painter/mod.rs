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

mod letter_painter;

use std::rc::Rc;
use super::paint_data::PaintData;
use letter_painter::LetterPainter;
use super::logic;
use glow::HasContext;

pub struct GamePainter {
    paint_data: Rc<PaintData>,
    letter_painter: LetterPainter,
    width: u32,
    height: u32,
    viewport_dirty: bool,
}

impl GamePainter {
    pub fn new(paint_data: Rc<PaintData>) -> Result<GamePainter, String> {
        let gl = &paint_data.gl;

        unsafe {
            gl.enable(glow::CULL_FACE);
        }

        Ok(GamePainter {
            paint_data: Rc::clone(&paint_data),
            letter_painter: LetterPainter::new(paint_data)?,
            width: 1,
            height: 1,
            viewport_dirty: true,
        })
    }

    pub fn paint(&mut self, logic: &logic::Logic) -> bool {
        let gl = &self.paint_data.gl;

        if self.viewport_dirty {
            unsafe {
                gl.viewport(0, 0, self.width as i32, self.height as i32);
            }
            self.viewport_dirty = false;
        }

        unsafe {
            gl.clear_color(0.0, 0.0, 1.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }

        self.letter_painter.paint(logic)
    }

    pub fn update_fb_size(&mut self, width: u32, height: u32) {
        self.viewport_dirty = true;
        self.width = width;
        self.height = height;

        self.letter_painter.update_fb_size(width, height);
    }

    pub fn handle_logic_event(
        &mut self,
        logic: &logic::Logic,
        event: &logic::Event,
    ) {
        self.letter_painter.handle_logic_event(logic, event);
    }
}
