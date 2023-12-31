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
use super::super::{logic, bingo_grid, timer, timeout};
use super::super::array_object::ArrayObject;
use glow::HasContext;
use nalgebra::{Matrix4, Vector3};
use super::letter_vertex;
use letter_vertex::Vertex;
use timeout::Timeout;

const TEX_SPACES_X: u32 = 8;
const TEX_SPACES_Y: u32 = 4;
// Size of the border around a space as a fraction of the total space
// allocated to a space
const BORDER_SIZE: f32 = 0.1;

// Time in milliseconds to flash the space when it is covered
const FLASH_TIME: i64 = 1000;
// Number of flashes per second
const FLASHES_PER_SECOND: i64 = 4;

// Total time for the bingo animation
const BINGO_TIME: i64 = 3000;
// Total time to reveal the BINGO letters
const BINGO_LETTER_TIME: i64 = BINGO_TIME / 2;

const COVERED_COLOR: [u8; 3] = [0xe7, 0x00, 0x2a];
const UNCOVERED_COLOR: [u8; 3] = [0x00, 0x77, 0xc7];

struct Flash {
    start_time: timer::Timer,
    space: usize,
}

struct FlashResult {
    space: usize,
    covered: bool,
}

struct AnimationTimes {
    flash: Option<FlashResult>,
    bingo_time: Option<i64>,
}

