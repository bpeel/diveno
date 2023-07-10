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
use super::super::array_object::ArrayObject;
use super::super::shaders;

// Number of digits in the display
const N_DIGITS: usize = 3;
// Number of quads needed to draw the frame
const N_FRAME_QUADS: usize = 8;
// Number of quads needed to draw the inner gap
const N_INNER_GAP_QUADS: usize = 4;
// Number of quads needed to draw the colon
pub const N_COLON_QUADS: usize = 5;
// Total number of quads to draw a display without a colon
pub const TOTAL_N_QUADS: usize =
    N_DIGITS + N_FRAME_QUADS + N_INNER_GAP_QUADS;

// The total width allocated to the display
pub const DISPLAY_WIDTH: f32 = 2.0 / 4.0;
// The empty gap surrounding the frame
pub const OUTER_GAP_SIZE: f32 = DISPLAY_WIDTH / 16.0;
// The width of the frame
const FRAME_WIDTH: f32 = DISPLAY_WIDTH / 8.0;
// The empty gap inside the frame
const INNER_GAP_SIZE: f32 = DISPLAY_WIDTH / 16.0;

// The total logical space occupied by a display
pub const TOTAL_HEIGHT: f32 = DIGIT_HEIGHT
    + (INNER_GAP_SIZE + FRAME_WIDTH + OUTER_GAP_SIZE) * 2.0;

pub const TEX_WIDTH: u32 = 1024;
pub const TEX_HEIGHT: u32 = 128;

// Width of the frame in pixels in the image
const FRAME_PIXEL_WIDTH: u32 = 40;
// Width of the frame in texture coordinates
const FRAME_TEX_WIDTH: u16 = (FRAME_PIXEL_WIDTH * 65535 / TEX_WIDTH) as u16;
// Height of the frame in texture coordinates
const FRAME_TEX_HEIGHT: u16 = (FRAME_PIXEL_WIDTH * 65535 / TEX_HEIGHT) as u16;
// Texture coordinate of the left side of the frame
const FRAME_TEX_LEFT: u16 = ((TEX_WIDTH - 100) * 65535 / TEX_WIDTH) as u16;
// Total width of all the digits in texture coordinates
const DIGITS_TEX_WIDTH: u16 = 56867;

const DIGIT_WIDTH: f32 = (DISPLAY_WIDTH
                          - (OUTER_GAP_SIZE + FRAME_WIDTH + INNER_GAP_SIZE)
                          * 2.0)
    / N_DIGITS as f32;
const DIGIT_HEIGHT: f32 = DIGIT_WIDTH
    * TEX_HEIGHT as f32
    / (DIGITS_TEX_WIDTH as f32
       / u16::MAX as f32
       / 10.0
       * TEX_WIDTH as f32);
const COLON_WIDTH: f32 = OUTER_GAP_SIZE / 2.0;

// Tex coords of a known black texel
const GAP_TEX_S: u16 = ((65535 + FRAME_TEX_LEFT as u32) / 2) as u16;
const GAP_TEX_T: u16 = u16::MAX / 2;

// Tex coords of a known green texel
const COLON_TEX_S: u16 = (65535 * 20 / TEX_WIDTH) as u16;
const COLON_TEX_T: u16 = (65535 * 30 / TEX_HEIGHT) as u16;

#[repr(C)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub s: u16,
    pub t: u16,
}

pub struct DigitTool<'a> {
    vertices: &'a mut Vec<Vertex>,
    width: u32,
    height: u32,
}

impl<'a> DigitTool<'a> {
    pub fn new(
        vertices: &'a mut Vec<Vertex>,
        fb_width: u32,
        fb_height: u32,
    ) -> DigitTool {
        DigitTool {
            vertices,
            width: fb_width,
            height: fb_height,
        }
    }

    fn add_quad(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        s1: u16,
        t1: u16,
        s2: u16,
        t2: u16,
    ) {
        self.vertices.push(Vertex { x: x1, y: y1, s: s1, t: t1, });
        self.vertices.push(Vertex { x: x1, y: y2, s: s1, t: t2, });
        self.vertices.push(Vertex { x: x2, y: y1, s: s2, t: t1, });
        self.vertices.push(Vertex { x: x2, y: y2, s: s2, t: t2, });
    }

