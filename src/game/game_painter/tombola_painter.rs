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

const N_SIDES_ELEMENTS: usize = (tombola::N_SIDES as usize + 1) * 2;
const FIRST_CLAW_VERTEX: usize = tombola::N_SIDES as usize * 2;
const N_CLAW_VERTICES: usize = 4;
const FIRST_WALL_VERTEX: usize = FIRST_CLAW_VERTEX + N_CLAW_VERTICES;
const N_WALL_VERTICES: usize = 8;

// Width of the side of the tombola, in the same units as the tombola module
const SIDE_WIDTH: f32 = tombola::BALL_SIZE / 2.0;

// Dimensions of the claw in the same units as the tombola module
const CLAW_WIDTH: f32 = tombola::BALL_SIZE / 63.7 * 83.328;
const CLAW_HEIGHT: f32 = CLAW_WIDTH * 2.0;

// Width of the walls to the sides of the tombola and the slope
const WALL_WIDTH: f32 = SIDE_WIDTH;

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

pub struct TombolaPainter {
    team: logic::Team,
    buffer: Rc<Buffer>,
    balls_array_object: ArrayObject,
    sides_array_object: ArrayObject,
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

impl TombolaPainter {
    pub fn new(
        paint_data: Rc<PaintData>,
        team: logic::Team,
    ) -> Result<TombolaPainter, String> {
        let buffer = create_vertex_buffer(&paint_data)?;
        let balls_array_object = create_array_object(
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

        Ok(TombolaPainter {
            team,
            buffer,
            balls_array_object,
            sides_array_object: create_sides_array_object(&paint_data)?,
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

        self.balls_array_object.bind();

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

            self.sides_array_object.bind();

            gl.bind_texture(
                glow::TEXTURE_2D,
                Some(self.paint_data.images.tombola.id()),
            );

            gl.use_program(Some(self.paint_data.shaders.tombola.id()));

            gl.uniform_2_f32(
                Some(&self.translation_uniform),
                self.tombola_center_x,
                self.tombola_center_y,
            );

            gl.uniform_1_f32(
                Some(&self.rotation_uniform),
                logic.tombola_rotation(self.team),
            );
            gl.draw_elements(
                glow::TRIANGLE_STRIP,
                N_SIDES_ELEMENTS as i32,
                glow::UNSIGNED_BYTE,
                0, // offset
            );

            gl.uniform_1_f32(
                Some(&self.rotation_uniform),
                0.0,
            );

            gl.draw_arrays(
                glow::TRIANGLE_STRIP,
                FIRST_WALL_VERTEX as i32,
                N_WALL_VERTICES as i32,
            );

            gl.enable(glow::BLEND);

            gl.bind_texture(
                glow::TEXTURE_2D,
                Some(self.paint_data.images.claw.id()),
            );

            let (claw_x, claw_y) = logic.claw_pos(self.team);

            gl.uniform_2_f32(
                Some(&self.translation_uniform),
                self.tombola_center_x
                    + claw_x / tombola::BALL_SIZE * self.ball_width,
                self.tombola_center_y
                    + claw_y / tombola::BALL_SIZE * self.ball_height,
            );

            gl.draw_arrays(
                glow::TRIANGLE_STRIP,
                FIRST_CLAW_VERTEX as i32,
                N_CLAW_VERTICES as i32,
            );

            gl.disable(glow::BLEND);
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
        let top = tombola::APOTHEM
            / (PI / tombola::N_SIDES as f32).cos()
            + tombola::BALL_SIZE * 2.0;
        let bottom = tombola::LEFT_SLOPE_Y - WALL_WIDTH;
        let left = -tombola::WALL_X - WALL_WIDTH;
        let right = tombola::WALL_X + WALL_WIDTH;
        let tombola_ratio = (right - left) / (top - bottom);
        let screen_ratio = self.width as f32 / self.height as f32;
        let ball_w;
        let ball_h;

        if tombola_ratio > screen_ratio {
            // Fit the width
            ball_w = tombola::BALL_SIZE / (right - left);
            let y_axis_height = screen_ratio / tombola_ratio;
            ball_h = tombola::BALL_SIZE / (top - bottom) * y_axis_height;
        } else {
            // Fit the height
            ball_h = tombola::BALL_SIZE / (top - bottom);
            let x_axis_length = tombola_ratio / screen_ratio;
            ball_w = tombola::BALL_SIZE / (right - left) * x_axis_length;
        }

        self.ball_width = ball_w;
        self.ball_height = ball_h;
        self.tombola_center_x = 0.0;
        self.tombola_center_y = 0.5 +
            ((top - bottom) / 2.0 - top) * ball_h / tombola::BALL_SIZE;

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
                logic::BallType::Number(n) => n,
                logic::BallType::Black => 25,
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
        let (s1, s2) = TombolaPainter::axis_tex_coord_for_ball(
            ball_num % N_BALLS_TEX_X,
            N_BALLS_TEX_X,
        );
        let (t1, t2) = TombolaPainter::axis_tex_coord_for_ball(
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
                &mut self.balls_array_object,
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

fn add_claw_vertices(vertices: &mut Vec<TombolaVertex>) {
    let x1 = -CLAW_WIDTH / 2.0;
    let x2 = CLAW_WIDTH / 2.0;
    let y1 = CLAW_HEIGHT;
    let y2 = 0.0;

    vertices.push(TombolaVertex {
        x: x1,
        y: y1,
        s: 0,
        t: 0,
    });
    vertices.push(TombolaVertex {
        x: x1,
        y: y2,
        s: 0,
        t: 65535,
    });
    vertices.push(TombolaVertex {
        x: x2,
        y: y1,
        s: 65535,
        t: 0,
    });
    vertices.push(TombolaVertex {
        x: x2,
        y: y2,
        s: 65535,
        t: 65535,
    });
}

fn add_wall_vertices(vertices: &mut Vec<TombolaVertex>) {
    let wall_top = tombola::APOTHEM / (PI / tombola::N_SIDES as f32).cos()
        + tombola::BALL_SIZE * 2.0;

    // Work out the angle of the slope
    let height_diff = tombola::RIGHT_SLOPE_Y - tombola::LEFT_SLOPE_Y;
    let angle = (height_diff / (tombola::WALL_X * 2.0)).atan();

    // Vertical offset from a point on the top line to the bottom line
    let vertical_offset = WALL_WIDTH / angle.cos();

    // The equation of the bottom line of the slope so we can work out
    // where to put the bottom vertices of the corners where it joins
    // the wall.
    let m = height_diff / (tombola::WALL_X * 2.0);
    let c = (tombola::RIGHT_SLOPE_Y + tombola::LEFT_SLOPE_Y) / 2.0
        - vertical_offset;
    let bottom_left_y = m * (-tombola::WALL_X - WALL_WIDTH) + c;
    let bottom_right_y = m * (tombola::WALL_X + WALL_WIDTH) + c;

    vertices.push(TombolaVertex {
        x: tombola::WALL_X + WALL_WIDTH,
        y: wall_top,
        s: 32768,
        t: 65535,
    });
    vertices.push(TombolaVertex {
        x: tombola::WALL_X,
        y: wall_top,
        s: 32768,
        t: 0,
    });
    vertices.push(TombolaVertex {
        x: tombola::WALL_X + WALL_WIDTH,
        y: bottom_right_y,
        s: 32768,
        t: 65535,
    });
    vertices.push(TombolaVertex {
        x: tombola::WALL_X,
        y: tombola::RIGHT_SLOPE_Y,
        s: 32768,
        t: 0,
    });
    vertices.push(TombolaVertex {
        x: -tombola::WALL_X - WALL_WIDTH,
        y: bottom_left_y,
        s: 32768,
        t: 65535,
    });
    vertices.push(TombolaVertex {
        x: -tombola::WALL_X,
        y: tombola::LEFT_SLOPE_Y,
        s: 32768,
        t: 0,
    });
    vertices.push(TombolaVertex {
        x: -tombola::WALL_X - WALL_WIDTH,
        y: tombola::LEFT_SLOPE_Y + tombola::BALL_SIZE,
        s: 32768,
        t: 65535,
    });
    vertices.push(TombolaVertex {
        x: -tombola::WALL_X,
        y: tombola::LEFT_SLOPE_Y + tombola::BALL_SIZE,
        s: 32768,
        t: 0,
    });
}

fn create_tombola_buffer(
    paint_data: &PaintData,
) -> Result<Rc<Buffer>, String> {
    let inner_radius = tombola::APOTHEM / (PI / tombola::N_SIDES as f32).cos();
    let outer_radius = inner_radius + SIDE_WIDTH;
    let mut vertices = Vec::with_capacity(FIRST_CLAW_VERTEX + N_CLAW_VERTICES);

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

    add_claw_vertices(&mut vertices);
    add_wall_vertices(&mut vertices);

    assert_eq!(vertices.len(), FIRST_WALL_VERTEX + N_WALL_VERTICES);

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

fn set_sides_element_buffer(
    paint_data: &PaintData,
    array_object: &mut ArrayObject,
) -> Result<(), String> {
    let mut elements = Vec::with_capacity(N_SIDES_ELEMENTS);

    for side in 0..tombola::N_SIDES as u8 {
        elements.push(side * 2);
        elements.push(side * 2 + 1);
    }

    elements.push(0);
    elements.push(1);

    assert_eq!(elements.len(), N_SIDES_ELEMENTS);

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

fn create_sides_array_object(
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

    set_sides_element_buffer(&paint_data, &mut array_object)?;

    Ok(array_object)
}
