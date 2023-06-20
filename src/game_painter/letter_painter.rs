use std::rc::Rc;
use crate::paint_data::PaintData;
use crate::logic;
use crate::buffer::Buffer;
use crate::letter_texture;
use crate::shaders;
use crate::array_object::ArrayObject;
use glow::HasContext;
use nalgebra::{Matrix4, Vector3};

#[repr(C)]
struct Vertex {
    x: f32,
    y: f32,
    s: u16,
    t: u16,
}

pub struct LetterPainter {
    buffer: Rc<Buffer>,
    array_object: ArrayObject,
    paint_data: Rc<PaintData>,
    width: u32,
    height: u32,
    transform_dirty: bool,
    vertices_dirty: bool,
    mvp_uniform: glow::NativeUniformLocation,
    // Temporary buffer used for building the vertex buffer
    vertices: Vec<Vertex>,
    // Used to keep track of whether we need to create a new quad buffer
    most_quads: u32,
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
        })
    }

    pub fn paint(&mut self, logic: &logic::Logic) {
        if self.transform_dirty {
            self.update_transform(logic);
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
            logic::Event::GuessEntered => self.vertices_dirty = true,
        }
    }

    fn update_transform(&mut self, logic: &logic::Logic) {
        let smallest_axis = std::cmp::min(self.width, self.height);
        let tile_size_pixels = smallest_axis as f32 / 10.0;

        let mut matrix = Matrix4::new_nonuniform_scaling(&Vector3::new(
            tile_size_pixels * 2.0 / self.width as f32,
            -tile_size_pixels * 2.0 / self.height as f32,
            1.0,
        ));
        matrix.prepend_translation_mut(&Vector3::new(
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

    fn fill_vertices_array(&mut self, logic: &logic::Logic) {
        self.vertices.clear();

        let mut guess_num = 0;

        for guess in logic.guesses() {
            for (x, letter) in guess.iter().enumerate() {
                let color = match letter.result {
                    logic::LetterResult::Correct => 2,
                    logic::LetterResult::WrongPosition => 3,
                    logic::LetterResult::Wrong => 1,
                };

                self.add_letter(
                    color,
                    x as u32,
                    guess_num as u32,
                    letter.letter
                );
            }

            guess_num += 1;
        }

        if guess_num < logic::N_GUESSES {
            self.add_in_progress_guess(logic, guess_num as u32);

            for y in guess_num + 1..logic::N_GUESSES {
                for x in 0..logic.word_length() {
                    self.add_letter(0, x as u32, y as u32, ' ');
                }
            }
        }
    }

    fn add_in_progress_guess(&mut self, logic: &logic::Logic, y: u32) {
        let mut added = 0;

        for (pos, ch) in logic.in_progress_guess().chars().enumerate() {
            self.add_letter(0, pos as u32, y, ch);
            added += 1;
        }

        if added == 0 {
            let visible_letters = logic.visible_letters();

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

    fn update_vertices(&mut self, logic: &logic::Logic) {
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

    fn add_letter(&mut self, color: usize, x: u32, y: u32, letter: char) {
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
        });
        self.vertices.push(Vertex {
            x,
            y: y + 1.0,
            s: letter.s1,
            t: letter.t2,
        });
        self.vertices.push(Vertex {
            x: x + 1.0,
            y,
            s: letter.s2,
            t: letter.t1,
        });
        self.vertices.push(Vertex {
            x: x + 1.0,
            y: y + 1.0,
            s: letter.s2,
            t: letter.t2,
        });
    }
}

fn create_array_object(
    paint_data: Rc<PaintData>,
    buffer: Rc<Buffer>,
) -> Result<ArrayObject, String> {
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
        buffer,
        std::mem::size_of::<f32>() as i32 * 2,
    );

    Ok(array_object)
}

fn create_letter_buffer(paint_data: &PaintData) -> Result<Rc<Buffer>, String> {
    let buffer = Buffer::new(Rc::clone(&paint_data.gl))?;

    Ok(Rc::new(buffer))
}
