use glow::HasContext;
use std::rc::Rc;
use std::cell::Cell;
use crate::array_object::ArrayObject;
use crate::buffer::Buffer;

pub struct QuadTool {
    gl: Rc<glow::Context>,
    buffer: Cell<Option<(u32, Rc<Buffer>)>>,
}

impl QuadTool {
    pub fn new(gl: Rc<glow::Context>) -> QuadTool {
        QuadTool { gl, buffer: Cell::new(None) }
    }

    pub fn set_element_buffer(
        &self,
        array_object: &mut ArrayObject,
        n_quads: u32
    ) -> Result<u32, String> {
        let mut new_n_quads = if let Some((current_n_quads, buffer)) =
            self.buffer.take()
        {
            if current_n_quads >= n_quads {
                array_object.set_element_buffer(Rc::clone(&buffer));
                self.buffer.replace(Some((current_n_quads, buffer)));
                return Ok(current_n_quads);
            }

            current_n_quads
        } else {
            1
        };

        while new_n_quads < n_quads {
            new_n_quads *= 2;
        }

        let buffer = create_buffer(
            &self.gl,
            array_object,
            new_n_quads
        )?;

        self.buffer.replace(Some((new_n_quads, buffer)));

        Ok(new_n_quads)
    }
}

fn create_buffer(
    gl: &Rc<glow::Context>,
    array_object: &mut ArrayObject,
    n_quads: u32,
) -> Result<Rc<Buffer>, String> {
    let mut indices = Vec::<u16>::with_capacity(n_quads as usize * 6);

    for quad_num in 0..n_quads {
        let base_index = quad_num as u16 * 4;
        indices.push(base_index + 0);
        indices.push(base_index + 1);
        indices.push(base_index + 2);
        indices.push(base_index + 2);
        indices.push(base_index + 1);
        indices.push(base_index + 3);
    }

    let buffer = Rc::new(Buffer::new(Rc::clone(gl))?);

    array_object.set_element_buffer(Rc::clone(&buffer));

    unsafe {
        let buffer_data = std::slice::from_raw_parts(
            indices.as_ptr() as *const u8,
            indices.len() * std::mem::size_of::<u16>(),
        );

        gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            buffer_data,
            glow::STATIC_DRAW,
        );
    }

    Ok(buffer)
}