    fn left_right(x: f32, with_colon: bool) -> (f32, f32) {
        let left = x + OUTER_GAP_SIZE;
        let right = x + DISPLAY_WIDTH - OUTER_GAP_SIZE;

        if with_colon {
            (left - COLON_WIDTH / 2.0, right + COLON_WIDTH / 2.0)
        } else {
            (left, right)
        }
    }

    fn add_frame(&mut self, x: f32, with_colon: bool) {
        let y_scale = self.width as f32 / self.height as f32;

        let (left, right) = DigitTool::left_right(x, with_colon);
        let top = (DIGIT_HEIGHT / 2.0 + INNER_GAP_SIZE + FRAME_WIDTH) * y_scale;
        let bottom = -top;

        // Left side
        self.add_quad(
            left,
            top - FRAME_WIDTH * y_scale,
            left + FRAME_WIDTH,
            bottom + FRAME_WIDTH * y_scale,
            FRAME_TEX_LEFT,
            u16::MAX / 2,
            FRAME_TEX_LEFT + FRAME_TEX_WIDTH,
            u16::MAX / 2,
        );
        // Right side
        self.add_quad(
            right - FRAME_WIDTH,
            top - FRAME_WIDTH * y_scale,
            right,
            bottom + FRAME_WIDTH * y_scale,
            u16::MAX - FRAME_TEX_WIDTH,
            u16::MAX / 2,
            u16::MAX,
            u16::MAX / 2,
        );
        // Top side
        self.add_quad(
            left + FRAME_WIDTH,
            top,
            right - FRAME_WIDTH,
            top - FRAME_WIDTH * y_scale,
            GAP_TEX_S,
            0,
            GAP_TEX_S,
            FRAME_TEX_HEIGHT,
        );
        // Bottom side
        self.add_quad(
            left + FRAME_WIDTH,
            bottom + FRAME_WIDTH * y_scale,
            right - FRAME_WIDTH,
            bottom,
            GAP_TEX_S,
            u16::MAX - FRAME_TEX_HEIGHT,
            GAP_TEX_S,
            u16::MAX,
        );

        // Top-left corner
        self.add_quad(
            left,
            top,
            left + FRAME_WIDTH,
            top - FRAME_WIDTH * y_scale,
            FRAME_TEX_LEFT,
            0,
            FRAME_TEX_LEFT + FRAME_TEX_WIDTH,
            FRAME_TEX_HEIGHT,
        );
        // Top-right corner
        self.add_quad(
            right - FRAME_WIDTH,
            top,
            right,
            top - FRAME_WIDTH * y_scale,
            u16::MAX - FRAME_TEX_WIDTH,
            0,
            u16::MAX,
            FRAME_TEX_HEIGHT,
        );
        // Bottom-left corner
        self.add_quad(
            left,
            bottom + FRAME_WIDTH * y_scale,
            left + FRAME_WIDTH,
            bottom,
            FRAME_TEX_LEFT,
            u16::MAX - FRAME_TEX_HEIGHT,
            FRAME_TEX_LEFT + FRAME_TEX_WIDTH,
            u16::MAX,
        );
        // Bottom-right corner
        self.add_quad(
            right - FRAME_WIDTH,
            bottom + FRAME_WIDTH * y_scale,
            right,
            bottom,
            u16::MAX - FRAME_TEX_WIDTH,
            u16::MAX - FRAME_TEX_HEIGHT,
            u16::MAX,
            u16::MAX,
        );
    }

    fn add_gap_quad(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.add_quad(
            x1,
            y1,
            x2,
            y2,
            GAP_TEX_S,
            GAP_TEX_T,
            GAP_TEX_S,
            GAP_TEX_T,
        );
    }

    fn add_colon_quad(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.add_quad(
            x1,
            y1,
            x2,
            y2,
            COLON_TEX_S,
            COLON_TEX_T,
            COLON_TEX_S,
            COLON_TEX_T,
        );
    }

