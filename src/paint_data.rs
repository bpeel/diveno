use std::rc::Rc;
use std::cell::Cell;
use crate::{shaders, images, quad_tool};

pub struct PaintData {
    pub gl: Rc<glow::Context>,
    pub shaders: shaders::Shaders,
    pub images: images::ImageSet,
    pub quad_tool: quad_tool::QuadTool,
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
        let quad_tool = quad_tool::QuadTool::new(Rc::clone(&gl));

        PaintData {
            gl,
            has_vertex_array_object,
            shaders,
            images,
            quad_tool,
            enabled_attribs: Cell::new(0),
        }
    }
}
