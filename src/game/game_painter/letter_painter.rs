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
use super::super::{shaders, logic, timer, letter_texture};
use letter_texture::LETTERS;
use super::super::array_object::ArrayObject;
use glow::HasContext;
use nalgebra::{Vector3, Perspective3};
use std::f32::consts::PI;

// Number of milliseconds per letter for the animation
use super::super::timing::MILLIS_PER_LETTER;
// Time for a letter to turn
const TURN_TIME: i64 = 500;

// How long to shake the guess for when a wrong guess is entered
const SHAKE_TIME: i64 = 500;
// The frequency of the shaking in shakes per second
const SHAKE_FREQUENCY: i64 = 20;
// Distance to move the tile while shaking where 1 is the size of a tile
const SHAKE_DISTANCE: f32 = 0.1;

// The time for one tile to raise and lower itself again
const WAVE_LIFT_TIME: i64 = 300;
// The time between starting each tile
const WAVE_LIFT_DELAY: i64 = 100;
// The amount that a tile should rise up
const WAVE_LIFT_DISTANCE: f32 = 0.2;

// Time to wait after the last guess reveal animation before revealing
// the answer
const ANSWER_DELAY: i64 = 1000;

const EMPTY_COLOR: [u8; 3] = [0; 3];

#[repr(C)]
struct Vertex {
    x: f32,
    y: f32,
    s: u16,
    t: u16,
    // Vertical Rotation centre
    ry: f32,
    // Rotation progress
    rp: f32,
    // Color of the background of the tile
    color: [u8; 3],
}

struct AnimationTimes {
    reveal_time: Option<i64>,
    shake_time: Option<i64>,
    wave_time: Option<i64>,
    answer_time: Option<i64>,
}

impl AnimationTimes {
    fn is_animating(&self) -> bool {
        self.reveal_time.is_some()
            || self.shake_time.is_some()
            || self.wave_time.is_some()
            || self.answer_time.is_some()
    }
}

pub struct LetterPainter {
    buffer: Rc<Buffer>,
    array_object: ArrayObject,
    paint_data: Rc<PaintData>,
    width: u32,
    height: u32,
    transform_dirty: bool,
    vertices_dirty: bool,
    mvp_uniform: glow::UniformLocation,
    // Temporary buffer used for building the vertex buffer
    vertices: Vec<Vertex>,
    // Used to keep track of whether we need to create a new quad buffer
    most_quads: u32,
    reveal_start_time: Option<timer::Timer>,
    shake_start_time: Option<timer::Timer>,
    wave_start_time: Option<timer::Timer>,
    answer_start_time: Option<timer::Timer>,
}

impl LetterPainter {
    pub fn new(paint_data: Rc<PaintData>) -> Result<LetterPainter, String> {
        let buffer = create_letter_buffer(&paint_data)?;
        let array_object = create_array_object(
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

        Ok(LetterPainter {
            buffer,
            array_object,
            paint_data,
            width: 1,
            height: 1,
            transform_dirty: true,
            vertices_dirty: true,
            mvp_uniform,
            vertices: Vec::new(),
            most_quads: 0,
            reveal_start_time: None,
            shake_start_time: None,
            wave_start_time: None,
            answer_start_time: None,
        })
    }

    fn update_animation_times(
        &mut self,
        logic: &logic::Logic,
    ) -> AnimationTimes {
        let total_reveal_time =
            (logic.word_length() as i64 - 1)
            * MILLIS_PER_LETTER
            + TURN_TIME;

        let reveal_time = self.reveal_start_time.and_then(|start_time| {
            let millis = start_time.elapsed();

            if millis < total_reveal_time {
                Some(millis)
            } else {
                self.reveal_start_time = None;
                None
            }
        });

        let shake_time = self.shake_start_time.and_then(|start_time| {
            let millis = start_time.elapsed();

            if millis < SHAKE_TIME {
                Some(millis)
            } else {
                self.shake_start_time = None;
                None
            }
        });

        let wave_time = self.wave_start_time.and_then(|start_time| {
            let millis = start_time.elapsed();

            let total_wave_time =
                (logic.word_length() as i64 - 1)
                * WAVE_LIFT_DELAY
                + WAVE_LIFT_TIME;

            if millis < total_reveal_time + total_wave_time {
                Some(millis - total_reveal_time)
            } else {
                self.wave_start_time = None;
                None
            }
        });

        let answer_time = self.answer_start_time.and_then(|start_time| {
            let millis = start_time.elapsed();

            if millis < total_reveal_time * 2 + ANSWER_DELAY {
                Some(millis - total_reveal_time - ANSWER_DELAY)
            } else {
                self.answer_start_time = None;
                None
            }
        });

        AnimationTimes {
            reveal_time,
            shake_time,
            wave_time,
            answer_time,
        }
    }

    pub fn paint(&mut self, logic: &logic::Logic) -> bool {
        let animation_times = self.update_animation_times(logic);

        if self.transform_dirty {
            self.update_transform(logic);
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
                Some(self.paint_data.images.letters.id()),
            );

            gl.use_program(Some(self.paint_data.shaders.letter.id()));

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

        if animation_times.is_animating() {
            self.vertices_dirty = true;
            true
        } else {
            false
        }
    }

    pub fn update_fb_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.transform_dirty = true;
    }

