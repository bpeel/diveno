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
use super::super::{shaders, logic, tombola};
use super::super::array_object::ArrayObject;
use glow::HasContext;
use std::f32::consts::PI;

// Number of balls in a row of the ball texture
const N_BALLS_TEX_X: u32 = 11;
// Number of balls in a column of the ball texture
const N_BALLS_TEX_Y: u32 = 3;

const N_TOMBOLA_ELEMENTS: usize = (tombola::N_SIDES as usize + 1) * 2;

// Width of the side of the tombola, in the same units as the tombola module
const SIDE_WIDTH: f32 = tombola::BALL_SIZE / 2.0;

#[repr(C)]
struct Vertex {
    x: f32,
    y: f32,
    ox: u8,
    oy: u8,
    s: u16,
    t: u16,
    rotation: u16,
}

pub struct BingoPainter {
    team: logic::Team,
    buffer: Rc<Buffer>,
    array_object: ArrayObject,
    tombola_array_object: ArrayObject,
    paint_data: Rc<PaintData>,
    width: u32,
    height: u32,
    transform_dirty: bool,
    ball_width: f32,
    ball_height: f32,
    tombola_center_x: f32,
    tombola_center_y: f32,
    vertices_dirty: bool,
    ball_size_uniform: glow::UniformLocation,
    translation_uniform: glow::UniformLocation,
    scale_uniform: glow::UniformLocation,
    rotation_uniform: glow::UniformLocation,
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
        let buffer = create_vertex_buffer(&paint_data)?;
        let array_object = create_array_object(
            Rc::clone(&paint_data),
            Rc::clone(&buffer),
        )?;
        let ball_size_uniform = unsafe {
            match paint_data.gl.get_uniform_location(
                paint_data.shaders.ball.id(),
                "ball_size",
            ) {
                Some(u) => u,
                None => return Err("Missing “ball_size” uniform".to_string()),
            }
        };

        let translation_uniform = unsafe {
            match paint_data.gl.get_uniform_location(
                paint_data.shaders.tombola.id(),
                "translation",
            ) {
                Some(u) => u,
                None => return Err("Missing “translation” uniform".to_string()),
            }
        };

        let scale_uniform = unsafe {
            match paint_data.gl.get_uniform_location(
                paint_data.shaders.tombola.id(),
                "scale",
            ) {
                Some(u) => u,
                None => return Err("Missing “scale” uniform".to_string()),
            }
        };

        let rotation_uniform = unsafe {
            match paint_data.gl.get_uniform_location(
                paint_data.shaders.tombola.id(),
                "rotation",
            ) {
                Some(u) => u,
                None => return Err("Missing “rotation” uniform".to_string()),
            }
        };

