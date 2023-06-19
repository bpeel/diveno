use std::rc::Rc;
use std::cell::Cell;
use crate::{shaders, images};

pub struct PaintData {
    pub gl: Rc<glow::Context>,
    pub shaders: shaders::Shaders,
    pub images: images::ImageSet,
    pub has_vertex_array_object: bool,
    pub enabled_attribs: Cell<u32>,
}

impl PaintData {
    pub fn new(
        gl: Rc<glow::Context>,
        has_vertex_array_object: bool,
        shaders: shaders::Shaders,
        images: images::ImageSet,
    ) -> PaintData {
        PaintData {
            gl,
            has_vertex_array_object,
            shaders,
            images,
            enabled_attribs: Cell::new(0),
        }
    }
}
