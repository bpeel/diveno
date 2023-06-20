mod letter_painter;

use std::rc::Rc;
use super::paint_data::PaintData;
use letter_painter::LetterPainter;
use crate::logic;
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

    pub fn handle_logic_event(&mut self, event: &logic::Event) {
        self.letter_painter.handle_logic_event(event);
    }
}