        Ok(BingoPainter {
            team,
            buffer,
            array_object,
            tombola_array_object: create_tombola_array_object(&paint_data)?,
            paint_data,
            width: 1,
            height: 1,
            transform_dirty: true,
            ball_width: 1.0,
            ball_height: 1.0,
            tombola_center_x: 0.0,
            tombola_center_y: 0.0,
            vertices_dirty: true,
            ball_size_uniform,
            translation_uniform,
            scale_uniform,
            rotation_uniform,
            vertices: Vec::new(),
            most_quads: 0,
        })
    }

    pub fn paint(&mut self, logic: &mut logic::Logic) -> bool {
        logic.step_tombola(self.team);

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
                Some(self.paint_data.images.balls.id()),
            );

            gl.use_program(Some(self.paint_data.shaders.ball.id()));

            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.enable(glow::BLEND);

            gl.draw_elements(
                glow::TRIANGLES,
                self.vertices.len() as i32 / 4 * 6,
                glow::UNSIGNED_SHORT,
                0, // offset
            );

            gl.disable(glow::BLEND);

            self.tombola_array_object.bind();

            gl.bind_texture(
                glow::TEXTURE_2D,
                Some(self.paint_data.images.tombola.id()),
            );

            gl.use_program(Some(self.paint_data.shaders.tombola.id()));
            gl.uniform_1_f32(
                Some(&self.rotation_uniform),
                logic.tombola_rotation(self.team),
            );
            gl.draw_elements(
                glow::TRIANGLE_STRIP,
                N_TOMBOLA_ELEMENTS as i32,
                glow::UNSIGNED_BYTE,
                0, // offset
            );
        }

        if logic.tombola_is_sleeping(self.team) {
            false
        } else {
            self.vertices_dirty = true;
            true
        }
    }

    pub fn update_fb_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.transform_dirty = true;
    }

    fn update_transform(&mut self) {
        let inner_diameter = tombola::APOTHEM
            / (PI / tombola::N_SIDES as f32).cos()
            * 2.0;
        let diameter = inner_diameter + SIDE_WIDTH * 2.0;
        // Size of a ball along the smaller axis. The tombola diameter
        // is 1 unit along the small axis.
        let small_size = tombola::BALL_SIZE / diameter;

        let (ball_w, ball_h) = if self.width < self.height {
            (small_size, small_size * self.width as f32 / self.height as f32)
        } else {
            (small_size * self.height as f32 / self.width as f32, small_size)
        };

        self.ball_width = ball_w;
        self.ball_height = ball_h;
        self.tombola_center_x = 0.0;
        self.tombola_center_y = diameter / 2.0 * ball_h / tombola::BALL_SIZE;

        let gl = &self.paint_data.gl;

        unsafe {
            gl.use_program(Some(self.paint_data.shaders.ball.id()));
            gl.uniform_2_f32(
                Some(&self.ball_size_uniform),
                ball_w,
                ball_h,
            );

            gl.use_program(Some(self.paint_data.shaders.tombola.id()));
            gl.uniform_2_f32(
                Some(&self.translation_uniform),
                self.tombola_center_x,
                self.tombola_center_y,
            );
            gl.uniform_2_f32(
                Some(&self.scale_uniform),
                self.ball_width / tombola::BALL_SIZE,
                self.ball_height / tombola::BALL_SIZE,
            );
        }

        self.vertices_dirty = true;
    }

    pub fn handle_logic_event(
        &mut self,
        _logic: &logic::Logic,
        event: &logic::Event,
    ) -> bool {
        match event {
            logic::Event::TombolaStartedSpinning(team) => {
                if *team == self.team {
                    self.vertices_dirty = true;
                    true
                } else {
                    false
                }
            },
            logic::Event::WordChanged => false,
            logic::Event::GridChanged => false,
            logic::Event::GuessEntered => false,
            logic::Event::WrongGuessEntered => false,
            logic::Event::GuessRejected => false,
            logic::Event::Solved => false,
            logic::Event::ScoreChanged(_) => false,
            logic::Event::CurrentTeamChanged => false,
            logic::Event::CurrentPageChanged(_) => false,
        }
    }

    fn fill_vertices_array(
        &mut self,
        logic: &logic::Logic,
    ) {
        self.vertices.clear();

        for ball in logic.balls(self.team) {
            let ball_num = match ball.ball_type {
                tombola::BallType::Number(n) => n as u32 - 1,
                tombola::BallType::Black => 25,
            };

            self.add_ball(
                ball_num,
                ball.x
                    * self.ball_width
                    / tombola::BALL_SIZE as f32
                    + self.tombola_center_x,
                ball.y
                    * self.ball_height
                    / tombola::BALL_SIZE as f32
                    + self.tombola_center_y,
                ball.rotation,
            );
        }
    }

    fn axis_tex_coord_for_ball(
        ball_num: u32,
        n_balls_axis: u32,
    ) -> (u16, u16) {
        let n_units = (n_balls_axis - 1) * 3 + 2;
        (
            (ball_num * 3 * 65535 / n_units) as u16,
            ((ball_num * 3 + 2) * 65535 / n_units) as u16,
        )
    }

    fn add_ball(
        &mut self,
        ball_num: u32,
        x: f32,
        y: f32,
        rotation: f32,
    ) {
        let (s1, s2) = BingoPainter::axis_tex_coord_for_ball(
            ball_num % N_BALLS_TEX_X,
            N_BALLS_TEX_X,
        );
        let (t1, t2) = BingoPainter::axis_tex_coord_for_ball(
            ball_num / N_BALLS_TEX_X,
            N_BALLS_TEX_Y,
        );

        // Normalise the rotation angle as 0->65535
        let normalised_rotation = (rotation / (2.0 * PI)).fract();
        let positive_rotation = if normalised_rotation < 0.0 {
            1.0 + normalised_rotation
        } else {
            normalised_rotation
        };
        let rotation = (positive_rotation * 65535.0).round() as u16;

        self.vertices.push(Vertex {
            x,
            y,
            ox: 0,
            oy: 0,
            s: s1,
            t: t2,
            rotation,
        });
        self.vertices.push(Vertex {
            x,
            y,
            ox: 255,
            oy: 0,
            s: s2,
            t: t2,
            rotation,
        });
        self.vertices.push(Vertex {
            x,
            y,
            ox: 0,
            oy: 255,
            s: s1,
            t: t1,
            rotation,
        });
        self.vertices.push(Vertex {
            x,
            y,
            ox: 255,
            oy: 255,
            s: s2,
            t: t1,
            rotation,
        });
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

fn create_array_object(
    paint_data: Rc<PaintData>,
    buffer: Rc<Buffer>,
) -> Result<ArrayObject, String> {
    let rotation_attrib = unsafe {
        match paint_data.gl.get_attrib_location(
            paint_data.shaders.ball.id(),
            "rotation",
        ) {
            Some(l) => l,
            None => return Err("Missing “rotation” attrib".to_string()),
        }
    };

    let position_offset_attrib = unsafe {
        match paint_data.gl.get_attrib_location(
            paint_data.shaders.ball.id(),
            "position_offset",
        ) {
            Some(l) => l,
            None => return Err("Missing “position_offset” attrib".to_string()),
        }
    };

    let mut array_object = ArrayObject::new(paint_data)?;
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
        position_offset_attrib,
        2, // size
        glow::UNSIGNED_BYTE,
        true, // normalized
        std::mem::size_of::<Vertex>() as i32,
        Rc::clone(&buffer),
        offset,
    );
    offset += std::mem::size_of::<u8>() as i32 * 2;

    array_object.set_attribute(
        shaders::TEX_COORD_ATTRIB,
        2, // size
        glow::UNSIGNED_SHORT,
        true, // normalized
        std::mem::size_of::<Vertex>() as i32,
        Rc::clone(&buffer),
        offset,
    );
    offset += std::mem::size_of::<u16>() as i32 * 2;

    array_object.set_attribute(
        rotation_attrib,
        1, // size
        glow::UNSIGNED_SHORT,
        true, // normalized
        std::mem::size_of::<Vertex>() as i32,
        Rc::clone(&buffer),
        offset,
    );

    Ok(array_object)
}