    fn add_inner_gap(&mut self, x: f32, with_colon: bool) {
        let y_scale = self.width as f32 / self.height as f32;

        let (left, right) = DigitTool::left_right(x, with_colon);

        let left = left + FRAME_WIDTH;
        let right = right - FRAME_WIDTH;
        let top = (DIGIT_HEIGHT / 2.0 + INNER_GAP_SIZE) * y_scale;
        let bottom = -top;

        // Left side
        self.add_gap_quad(
            left,
            top,
            left + INNER_GAP_SIZE,
            bottom,
        );
        // Right side
        self.add_gap_quad(
            right - INNER_GAP_SIZE,
            top,
            right,
            bottom,
        );
        // Top side
        self.add_gap_quad(
            left + INNER_GAP_SIZE,
            top,
            right - INNER_GAP_SIZE,
            top - INNER_GAP_SIZE * y_scale,
        );
        // Bottom side
        self.add_gap_quad(
            left + INNER_GAP_SIZE,
            bottom + INNER_GAP_SIZE * y_scale,
            right - INNER_GAP_SIZE,
            bottom,
        );
    }

    fn add_colon(&mut self, left: f32, right: f32, top: f32, bottom: f32) {
        let part_size = (top - bottom) / 8.0;

        self.add_gap_quad(
            left,
            top,
            right,
            top - part_size * 2.0,
        );
        self.add_colon_quad(
            left,
            top - part_size * 2.0,
            right,
            top - part_size * 3.0,
        );
        self.add_gap_quad(
            left,
            top - part_size * 3.0,
            right,
            top - part_size * 5.0,
        );
        self.add_colon_quad(
            left,
            top - part_size * 5.0,
            right,
            top - part_size * 6.0,
        );
        self.add_gap_quad(
            left,
            top - part_size * 6.0,
            right,
            bottom,
        );
    }

    fn add_digits(&mut self, x: f32, with_colon: bool, mut value: u32) {
        let y_scale = self.width as f32 / self.height as f32;

        let (edge_left, edge_right) = DigitTool::left_right(x, with_colon);
        let edge_left = edge_left + FRAME_WIDTH + INNER_GAP_SIZE;
        let edge_right = edge_right - FRAME_WIDTH - INNER_GAP_SIZE;
        let mut right = edge_right;
        let top = DIGIT_HEIGHT / 2.0 * y_scale;
        let bottom = -top;

        for digit_num in 0..N_DIGITS {
            if with_colon && digit_num == N_DIGITS - 1 {
                let left = right - COLON_WIDTH;
                self.add_colon(left, right, top, bottom);
                right = left;
            }

            let (s1, t1, s2, t2) = if value <= 0 && digit_num > 0{
                (GAP_TEX_S, GAP_TEX_T, GAP_TEX_S, GAP_TEX_T)
            } else {
                let digit = value % 10;

                (
                    (DIGITS_TEX_WIDTH as u32 * digit / 10) as u16,
                    0,
                    (DIGITS_TEX_WIDTH as u32 * (digit + 1) / 10) as u16,
                    u16::MAX,
                )
            };

            let left = if digit_num == N_DIGITS - 1 {
                edge_left
            } else {
                right - DIGIT_WIDTH
            };

            self.add_quad(
                left,
                top,
                right,
                bottom,
                s1,
                t1,
                s2,
                t2,
            );

            value /= 10;
            right = left;
        }
    }

    pub fn add_display(&mut self, x: f32, value: u32, with_colon: bool) {
        self.add_frame(x, with_colon);
        self.add_inner_gap(x, with_colon);
        self.add_digits(x, with_colon, value);
    }
}

pub fn create_array_object(
    paint_data: &Rc<PaintData>,
    buffer: Rc<Buffer>,
) -> Result<ArrayObject, String> {
    let mut array_object = ArrayObject::new(Rc::clone(paint_data))?;
    let mut offset = 0;

    array_object.set_attribute(
        shaders::POSITION_ATTRIB,
        2, // size
        glow::FLOAT,
        false, // normalized
        std::mem::size_of::<Vertex>() as i32,
        Rc::clone(&buffer),
        offset,
    );
    offset += std::mem::size_of::<f32>() as i32 * 2;

    array_object.set_attribute(
        shaders::TEX_COORD_ATTRIB,
        2, // size
        glow::UNSIGNED_SHORT,
        true, // normalized
        std::mem::size_of::<Vertex>() as i32,
        Rc::clone(&buffer),
        offset,
    );

    Ok(array_object)
}
