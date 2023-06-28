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

use cairo;
use std::f64::consts::PI;
use std::process::ExitCode;

const IMAGE_WIDTH: u32 = 100;
const IMAGE_HEIGHT: u32 = 128;
const FRAME_WIDTH: f64 = 40.0;
const COLOR_STOP_POINT: f64 = 0.25;

const INNER_COLOR: [f64; 3] = [0.153, 0.153, 0.153];
const MIDDLE_COLOR: [f64; 3] = [0.698, 0.698, 1.000];
const OUTER_COLOR: [f64; 3] = [0.000, 0.000, 1.000];

fn add_color_stop(gradient: &cairo::Gradient, offset: f64, color: [f64; 3]) {
    gradient.add_color_stop_rgb(offset, color[0], color[1], color[2]);
}

fn add_color_stops(gradient: &cairo::Gradient) {
    add_color_stop(&gradient, 0.0, OUTER_COLOR);
    add_color_stop(&gradient, COLOR_STOP_POINT, MIDDLE_COLOR);
    add_color_stop(&gradient, 1.0 - COLOR_STOP_POINT, MIDDLE_COLOR);
    add_color_stop(&gradient, 1.0, INNER_COLOR);
}

fn create_gradient(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64
) -> cairo::LinearGradient {
    let gradient = cairo::LinearGradient::new(x0, y0, x1, y1);

    add_color_stops(&gradient);

    gradient
}

fn draw_rectangles(cr: &cairo::Context) -> Result<(), cairo::Error> {
    // Left side
    cr.rectangle(
        0.0,
        FRAME_WIDTH,
        FRAME_WIDTH,
        IMAGE_HEIGHT as f64 - FRAME_WIDTH * 2.0,
    );
    cr.set_source(&create_gradient(0.0, 0.0, FRAME_WIDTH, 0.0))?;
    cr.fill()?;

    // Right side
    cr.rectangle(
        IMAGE_WIDTH as f64 - FRAME_WIDTH,
        FRAME_WIDTH,
        FRAME_WIDTH,
        IMAGE_HEIGHT as f64 - FRAME_WIDTH * 2.0,
    );
    cr.set_source(&create_gradient(
        IMAGE_WIDTH as f64,
        0.0,
        IMAGE_WIDTH as f64 - FRAME_WIDTH,
        0.0,
    ))?;
    cr.fill()?;

    // Top side
    cr.rectangle(
        FRAME_WIDTH,
        0.0,
        IMAGE_WIDTH as f64 - FRAME_WIDTH * 2.0,
        FRAME_WIDTH,
    );
    cr.set_source(&create_gradient(0.0, 0.0, 0.0, FRAME_WIDTH))?;
    cr.fill()?;

    // Bottom side
    cr.rectangle(
        FRAME_WIDTH,
        IMAGE_HEIGHT as f64 - FRAME_WIDTH,
        IMAGE_WIDTH as f64 - FRAME_WIDTH * 2.0,
        FRAME_WIDTH,
    );
    cr.set_source(&create_gradient(
        0.0,
        IMAGE_HEIGHT as f64,
        0.0,
        IMAGE_HEIGHT as f64 - FRAME_WIDTH,
    ))?;
    cr.fill()?;

    Ok(())
}

fn create_radial_gradient(x: f64, y: f64) -> cairo::RadialGradient {
    let gradient = cairo::RadialGradient::new(
        x,
        y,
        FRAME_WIDTH,
        x,
        y,
        0.0,
    );

    add_color_stops(&gradient);

    gradient
}

fn draw_corner(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    angle1: f64,
    angle2: f64,
) -> Result<(), cairo::Error> {
    // Top-left
    cr.move_to(x, y);
    cr.rel_line_to(FRAME_WIDTH * angle1.cos(), FRAME_WIDTH * angle1.sin());
    cr.arc(x, y, FRAME_WIDTH, angle1, angle2);
    cr.set_source(&create_radial_gradient(x, y))?;
    cr.fill()
}

