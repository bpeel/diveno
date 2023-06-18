use std::rc::Rc;
use crate::{shaders, images};

pub struct PaintData {
    pub gl: Rc<glow::Context>,
    pub shaders: shaders::Shaders,
    pub images: images::ImageSet,
}

impl PaintData {
    pub fn new(
        gl: Rc<glow::Context>,
        shaders: shaders::Shaders,
        images: images::ImageSet,
    ) -> PaintData {
        PaintData {
            gl,
            shaders,
            images,
        }
    }
}
