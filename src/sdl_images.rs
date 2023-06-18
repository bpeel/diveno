use std::rc::Rc;
use std::path::PathBuf;
use sdl2::image::LoadSurface;
use sdl2::surface::Surface;
use sdl2::pixels::PixelFormatEnum;
use glow::HasContext;
use crate::images::{Texture, ImageSet};

fn copy_image(
    dst: &mut Vec<u8>,
    src: &[u8],
    width: usize,
    height: usize,
    pixel_stride: usize,
    src_row_stride: usize,
) {
    let dst_row_stride = (width * pixel_stride + 3) & !3;

    dst.resize(dst_row_stride * height, 0);

    let row_len = pixel_stride * width;

    for row in 0..height {
        let dst_start = row * dst_row_stride;
        let src_start = row * src_row_stride;

        dst[dst_start..dst_start + row_len]
            .copy_from_slice(&src[src_start..src_start + row_len]);
    }
}


fn copy_mipmap_surface_to_texture(
    gl: &glow::Context,
    surface: &Surface,
    pixels: &[u8],
) -> Result<(), String> {
    let mut width = surface.width() as usize;
    let full_height = surface.height() as usize;
    // The image on disk is 1.5 as tall to store the mipmaps
    let mut height = full_height * 2 / 3;
    let mut level = 0;
    let mut data_copy = Vec::new();
    let mut x = 0;
    let mut y = 0;

    let (pixel_stride, gl_format) = match surface.pixel_format_enum() {
        PixelFormatEnum::RGBA32 => (4, glow::RGBA),
        _ => return Err(format!(
            "Unsupported pixel format: {:?}",
            surface.pixel_format_enum(),
        )),
    };

    let row_stride = surface.pitch() as usize;

    loop {
        let pixels = if level == 0 {
            &pixels[0..height * row_stride]
        } else {
            // We can’t upload a subregion of an image with GLES so
            // let’s copy it into a temporary buffer without any
            // padding between the lines.
            copy_image(
                &mut data_copy,
                &pixels[x * pixel_stride + y * row_stride..],
                width,
                height,
                pixel_stride,
                row_stride,
            );

            &data_copy
        };

        unsafe {
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                level,
                gl_format as i32,
                width as i32,
                height as i32,
                0, // border
                gl_format,
                glow::UNSIGNED_BYTE,
                Some(pixels),
            );
        }

        if width <= 1 && height <= 1 {
            break;
        }

        if level & 1 == 0 {
            y += height;
        } else {
            x += width;
        }

        width = std::cmp::max(width / 2, 1);
        height = std::cmp::max(height / 2, 1);
        level += 1;
    }

    Ok(())
}


fn load_mipmap_texture(
    gl: Rc<glow::Context>,
    filename: &str,
) -> Result<Texture, String> {
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
        copy_mipmap_surface_to_texture(
            &gl,
            &surface,
            pixels,
        )
    })?;

    Ok(Texture::new(gl, id))
}

pub fn load_image_set(gl: &Rc<glow::Context>) -> Result<ImageSet, String> {
    ImageSet::new(gl, load_mipmap_texture)
}
