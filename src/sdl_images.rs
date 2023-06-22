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
use std::path::PathBuf;
use sdl2::image::LoadSurface;
use sdl2::surface::Surface;
use sdl2::pixels::PixelFormatEnum;
use glow::HasContext;
use crate::game::images::{ImageSet, ImageLoader};

fn copy_surface_to_texture(
    gl: &glow::Context,
    surface: &Surface,
    pixels: &[u8],
) -> Result<(), String> {
    let width = surface.width() as usize;
    let height = surface.height() as usize;

    let gl_format = match surface.pixel_format_enum() {
        PixelFormatEnum::RGBA32 => glow::RGBA,
        _ => return Err(format!(
            "Unsupported pixel format: {:?}",
            surface.pixel_format_enum(),
        )),
    };

    let row_stride = surface.pitch() as usize;

    let pixels = &pixels[0..height * row_stride];

    unsafe {
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0, // level
            gl_format as i32,
            width as i32,
            height as i32,
            0, // border
            gl_format,
            glow::UNSIGNED_BYTE,
            Some(pixels),
        );
    }

    Ok(())
}

fn load_mipmap_texture(
    gl: &glow::Context,
    filename: &str,
) -> Result<glow::Texture, String> {
    let path: PathBuf = ["data", filename].iter().collect();

    let surface = Surface::from_file(path)?;

    let id = unsafe {
        gl.create_texture()?
    };

    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(id));
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR_MIPMAP_NEAREST as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );
    }

    surface.with_lock(|pixels| {
        copy_surface_to_texture(
            gl,
            &surface,
            pixels,
        )
    })?;

    unsafe {
        gl.generate_mipmap(glow::TEXTURE_2D);
    }

    Ok(id)
}

pub fn load_image_set(gl: &Rc<glow::Context>) -> Result<ImageSet, String> {
    let mut loader = ImageLoader::new(Rc::clone(gl));

    while let Some(filename) = loader.next_filename() {
        loader.loaded(load_mipmap_texture(&gl, filename)?);
    }

    Ok(loader.complete())
}