fn create_vertex_buffer(paint_data: &PaintData) -> Result<Rc<Buffer>, String> {
    let buffer = Buffer::new(Rc::clone(&paint_data.gl))?;

    Ok(Rc::new(buffer))
}

#[repr(C)]
struct TombolaVertex {
    x: f32,
    y: f32,
    s: u16,
    t: u16,
}

fn create_tombola_buffer(
    paint_data: &PaintData,
) -> Result<Rc<Buffer>, String> {
    let inner_radius = tombola::APOTHEM / (PI / tombola::N_SIDES as f32).cos();
    let outer_radius = inner_radius + SIDE_WIDTH;
    let mut vertices = Vec::with_capacity(tombola::N_SIDES as usize * 2);

    for side in 0..tombola::N_SIDES {
        let angle = side as f32
            * 2.0 * PI
            / tombola::N_SIDES as f32;
        let sin_angle = angle.sin();
        let cos_angle = angle.cos();

        vertices.push(TombolaVertex {
            x: sin_angle * outer_radius,
            y: cos_angle * outer_radius,
            s: 32767,
            t: 65535,
        });
        vertices.push(TombolaVertex {
            x: sin_angle * inner_radius,
            y: cos_angle * inner_radius,
            s: 32767,
            t: 0,
        });
    }

    assert_eq!(vertices.len(), tombola::N_SIDES as usize * 2);

    let buffer = Buffer::new(Rc::clone(&paint_data.gl))?;

    let gl = &paint_data.gl;

    unsafe {
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer.id()));

        let buffer_data = std::slice::from_raw_parts(
            vertices.as_ptr() as *const u8,
            vertices.len() * std::mem::size_of::<TombolaVertex>(),
        );

        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            buffer_data,
            glow::STATIC_DRAW,
        );
    }

    Ok(Rc::new(buffer))
}

fn set_tombola_element_buffer(
    paint_data: &PaintData,
    array_object: &mut ArrayObject,
) -> Result<(), String> {
    let mut elements = Vec::with_capacity(N_TOMBOLA_ELEMENTS);

    for side in 0..tombola::N_SIDES as u8 {
        elements.push(side * 2);
        elements.push(side * 2 + 1);
    }

    elements.push(0);
    elements.push(1);

    assert_eq!(elements.len(), N_TOMBOLA_ELEMENTS);

    let buffer = Rc::new(Buffer::new(Rc::clone(&paint_data.gl))?);
    array_object.set_element_buffer(buffer);

    unsafe {
        paint_data.gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            &elements,
            glow::STATIC_DRAW,
        );
    }

    Ok(())
}

fn create_tombola_array_object(
    paint_data: &Rc<PaintData>
) -> Result<ArrayObject, String> {
    let buffer = create_tombola_buffer(paint_data)?;

    let mut array_object = ArrayObject::new(Rc::clone(paint_data))?;
    let mut offset = 0;

    array_object.set_attribute(
        shaders::POSITION_ATTRIB,
        2, // size
        glow::FLOAT,
        false, // normalized
        std::mem::size_of::<TombolaVertex>() as i32,
        Rc::clone(&buffer),
        offset,
    );
    offset += std::mem::size_of::<f32>() as i32 * 2;

    array_object.set_attribute(
        shaders::TEX_COORD_ATTRIB,
        2, // size
        glow::UNSIGNED_SHORT,
        true, // normalized
        std::mem::size_of::<TombolaVertex>() as i32,
        buffer,
        offset,
    );

    set_tombola_element_buffer(&paint_data, &mut array_object)?;

    Ok(array_object)
}