fn draw_corners(cr: &cairo::Context) -> Result<(), cairo::Error> {
    // Top left
    draw_corner(cr, FRAME_WIDTH, FRAME_WIDTH, PI, PI * 1.5)?;
    // Top right
    draw_corner(
        cr,
        IMAGE_WIDTH as f64 - FRAME_WIDTH,
        FRAME_WIDTH,
        PI * 1.5,
        PI * 2.0,
    )?;
    // Bottom right
    draw_corner(
        cr,
        IMAGE_WIDTH as f64 - FRAME_WIDTH,
        IMAGE_HEIGHT as f64 - FRAME_WIDTH,
        0.0,
        PI / 2.0,
    )?;
    // Bottom left
    draw_corner(
        cr,
        FRAME_WIDTH,
        IMAGE_HEIGHT as f64 - FRAME_WIDTH,
        PI / 2.0,
        PI,
    )?;

    Ok(())
}

fn draw_inner_part(cr: &cairo::Context) -> Result<(), cairo::Error> {
    cr.set_source_rgb(INNER_COLOR[0], INNER_COLOR[1], INNER_COLOR[2]);
    cr.rectangle(
        FRAME_WIDTH,
        FRAME_WIDTH,
        IMAGE_WIDTH as f64 - FRAME_WIDTH * 2.0,
        IMAGE_HEIGHT as f64 - FRAME_WIDTH * 2.0,
    );
    cr.fill()?;

    Ok(())
}

fn draw_frame(cr: &cairo::Context) -> Result<(), cairo::Error> {
    draw_rectangles(cr)?;
    draw_corners(cr)?;
    draw_inner_part(cr)?;

    Ok(())
}

fn generate_image() -> Result<cairo::ImageSurface, cairo::Error> {
    let surface = cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        IMAGE_WIDTH as i32,
        IMAGE_HEIGHT as i32,
    )?;

    let cr = cairo::Context::new(&surface)?;

    cr.save()?;
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    cr.set_operator(cairo::Operator::Source);
    cr.paint()?;
    cr.restore()?;

    draw_frame(&cr)?;

    surface.flush();

    Ok(surface)
}

fn write_surface<S: AsRef<cairo::Surface>, P: AsRef<std::path::Path>>(
    surface: S,
    filename: P,
) -> Result<(), String> {
    let mut file = match std::fs::File::create(filename) {
        Ok(f) => f,
        Err(e) => return Err(e.to_string()),
    };

    match surface.as_ref().write_to_png(&mut file) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn generate_png_image<P: AsRef<std::path::Path>>(
    output_filename: P,
) -> Result<(), String> {
    match generate_image() {
        Ok(surface) => write_surface(surface, output_filename),
        Err(e) => Err(e.to_string()),
    }
}

fn generate_svg_image_inner<P: AsRef<std::path::Path>>(
    output_filename: P,
) -> Result<(), cairo::Error> {
    let surface = cairo::SvgSurface::new(
        IMAGE_WIDTH as f64,
        IMAGE_HEIGHT as f64,
        Some(output_filename),
    )?;

    let cr = cairo::Context::new(&surface)?;

    draw_frame(&cr)?;

    surface.flush();

    Ok(())
}

fn generate_svg_image<P: AsRef<std::path::Path>>(
    output_filename: P,
) -> Result<(), String> {
    generate_svg_image_inner(output_filename).map_err(|e| e.to_string())
}

pub fn main() -> ExitCode {
    let mut args = std::env::args_os();

    if args.len() != 2 {
        eprintln!(
            "usage: create_frame_image <filename>"
        );
        return ExitCode::FAILURE;
    }

    let output_filename = args.nth(1).unwrap();

    match if output_filename.to_string_lossy().ends_with(".svg") {
        generate_svg_image(&output_filename)
    } else {
        generate_png_image(&output_filename)
    } {
        Err(e) => {
            eprintln!("{}: {}", output_filename.to_string_lossy(), e);
            ExitCode::FAILURE
        },
        Ok(()) => {
            ExitCode::SUCCESS
        },
    }
}
