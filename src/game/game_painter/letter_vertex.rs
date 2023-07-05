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

use super::super::paint_data::PaintData;
use super::super::buffer::Buffer;
use super::super::shaders;
use super::super::array_object::ArrayObject;
use glow::HasContext;
use std::rc::Rc;

#[repr(C)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub s: u16,
    pub t: u16,
    // Vertical Rotation centre
    pub ry: f32,
    // Rotation progress
    pub rp: f32,
    // Color of the background of the tile
    pub color: [u8; 3],
}

pub fn create_array_object(
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