    pub fn handle_logic_event(
        &mut self,
        logic: &logic::Logic,
        event: &logic::Event,
    ) -> bool {
        match event {
            logic::Event::WordChanged => {
                self.vertices_dirty = true;
                self.transform_dirty = true;
                true
            },
            logic::Event::GridChanged => {
                self.vertices_dirty = true;
                true
            },
            logic::Event::GuessEntered => {
                self.reveal_start_time = Some(timer::Timer::new());

                if logic.n_guesses() >= logic::N_GUESSES && !logic.is_solved() {
                    self.answer_start_time = self.reveal_start_time;
                }

                self.vertices_dirty = true;

                true
            },
            logic::Event::WrongGuessEntered => {
                self.shake_start_time = Some(timer::Timer::new());
                self.vertices_dirty = true;
                true
            },
            logic::Event::Solved => {
                self.wave_start_time = Some(timer::Timer::new());
                self.vertices_dirty = true;
                true
            },
            logic::Event::ScoreChanged(_) => false,
        }
    }

    fn update_transform(&mut self, logic: &logic::Logic) {
        // We want to fit ten tiles along either the height or half of
        // the width, whichever is smaller. We only use half of the
        // width so that there will be a quarter of the space on
        // either side in order to draw the scores.
        let smallest_axis = (self.width / 2).clamp(1, self.height);
        const TILE_SIZE: f32 = 2.0 / 10.0;
        const FOV: f32 = PI / 4.0;

        let y_top = self.height as f32 / smallest_axis as f32;

        // The distance to the point where the y coordinate top of the
        // frustrum is y_top for the chosen field of view. This
        // will be the zero z-coordinate after the translation
        let zero_distance = y_top / (FOV / 2.0).tan();

        let perspective = Perspective3::new(
            self.width as f32 / self.height as f32,
            FOV,
            zero_distance - TILE_SIZE * 2.0,
            zero_distance + TILE_SIZE * 2.0,
        );

        let matrix = perspective
            .as_matrix()
            .prepend_translation(&Vector3::new(0.0, 0.0, -zero_distance))
            .prepend_nonuniform_scaling(&Vector3::new(
                TILE_SIZE,
                -TILE_SIZE,
                TILE_SIZE,
            ))
            .prepend_translation(&Vector3::new(
                -(logic.word_length() as f32) / 2.0,
                -(logic::N_GUESSES as f32) / 2.0,
                0.0,
            ));

        let gl = &self.paint_data.gl;

        unsafe {
            gl.use_program(Some(self.paint_data.shaders.letter.id()));
            gl.uniform_matrix_4_f32_slice(
                Some(&self.mvp_uniform),
                false, // transpose
                matrix.as_slice(),
            );
        }

        self.vertices_dirty = true;
    }

