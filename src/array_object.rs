use std::rc::Rc;
use crate::paint_data::PaintData;
use std::collections::HashMap;
use crate::buffer::Buffer;
use glow::HasContext;

struct Attribute {
    size: i32,
    data_type: u32,
    normalized: bool,
    stride: i32,
    buffer: Rc<Buffer>,
    offset: i32,
}

enum InternalArrayObject {
    Legacy {
        attributes: HashMap<u32, Attribute>,
    },
    Native {
        buffers: HashMap<u32, Rc<Buffer>>,
        vertex_array: glow::NativeVertexArray,
    },
}

pub struct ArrayObject {
    paint_data: Rc<PaintData>,
    data: InternalArrayObject,
    element_buffer: Option<Rc<Buffer>>,
}

impl ArrayObject {
    pub fn new(paint_data: Rc<PaintData>) -> Result<ArrayObject, String> {
        let data = if paint_data.has_vertex_array_object {
            let vertex_array = unsafe {
                paint_data.gl.create_vertex_array()?
            };

            InternalArrayObject::Native {
                buffers: HashMap::new(),
                vertex_array,
            }
        } else {
            InternalArrayObject::Legacy {
                attributes: HashMap::new(),
            }
        };

        Ok(ArrayObject {
            paint_data,
            data,
            element_buffer: None,
        })
    }

    pub fn set_attribute(
        &mut self,
        index: u32,
        size: i32,
        data_type: u32,
        normalized: bool,
        stride: i32,
        buffer: Rc<Buffer>,
        offset: i32,
    ) {
        match self.data {
            InternalArrayObject::Legacy { ref mut attributes, .. } => {
                attributes.insert(
                    index,
                    Attribute {
                        size,
                        data_type,
                        normalized,
                        stride,
                        buffer,
                        offset,
                    }
                );
            },
            InternalArrayObject::Native { ref mut buffers, vertex_array } => {
                let gl = &self.paint_data.gl;

                unsafe {
                    gl.bind_vertex_array(Some(vertex_array));
                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer.id()));
                    gl.vertex_attrib_pointer_f32(
                        index,
                        size,
                        data_type,
                        normalized,
                        stride,
                        offset,
                    );
                    gl.enable_vertex_attrib_array(index);
                }

                buffers.insert(index, buffer);
            },
        }
    }

    pub fn set_element_buffer(&mut self, buffer: Rc<Buffer>) {
        let gl = &self.paint_data.gl;

        match self.data {
            InternalArrayObject::Legacy { .. } => (),
            InternalArrayObject::Native { vertex_array, .. } => {
                unsafe {
                    gl.bind_vertex_array(Some(vertex_array));
                }
            },
        }

        // We bind the buffer immediately even if VAOs aren't
        // available so that the callee can assume it's bound and fill
        // it with data.
        unsafe {
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(buffer.id()));
        }

        self.element_buffer = Some(buffer);
    }

    pub fn bind(&self) {
        let gl = &self.paint_data.gl;

        match self.data {
            InternalArrayObject::Legacy { ref attributes, .. } => {
                set_attributes(&self.paint_data, attributes);

                unsafe {
                    gl.bind_buffer(
                        glow::ELEMENT_ARRAY_BUFFER,
                        self.element_buffer.as_ref().map(|b| b.id()),
                    );
                }
            },
            InternalArrayObject::Native { vertex_array, .. } => {
                unsafe {
                    gl.bind_vertex_array(Some(vertex_array));
                }
            },
        }
    }
}

impl Drop for ArrayObject {
    fn drop(&mut self) {
        match self.data {
            InternalArrayObject::Native { vertex_array, .. } => {
                unsafe {
                    self.paint_data.gl.delete_vertex_array(vertex_array);
                }
            },
            InternalArrayObject::Legacy { .. } => (),
        }
    }
}

fn set_attributes(
    paint_data: &PaintData,
    attributes: &HashMap<u32, Attribute>,
) {
    let mut array_attributes = 0;
    let mut last_buffer = None;
    let gl = &paint_data.gl;

    for (&index, attribute) in attributes.iter() {
        if last_buffer.map(|b| b == attribute.buffer.id()).unwrap_or(false) {
            unsafe {
                gl.bind_buffer(
                    glow::ARRAY_BUFFER,
                    Some(attribute.buffer.id()),
                );
            }
            last_buffer = Some(attribute.buffer.id());
        }

        unsafe {
            gl.vertex_attrib_pointer_f32(
                index,
                attribute.size,
                attribute.data_type,
                attribute.normalized,
                attribute.stride,
                attribute.offset,
            );
        }

        array_attributes |= 1 << index;
    }

    let enabled_attributes = paint_data.enabled_attribs.get();

    let mut changed_attributes = enabled_attributes ^ array_attributes;

    while changed_attributes != 0 {
        let index = changed_attributes.trailing_zeros();

        unsafe {
            if array_attributes & (1 << index) == 0 {
                gl.disable_vertex_attrib_array(index);
            } else {
                gl.enable_vertex_attrib_array(index);
            }
        }

        changed_attributes &= !(1 << index);
    }

    paint_data.enabled_attribs.replace(array_attributes);
}
