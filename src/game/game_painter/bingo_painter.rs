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
use super::super::{logic, bingo_grid};
use super::super::array_object::ArrayObject;
use glow::HasContext;
use nalgebra::{Matrix4, Vector3};
use super::letter_vertex;
use letter_vertex::Vertex;

const TEX_SPACES_X: u32 = 8;
const TEX_SPACES_Y: u32 = 4;

pub struct BingoPainter {
    team: logic::Team,
    buffer: Rc<Buffer>,
    array_object: ArrayObject,
    paint_data: Rc<PaintData>,
    width: u32,
    height: u32,
    transform_dirty: bool,
    vertices_dirty: bool,
    mvp_uniform: glow::UniformLocation,
    mvp_matrix: Matrix4<f32>,
    // Temporary buffer used for building the vertex buffer
    vertices: Vec<Vertex>,
    // Used to keep track of whether we need to create a new quad buffer
    most_quads: u32,
}

impl BingoPainter {
    pub fn new(
        paint_data: Rc<PaintData>,
        team: logic::Team,
    ) -> Result<BingoPainter, String> {
        let buffer = Rc::new(Buffer::new(Rc::clone(&paint_data.gl))?);

        let array_object = letter_vertex::create_array_object(
            Rc::clone(&paint_data),
            Rc::clone(&buffer),
        )?;

        let mvp_uniform = unsafe {
            match paint_data.gl.get_uniform_location(
                paint_data.shaders.letter.id(),
                "mvp",
            ) {
                Some(u) => u,
                None => return Err("Missing “mvp” uniform".to_string()),
            }
        };

        Ok(BingoPainter {
            team,
            buffer,
            array_object,
            paint_data,
            width: 1,
            height: 1,
            transform_dirty: true,
            vertices_dirty: true,
            mvp_uniform,
            mvp_matrix: Default::default(),
            vertices: Vec::new(),
            most_quads: 0,
        })
    }

    pub fn paint(&mut self, logic: &logic::Logic) -> bool {
        if self.transform_dirty {
            self.update_transform();
            self.transform_dirty = false;
        }

        if self.vertices_dirty {
            self.update_vertices(logic);
            self.vertices_dirty = false;
        }

        self.array_object.bind();

        let gl = &self.paint_data.gl;

        unsafe {
            gl.bind_texture(
                glow::TEXTURE_2D,
                Some(self.paint_data.images.bingo.id()),
            );

            gl.use_program(Some(self.paint_data.shaders.letter.id()));

            gl.uniform_matrix_4_f32_slice(
                Some(&self.mvp_uniform),
                false, // transpose
                self.mvp_matrix.as_slice(),
            );

            gl.draw_elements(
                glow::TRIANGLES,
                self.vertices.len() as i32 / 4 * 6,
                glow::UNSIGNED_SHORT,
                0, // offset
            );
        }

        false
    }

    pub fn update_fb_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.transform_dirty = true;
    }

    pub fn handle_logic_event(
        &mut self,
        _logic: &logic::Logic,
        _event: &logic::Event,
    ) -> bool {
        false
    }

    fn update_transform(&mut self) {
        let bingo_ratio = bingo_grid::GRID_WIDTH as f32
            / bingo_grid::GRID_HEIGHT as f32;
        let fb_ratio = self.width as f32 / self.height as f32;
        let space_width;
        let space_height;

        if bingo_ratio > fb_ratio {
            // Fit the width
            space_width = 1.0 / bingo_grid::GRID_WIDTH as f32;
            space_height = space_width * fb_ratio;
        } else {
            // Fit the height
            space_height = 1.0 / bingo_grid::GRID_HEIGHT as f32;
            space_width = space_height / fb_ratio;
        }

        self.mvp_matrix = Matrix4::new_translation(&Vector3::new(
            0.0,
            -0.5,
            0.0,
        )).prepend_nonuniform_scaling(&Vector3::new(
            space_width,
            -space_height,
            1.0,
        )).prepend_translation(&Vector3::new(
            bingo_grid::GRID_WIDTH as f32 / -2.0,
            bingo_grid::GRID_HEIGHT as f32 / -2.0,
            0.0,
        ));

        self.vertices_dirty = true;
    }

    fn fill_vertices_array(
        &mut self,
        logic: &logic::Logic,
    ) {
        self.vertices.clear();

        for (index, space) in logic.bingo_grid(self.team).spaces().enumerate() {
            let x = (index % bingo_grid::GRID_WIDTH) as f32;
            let y = (index / bingo_grid::GRID_WIDTH) as f32;
            let tex_x = space.ball as u32 % TEX_SPACES_X;
            let tex_y = space.ball as u32 / TEX_SPACES_X;
            let s1 = (tex_x * 65535 / TEX_SPACES_X) as u16;
            let t1 = (tex_y * 65535 / TEX_SPACES_Y) as u16;
            let s2 = ((tex_x + 1) * 65535 / TEX_SPACES_X) as u16;
            let t2 = ((tex_y + 1) * 65535 / TEX_SPACES_Y) as u16;
            let color = if space.covered {
                [0xe7, 0x00, 0x2a]
            } else {
                [0x00, 0x77, 0xc7]
            };

            self.vertices.push(Vertex {
                x,
                y,
                s: s1,
                t: t1,
                ry: 0.0,
                rp: 0.0,
                color,
            });
            self.vertices.push(Vertex {
                x,
                y: y + 1.0,
                s: s1,
                t: t2,
                ry: 0.0,
                rp: 0.0,
                color,
            });
            self.vertices.push(Vertex {
                x: x + 1.0,
                y,
                s: s2,
                t: t1,
                ry: 0.0,
                rp: 0.0,
                color,
            });
            self.vertices.push(Vertex {
                x: x + 1.0,
                y: y + 1.0,
                s: s2,
                t: t2,
                ry: 0.0,
                rp: 0.0,
                color,
            });
        }
    }

    fn update_vertices(
        &mut self,
        logic: &logic::Logic,
    ) {
        self.fill_vertices_array(logic);

        let n_quads = self.vertices.len() as u32 / 4;

        if n_quads > self.most_quads {
            match self.paint_data.quad_tool.set_element_buffer(
                &mut self.array_object,
                n_quads,
            ) {
                Ok(most_quads) => self.most_quads = most_quads,
                Err(_) => return,
            }
        }

        let gl = &self.paint_data.gl;

        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.buffer.id()));

            let buffer_data = std::slice::from_raw_parts(
                self.vertices.as_ptr() as *const u8,
                self.vertices.len() * std::mem::size_of::<Vertex>(),
            );

            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                buffer_data,
                glow::DYNAMIC_DRAW,
            );
        }
    }
}