impl AnimationTimes {
    fn is_animating(&self) -> bool {
        self.flash.is_some() || self.bingo_time.is_some()
    }
}

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
    flash: Option<Flash>,
    bingo_start_time: Option<timer::Timer>,
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
            flash: None,
            bingo_start_time: None,
        })
    }

    fn update_animation_times(&mut self) -> AnimationTimes {
        let flash = if let Some(flash) = self.flash.as_ref() {
            let elapsed = flash.start_time.elapsed();

            if elapsed >= FLASH_TIME {
                self.flash = None;
                None
            } else {
                Some(FlashResult {
                    space: flash.space,
                    covered: (elapsed * FLASHES_PER_SECOND / 1000) & 1 == 0,
                })
            }
        } else {
            None
        };

        let bingo_time = self.bingo_start_time.and_then(|start_time| {
            let millis = start_time.elapsed();

            if millis < FLASH_TIME + BINGO_TIME {
                Some(millis - FLASH_TIME)
            } else {
                self.bingo_start_time = None;
                None
            }
        });

        AnimationTimes {
            flash,
            bingo_time,
        }
    }

    pub fn paint(&mut self, logic: &logic::Logic) -> Timeout {
        let animation_times = self.update_animation_times();

        if self.transform_dirty {
            self.update_transform();
            self.transform_dirty = false;
        }

        if self.vertices_dirty {
            self.update_vertices(logic, &animation_times);
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

        if animation_times.is_animating() {
            self.vertices_dirty = true;
            timeout::IMMEDIATELY
        } else {
            Timeout::Forever
        }
    }

    pub fn update_fb_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.transform_dirty = true;
    }

    pub fn handle_logic_event(
        &mut self,
        _logic: &logic::Logic,
        event: &logic::Event,
    ) -> bool {
        match event {
            logic::Event::BingoChanged(team, space) => {
                if *team == self.team {
                    self.flash = Some(Flash {
                        start_time: timer::Timer::new(),
                        space: *space,
                    });
                    self.vertices_dirty = true;
                    true
                } else {
                    false
                }
            },
            logic::Event::Bingo(team, _) => {
                if *team == self.team {
                    self.bingo_start_time = Some(timer::Timer::new());
                    self.vertices_dirty = true;
                    true
                } else {
                    false
                }
            },
            logic::Event::BingoReset(team) => {
                if *team == self.team {
                    self.vertices_dirty = true;
                    true
                } else {
                    false
                }
            },
            logic::Event::TombolaStartedSpinning(_) => false,
            logic::Event::WordChanged => false,
            logic::Event::GridChanged => false,
            logic::Event::GuessEntered => false,
            logic::Event::WrongGuessEntered => false,
            logic::Event::GuessRejected => false,
            logic::Event::Solved => false,
            logic::Event::ScoreChanged(_) => false,
            logic::Event::CurrentTeamChanged => false,
            logic::Event::CurrentPageChanged(_) => false,
            logic::Event::SuperDivenoToggled => false,
            logic::Event::SuperDivenoPauseToggled => false,
        }
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

    fn bingo_index(
        index: usize,
        bingo: Option<bingo_grid::Bingo>,
        animation_times: &AnimationTimes
    ) -> Option<u32> {
        let Some(bingo) = bingo
        else {
            return None;
        };

        let Some(index) = bingo.letter_index_for_space(index as u8)
        else {
            return None;
        };

        match animation_times.bingo_time {
            Some(bingo_time) => {
                if bingo_time >= 0
                    && bingo_time
                    * bingo_grid::GRID_WIDTH as i64
                    / BINGO_LETTER_TIME
                    >= index as i64
                {
                    Some(index as u32)
                } else {
                    None
                }
            },
            None => Some(index as u32),
        }
    }

    fn rainbow_color(index: u8, bingo_time: i64) -> [u8; 3] {
        let rainbow_end = bingo_time as f64
            * bingo_grid::GRID_WIDTH as f64
            / BINGO_TIME as f64
            * 2.0;

        let index = index as f64;

        if index < rainbow_end - bingo_grid::GRID_WIDTH as f64
            || index >= rainbow_end
        {
            return COVERED_COLOR;
        }

        let hsv = color_space::Hsv::new(
            (rainbow_end - index)
                / bingo_grid::GRID_WIDTH as f64
                * 360.0,
            1.0,
            1.0,
        );
        let rgb = color_space::Rgb::from(hsv);

        [
            rgb.r.round() as u8,
            rgb.g.round() as u8,
            rgb.b.round() as u8
        ]
    }

    fn square_color(
        index: usize,
        covered: bool,
        animation_times: &AnimationTimes,
        bingo: Option<bingo_grid::Bingo>,
    ) -> [u8; 3] {
        if let Some(index) = bingo.and_then(|b| {
            b.letter_index_for_space(index as u8)
        }) {
            match animation_times.bingo_time {
                Some(bingo_time) => {
                    if bingo_time >= 0 {
                        return BingoPainter::rainbow_color(index, bingo_time);
                    }
                },
                None => return COVERED_COLOR,
            };
        }

        let covered = match animation_times.flash.as_ref() {
            Some(flash) => if flash.space as usize == index {
                flash.covered
            } else {
                covered
            },
            None => covered,
        };

        if covered {
            COVERED_COLOR
        } else {
            UNCOVERED_COLOR
        }
    }

    fn fill_vertices_array(
        &mut self,
        logic: &logic::Logic,
        animation_times: &AnimationTimes,
    ) {
        self.vertices.clear();

        let bingo_grid = logic.bingo_grid(self.team);
        let bingo = bingo_grid.bingo();

        for (index, space) in bingo_grid.spaces().enumerate() {
            let x = (index % bingo_grid::GRID_WIDTH) as f32;
            let y = (index / bingo_grid::GRID_WIDTH) as f32;
            let x1 = x + BORDER_SIZE;
            let y1 = y + BORDER_SIZE;
            let x2 = x + 1.0 - BORDER_SIZE;
            let y2 = y + 1.0 - BORDER_SIZE;

            let image_index = match BingoPainter::bingo_index(
                index,
                bingo,
                animation_times
            ) {
                None => space.ball as u32,
                Some(index) => {
                    TEX_SPACES_X * TEX_SPACES_Y
                        - bingo_grid::GRID_WIDTH as u32
                        + index as u32
                },
            };

            let tex_x = image_index % TEX_SPACES_X;
            let tex_y = image_index / TEX_SPACES_X;

            let s1 = (tex_x * 65535 / TEX_SPACES_X) as u16;
            let t1 = (tex_y * 65535 / TEX_SPACES_Y) as u16;
            let s2 = ((tex_x + 1) * 65535 / TEX_SPACES_X) as u16;
            let t2 = ((tex_y + 1) * 65535 / TEX_SPACES_Y) as u16;

            let color = BingoPainter::square_color(
                index,
                space.covered,
                animation_times,
                bingo,
            );

            self.vertices.push(Vertex {
                x: x1,
                y: y1,
                s: s1,
                t: t1,
                ry: 0.0,
                rp: 0.0,
                color,
            });
            self.vertices.push(Vertex {
                x: x1,
                y: y2,
                s: s1,
                t: t2,
                ry: 0.0,
                rp: 0.0,
                color,
            });
            self.vertices.push(Vertex {
                x: x2,
                y: y1,
                s: s2,
                t: t1,
                ry: 0.0,
                rp: 0.0,
                color,
            });
            self.vertices.push(Vertex {
                x: x2,
                y: y2,
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
        animation_times: &AnimationTimes,
    ) {
        self.fill_vertices_array(logic, animation_times);

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