    fn fill_vertices_array(
        &mut self,
        logic: &logic::Logic,
        animation_times: &AnimationTimes,
    ) {
        self.vertices.clear();

        let mut guess_num = 0;

        for guess in logic.guesses() {
            if animation_times.reveal_time.is_some()
                && guess_num >= logic.n_guesses() - 1
            {
                self.add_animated_guess(
                    guess,
                    guess_num as u32,
                    animation_times.reveal_time.unwrap()
                );
            } else {
                self.add_guess(
                    guess,
                    guess_num as u32,
                    animation_times.wave_time
                );
            }

            guess_num += 1;
        }

        if guess_num < logic::N_GUESSES {
            if !logic.is_finished() {
                let visible_letters = if animation_times.reveal_time.is_some() {
                    0
                } else {
                    logic.visible_letters()
                };

                self.add_in_progress_guess(
                    logic,
                    guess_num as u32,
                    visible_letters,
                    animation_times.shake_time,
                );

                guess_num += 1;
            }

            for x in 0..logic.word_length() {
                let wave_offset = wave_offset_for_column(
                    animation_times.wave_time,
                    x
                );

                for y in guess_num..logic::N_GUESSES {
                    self.add_letter(
                        EMPTY_COLOR,
                        x as f32,
                        y as f32 + wave_offset,
                        ' '
                    );
                }
            }
        } else if !logic.is_solved() {
            self.add_answer(logic, animation_times.answer_time);
        }
    }

    fn color_for_result(result: logic::LetterResult) -> [u8; 3] {
        match result {
            logic::LetterResult::Correct => [0xe7, 0x00, 0x2a],
            logic::LetterResult::WrongPosition => [0xff, 0xbd, 0x00],
            logic::LetterResult::Wrong => [0x00, 0x77, 0xc7],
        }
    }

    fn add_guess(
        &mut self,
        guess: &[logic::Letter],
        y: u32,
        wave_time: Option<i64>,
    ) {
        for (x, letter) in guess.iter().enumerate() {
            let wave_offset = wave_offset_for_column(wave_time, x);

            self.add_letter(
                LetterPainter::color_for_result(letter.result),
                x as f32,
                y as f32 + wave_offset,
                letter.letter
            );
        }
    }

    fn add_animated_guess(
        &mut self,
        guess: &[logic::Letter],
        y: u32,
        animation_time: i64,
    ) {
        for (x, letter) in guess.iter().enumerate() {
            let rotation_progress =
                ((animation_time - MILLIS_PER_LETTER * x as i64) as f32
                 / TURN_TIME as f32)
                .clamp(0.0, 1.0);

            self.add_rotated_letter(
                EMPTY_COLOR,
                x as f32,
                y as f32,
                rotation_progress,
                letter.letter,
            );
            self.add_rotated_letter(
                LetterPainter::color_for_result(letter.result),
                x as f32,
                y as f32,
                rotation_progress + 1.0,
                letter.letter,
            );
        }
    }

    fn add_in_progress_guess(
        &mut self,
        logic: &logic::Logic,
        y: u32,
        visible_letters: u32,
        shake_time: Option<i64>,
    ) {
        let mut added = 0;

        let shake_offset = shake_time
            .map(|t| {
                (((t * SHAKE_FREQUENCY / 1000) & 1) * 2 - 1) as f32
                    * SHAKE_DISTANCE
            })
            .unwrap_or(0.0);

        for (pos, ch) in logic.in_progress_guess().chars().enumerate() {
            self.add_letter(
                EMPTY_COLOR,
                pos as f32 + shake_offset,
                y as f32,
                ch
            );
            added += 1;
        }

        if added == 0 {
            for (index, ch) in logic.word().chars().enumerate() {
                let ch = if visible_letters & (1 << index) != 0 {
                    ch
                } else {
                    '.'
                };

                self.add_letter(
                    EMPTY_COLOR,
                    index as f32 + shake_offset,
                    y as f32,
                    ch
                );
            }
        } else {
            for x in added..logic.word_length() {
                self.add_letter(
                    EMPTY_COLOR,
                    x as f32 + shake_offset,
                    y as f32,
                    '.'
                );
            }
        }
    }

