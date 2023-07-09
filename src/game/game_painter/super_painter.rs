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

use std::rc::Rc;
use super::super::paint_data::PaintData;
use super::super::buffer::Buffer;
use super::super::{logic, timeout};
use super::super::array_object::ArrayObject;
use super::digit_tool;
use timeout::Timeout;
use glow::HasContext;

// Total number of quads to draw the two displays
const TOTAL_N_QUADS: usize = digit_tool::TOTAL_N_QUADS * 2;

pub struct SuperPainter {
    buffer: Rc<Buffer>,
    array_object: ArrayObject,
    paint_data: Rc<PaintData>,
    width: u32,
    height: u32,
    last_remaining_seconds: u32,
    vertices_dirty: bool,
    // Temporary buffer used for building the vertex buffer
    vertices: Vec<digit_tool::Vertex>,
}

impl SuperPainter {
    pub fn new(paint_data: Rc<PaintData>) -> Result<SuperPainter, String> {
        let buffer = create_score_buffer(&paint_data)?;
        let array_object = create_array_object(
            &paint_data,
            Rc::clone(&buffer),
        )?;

        Ok(SuperPainter {
            buffer,
            array_object,
            paint_data,
            width: 1,
            height: 1,
            last_remaining_seconds: u32::MAX,
            vertices_dirty: true,
            vertices: Vec::with_capacity(TOTAL_N_QUADS * 4),
        })
    }

    pub fn paint(&mut self, logic: &logic::Logic) -> Timeout {
        let Some(super_diveno) = logic.super_diveno()
        else {
            return Timeout::Forever;
        };

        self.update_vertices(&super_diveno);

        self.array_object.bind();

        let gl = &self.paint_data.gl;

        unsafe {
            gl.bind_texture(
                glow::TEXTURE_2D,
                Some(self.paint_data.images.segments.id()),
            );

            gl.use_program(Some(self.paint_data.shaders.score.id()));

            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.enable(glow::BLEND);

            gl.draw_elements(
                glow::TRIANGLES,
                self.vertices.len() as i32 / 4 * 6,
                glow::UNSIGNED_SHORT,
                0, // offset
            );

            gl.disable(glow::BLEND);
        }

        Timeout::Milliseconds(super_diveno.remaining_time() % 1000 + 1)
    }

    pub fn update_fb_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.vertices_dirty = true;
    }

    pub fn handle_logic_event(
        &mut self,
        logic: &logic::Logic,
        event: &logic::Event,
    ) -> bool {
        match event {
            logic::Event::WordChanged => false,
            logic::Event::GridChanged => false,
            logic::Event::GuessEntered => false,
            logic::Event::WrongGuessEntered => false,
            logic::Event::GuessRejected => false,
            logic::Event::CurrentPageChanged(_) => false,
            logic::Event::TombolaStartedSpinning(_) => false,
            logic::Event::BingoReset(_) => false,
            logic::Event::BingoChanged(..) => false,
            logic::Event::Bingo(..) => false,
            logic::Event::ScoreChanged(..) => false,
            logic::Event::CurrentTeamChanged => false,
            logic::Event::Solved => {
                if logic.super_diveno().is_some() {
                    self.vertices_dirty = true;
                    true
                } else {
                    false
                }
            },
            logic::Event::SuperDivenoToggled => {
                if logic.super_diveno().is_some() {
                    self.vertices_dirty = true;
                }
                true
            },
        }
    }

    fn fill_vertices_array(
        &mut self,
        remaining_seconds: u32,
        super_diveno: &logic::SuperDiveno,
    ) {
        self.vertices.clear();

        let mut digit_tool = digit_tool::DigitTool::new(
            &mut self.vertices,
            self.width,
            self.height,
        );

        digit_tool.add_display(-1.0, remaining_seconds);
        digit_tool.add_display(
            1.0 - digit_tool::DISPLAY_WIDTH,
            super_diveno.guessed_words(),
        );

        assert_eq!(self.vertices.len(), TOTAL_N_QUADS * 4);
    }

    fn update_vertices(&mut self, super_diveno: &logic::SuperDiveno) {
        let remaining_seconds =
            ((super_diveno.remaining_time() + 999) / 1000) as u32;

        if self.last_remaining_seconds == remaining_seconds
            && !self.vertices_dirty
        {
            return;
        }

        self.fill_vertices_array(remaining_seconds, super_diveno);

        let gl = &self.paint_data.gl;

        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.buffer.id()));

            let buffer_data = std::slice::from_raw_parts(
                self.vertices.as_ptr() as *const u8,
                self.vertices.len() * std::mem::size_of::<digit_tool::Vertex>(),
            );

            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                buffer_data,
                glow::DYNAMIC_DRAW,
            );
        }

        self.last_remaining_seconds = remaining_seconds;
        self.vertices_dirty = false;
    }
}

fn create_array_object(
    paint_data: &Rc<PaintData>,
    buffer: Rc<Buffer>,
) -> Result<ArrayObject, String> {
    let mut array_object = digit_tool::create_array_object(paint_data, buffer)?;

    paint_data.quad_tool.set_element_buffer(
        &mut array_object,
        TOTAL_N_QUADS as u32,
    )?;

    Ok(array_object)
}

fn create_score_buffer(paint_data: &PaintData) -> Result<Rc<Buffer>, String> {
    let buffer = Buffer::new(Rc::clone(&paint_data.gl))?;

    Ok(Rc::new(buffer))
}
