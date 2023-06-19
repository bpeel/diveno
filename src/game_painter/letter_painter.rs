use std::rc::Rc;
use crate::paint_data::PaintData;
use crate::logic;
use crate::buffer::Buffer;
use crate::letter_texture;
use crate::shaders;
use crate::array_object::ArrayObject;
use glow::HasContext;

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
    // Top-left corner of the grid in clip-space coordinates
    grid_x: f32,
    grid_y: f32,
    // Size of a letter in clip-space coordinates
    tile_w: f32,
    tile_h: f32,
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

        Ok(LetterPainter {
            buffer,
            array_object,
            paint_data,
            width: 1,
            height: 1,
            transform_dirty: true,
            vertices_dirty: true,
            grid_x: 1.0,
            grid_y: 1.0,
            tile_w: 1.0,
            tile_h: 1.0,
            vertices: Vec::new(),
            most_quads: 0,
        })
    }

    pub fn paint(&mut self, logic: &logic::Logic) {
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
        }
    }

    fn update_transform(&mut self) {
        let smallest_axis = std::cmp::min(self.width, self.height);
        let tile_size_pixels = smallest_axis as f32 / 10.0;

        self.tile_w = tile_size_pixels * 2.0 / self.width as f32;
        self.tile_h = tile_size_pixels * 2.0 / self.height as f32;
        self.grid_x = -self.tile_w * 3.0;
        self.grid_y = self.tile_h * 3.0;

        self.vertices_dirty = true;
    }

    fn update_vertices(&mut self, logic: &logic::Logic) {
        self.vertices.clear();

        for (pos, ch) in logic.in_progress_guess().chars().enumerate() {
            self.add_letter(0, pos as u32, 0, ch);
        }

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

        let x = self.grid_x + x as f32 * self.tile_w;
        let y = self.grid_y - y as f32 * self.tile_h;

        self.vertices.push(Vertex {
            x,
            y,
            s: letter.s1,
            t: letter.t1,
        });
        self.vertices.push(Vertex {
            x,
            y: y - self.tile_h,
            s: letter.s1,
            t: letter.t2,
        });
        self.vertices.push(Vertex {
            x: x + self.tile_w,
            y,
            s: letter.s2,
            t: letter.t1,
        });
        self.vertices.push(Vertex {
            x: x + self.tile_w,
            y: y - self.tile_h,
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