    fn add_answer(&mut self, logic: &logic::Logic, answer_time: Option<i64>) {
        match answer_time {
            Some(answer_time) => {
                if answer_time < 0 {
                    return;
                }

                for (x, letter) in logic.word().chars().enumerate() {
                    let rotation_progress =
                        1.0
                        - ((answer_time - MILLIS_PER_LETTER * x as i64) as f32
                           / TURN_TIME as f32).clamp(0.0, 1.0);

                    self.add_rotated_letter(
                        EMPTY_COLOR,
                        x as f32,
                        logic::N_GUESSES as f32,
                        rotation_progress,
                        letter
                    );
                }
            },
            None => {
                for (x, letter) in logic.word().chars().enumerate() {
                    self.add_letter(
                        EMPTY_COLOR,
                        x as f32,
                        logic::N_GUESSES as f32,
                        letter
                    );
                }
            },
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

    fn add_rotated_letter(
        &mut self,
        color: [u8; 3],
        x: f32,
        y: f32,
        rotation_progress: f32,
        letter: char
    ) {
        let Ok(letter_index) = LETTERS.binary_search_by(|probe| {
            probe.ch.cmp(&letter)
        })
        else {
            return;
        };

        let letter = &LETTERS[letter_index];

        self.vertices.push(Vertex {
            x,
            y,
            s: letter.s1,
            t: letter.t1,
            ry: y + 0.5,
            rp: rotation_progress,
            color,
        });
        self.vertices.push(Vertex {
            x,
            y: y + 1.0,
            s: letter.s1,
            t: letter.t2,
            ry: y + 0.5,
            rp: rotation_progress,
            color,
        });
        self.vertices.push(Vertex {
            x: x + 1.0,
            y,
            s: letter.s2,
            t: letter.t1,
            ry: y + 0.5,
            rp: rotation_progress,
            color,
        });
        self.vertices.push(Vertex {
            x: x + 1.0,
            y: y + 1.0,
            s: letter.s2,
            t: letter.t2,
            ry: y + 0.5,
            rp: rotation_progress,
            color,
        });
    }

    fn add_letter(
        &mut self,
        color: [u8; 3],
        x: f32,
        y: f32,
        letter: char
    ) {
        self.add_rotated_letter(color, x, y, 0.0, letter);
    }
}

fn create_array_object(
    paint_data: Rc<PaintData>,
    buffer: Rc<Buffer>,
) -> Result<ArrayObject, String> {
    let rotation_attrib = unsafe {
        match paint_data.gl.get_attrib_location(
            paint_data.shaders.letter.id(),
            "rotation",
        ) {
            Some(l) => l,
            None => return Err("Missing “rotation” attrib".to_string()),
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
        2, // size
        glow::FLOAT,
        false, // normalized
        std::mem::size_of::<Vertex>() as i32,
        Rc::clone(&buffer),
        offset,
    );
    offset += std::mem::size_of::<f32>() as i32 * 2;

    array_object.set_attribute(
        shaders::COLOR_ATTRIB,
        3, // size
        glow::UNSIGNED_BYTE,
        true, // normalized
        std::mem::size_of::<Vertex>() as i32,
        buffer,
        offset,
    );

    Ok(array_object)
}

fn create_letter_buffer(paint_data: &PaintData) -> Result<Rc<Buffer>, String> {
    let buffer = Buffer::new(Rc::clone(&paint_data.gl))?;

    Ok(Rc::new(buffer))
}

fn wave_offset_for_column(wave_time: Option<i64>, x: usize) -> f32 {
    wave_time.map(|wave_time| {
        let t = ((wave_time - x as i64 * WAVE_LIFT_DELAY) as f32
                 / WAVE_LIFT_TIME as f32)
            .clamp(0.0, 1.0);

        (t * PI).sin() * -WAVE_LIFT_DISTANCE
    }).unwrap_or(0.0)
}
