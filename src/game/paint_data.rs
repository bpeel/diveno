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
use std::cell::Cell;
use super::{shaders, images, quad_tool};

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
