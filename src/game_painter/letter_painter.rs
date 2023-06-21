use std::rc::Rc;
use crate::paint_data::PaintData;
use crate::logic;
use crate::buffer::Buffer;
use crate::letter_texture;
use crate::shaders;
use crate::array_object::ArrayObject;
use glow::HasContext;
use nalgebra::{Vector3, Perspective3};
use std::f32::consts::PI;
use std::time::Instant;

// Number of seconds per letter for the animation
const SECONDS_PER_LETTER: f32 = 0.3;
// Time for a letter to turn
const TURN_TIME: f32 = 0.5;

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
    animation_start_time: Option<Instant>,
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
            animation_start_time: None,
        })
    }

    pub fn paint(&mut self, logic: &logic::Logic) -> bool {
        let animation_time = self.animation_start_time.and_then(|start_time| {
            let secs = start_time.elapsed().as_millis() as f32 / 1000.0;

            let total_time = (logic.word_length() as f32 - 1.0)
                * SECONDS_PER_LETTER
                + TURN_TIME;

            if secs < total_time {
                Some(secs)
            } else {
                self.animation_start_time = None;
                None
            }
        });

        if self.transform_dirty {
            self.update_transform(logic);
            self.transform_dirty = false;
        }

        if self.vertices_dirty {
            self.update_vertices(logic, animation_time);
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

        if animation_time.is_some() {
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

    pub fn handle_logic_event(&mut self, event: &logic::Event) {
        match event {
            logic::Event::WordChanged => {
                self.vertices_dirty = true;
                self.transform_dirty = true;
            },
            logic::Event::GridChanged => self.vertices_dirty = true,
            logic::Event::GuessEntered => {
                self.animation_start_time = Some(Instant::now());
                self.vertices_dirty = true;
            },
        }
    }

    fn update_transform(&mut self, logic: &logic::Logic) {
        let smallest_axis = std::cmp::min(self.width, self.height);
        // Ten tiles should fit along the smallest axis
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
        animation_time: Option<f32>,
    ) {
        self.vertices.clear();

        let mut guess_num = 0;

        for guess in logic.guesses() {
            if animation_time.is_some()
                && guess_num >= logic.n_guesses() - 1
            {
                self.add_animated_guess(
                    guess,
                    guess_num as u32,
                    animation_time.unwrap()
                );
            } else {
                self.add_guess(guess, guess_num as u32);
            }

            guess_num += 1;
        }

        if guess_num < logic::N_GUESSES {
            let visible_letters = if animation_time.is_some() {
                0
            } else {
                logic.visible_letters()
            };

            self.add_in_progress_guess(
                logic,
                guess_num as u32,
                visible_letters,
            );

            for y in guess_num + 1..logic::N_GUESSES {
                for x in 0..logic.word_length() {
                    self.add_letter(0, x as u32, y as u32, ' ');
                }
            }
        }
    }

    fn add_guess(
        &mut self,
        guess: &[logic::Letter],
        y: u32,
    ) {
        for (x, letter) in guess.iter().enumerate() {
            let color = match letter.result {
                logic::LetterResult::Correct => 2,
                logic::LetterResult::WrongPosition => 3,
                logic::LetterResult::Wrong => 1,
            };

            self.add_letter(
                color,
                x as u32,
                y,
                letter.letter
            );
        }
    }

    fn add_animated_guess(
        &mut self,
        guess: &[logic::Letter],
        y: u32,
        animation_time: f32,
    ) {
        for (x, letter) in guess.iter().enumerate() {
            let color = match letter.result {
                logic::LetterResult::Correct => 2,
                logic::LetterResult::WrongPosition => 3,
                logic::LetterResult::Wrong => 1,
            };

            let rotation_progress =
                ((animation_time - SECONDS_PER_LETTER * x as f32)
                 / TURN_TIME)
                .clamp(0.0, 1.0);

            self.add_rotated_letter(
                0,
                x as u32,
                y,
                rotation_progress,
                letter.letter,
            );
            self.add_rotated_letter(
                color,
                x as u32,
                y,
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
    ) {
        let mut added = 0;

        for (pos, ch) in logic.in_progress_guess().chars().enumerate() {
            self.add_letter(0, pos as u32, y, ch);
            added += 1;
        }

        if added == 0 {
            for (index, ch) in logic.word().chars().enumerate() {
                let ch = if visible_letters & (1 << index) != 0 {
                    ch
                } else {
                    '.'
                };

                self.add_letter(0, index as u32, y, ch);
            }
        } else {
            for x in added..logic.word_length() {
                self.add_letter(0, x as u32, y, '.');
            }
        }
    }

    fn update_vertices(
        &mut self,
        logic: &logic::Logic,
        animation_time: Option<f32>,
    ) {
        self.fill_vertices_array(logic, animation_time);

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
        color: usize,
        x: u32,
        y: u32,
        rotation_progress: f32,
        letter: char
    ) {
        let letters = &letter_texture::COLORS[color].letters;

        let Ok(letter_index) = letters.binary_search_by(|probe| {
            probe.ch.cmp(&letter)
        })
        else {
            return;
        };

        let letter = &letters[letter_index];

        let x = x as f32;
        let y = y as f32;

        self.vertices.push(Vertex {
            x,
            y,
            s: letter.s1,
            t: letter.t1,
            ry: y + 0.5,
            rp: rotation_progress,
        });
        self.vertices.push(Vertex {
            x,
            y: y + 1.0,
            s: letter.s1,
            t: letter.t2,
            ry: y + 0.5,
            rp: rotation_progress,
        });
        self.vertices.push(Vertex {
            x: x + 1.0,
            y,
            s: letter.s2,
            t: letter.t1,
            ry: y + 0.5,
            rp: rotation_progress,
        });
        self.vertices.push(Vertex {
            x: x + 1.0,
            y: y + 1.0,
            s: letter.s2,
            t: letter.t2,
            ry: y + 0.5,
            rp: rotation_progress,
        });
    }

    fn add_letter(
        &mut self,
        color: usize,
        x: u32,
        y: u32,
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

    array_object.set_attribute(
        shaders::POSITION_ATTRIB,
        2, // size
        glow::FLOAT,
        false, // normalized
        std::mem::size_of::<Vertex>() as i32,
        Rc::clone(&buffer),
        0, // offset
    );

    array_object.set_attribute(
        shaders::TEX_COORD_ATTRIB,
        2, // size
        glow::UNSIGNED_SHORT,
        true, // normalized
        std::mem::size_of::<Vertex>() as i32,
        Rc::clone(&buffer),
        std::mem::size_of::<f32>() as i32 * 2,
    );

    array_object.set_attribute(
        rotation_attrib,
        2, // size
        glow::FLOAT,
        false, // normalized
        std::mem::size_of::<Vertex>() as i32,
        buffer,
        std::mem::size_of::<f32>() as i32 * 2
            + std::mem::size_of::<u16>() as i32 * 2,
    );

    Ok(array_object)
}

fn create_letter_buffer(paint_data: &PaintData) -> Result<Rc<Buffer>, String> {
    let buffer = Buffer::new(Rc::clone(&paint_data.gl))?;

    Ok(Rc::new(buffer))
}
