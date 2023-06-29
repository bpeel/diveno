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

const BALL_SIZE: u32 = 64;
const N_BALLS: usize = 26;
const TEXTURE_WIDTH: u32 = 1024;
const TEXTURE_HEIGHT: u32 = 256;

fn add_color_stop(gradient: &cairo::Gradient, offset: f64, color: [f64; 3]) {
    gradient.add_color_stop_rgb(offset, color[0], color[1], color[2]);
}

fn draw_ball_background(
    cr: &cairo::Context,
    inner_color: [f64; 3],
    outer_color: [f64; 3],
) -> Result<(), cairo::Error> {
    cr.arc(
        BALL_SIZE as f64 / 2.0,
        BALL_SIZE as f64 / 2.0,
        BALL_SIZE as f64 / 2.0,
        0.0,
        2.0 * PI,
    );

    let gradient = cairo::RadialGradient::new(
        BALL_SIZE as f64 / 2.0,
        BALL_SIZE as f64 / 2.0,
        0.0,
        BALL_SIZE as f64 / 2.0,
        BALL_SIZE as f64 / 2.0,
        BALL_SIZE as f64 / 2.0,
    );

    add_color_stop(&gradient, 0.0, inner_color);
    add_color_stop(&gradient, 0.75, inner_color);
    add_color_stop(&gradient, 1.0, outer_color);

    cr.set_source(&gradient)?;
    cr.fill()?;

    Ok(())
}

fn draw_numbered_ball(
    cr: &cairo::Context,
    ball_num: u32,
) -> Result<(), cairo::Error> {
    draw_ball_background(cr, [0.614, 0.177, 0.196], [0.457, 0.147, 0.161])?;

    cr.save()?;

    cr.set_font_size(BALL_SIZE as f64 * 0.5);
    cr.select_font_face(
        "Noto Sans",
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );

    let text = format!("{}", ball_num);

    let extents = cr.text_extents(&text)?;
    cr.move_to(
        (BALL_SIZE as f64 / 2.0
         - extents.x_bearing()
         - extents.width() / 2.0)
            .round(),
        BALL_SIZE as f64 * 0.68,
    );

    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.show_text(&text)?;

    cr.restore()?;

    Ok(())
}

fn draw_black_ball(cr: &cairo::Context) -> Result<(), cairo::Error> {
    draw_ball_background(cr, [0.1, 0.1, 0.1], [0.0, 0.0, 0.0])?;

    Ok(())
}

fn draw_balls(cr: &cairo::Context) -> Result<(), cairo::Error> {
    let (mut x, mut y) = (0, 0);

    for ball_num in 0..N_BALLS {
        assert!(x + BALL_SIZE <= TEXTURE_WIDTH);
        assert!(y + BALL_SIZE <= TEXTURE_HEIGHT);

        cr.save()?;
        cr.translate(x as f64, y as f64);

        if ball_num + 1 < N_BALLS {
            draw_numbered_ball(cr, ball_num as u32 + 1)?;
        } else {
            draw_black_ball(cr)?;
        }

        x += BALL_SIZE + BALL_SIZE / 2;

        if x + BALL_SIZE > TEXTURE_WIDTH {
            x = 0;
            y += BALL_SIZE + BALL_SIZE / 2;
        }

        cr.restore()?;
    }

    Ok(())
}

fn generate_image() -> Result<cairo::ImageSurface, cairo::Error> {
    let surface = cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        TEXTURE_WIDTH as i32,
        TEXTURE_HEIGHT as i32,
    )?;

    let cr = cairo::Context::new(&surface)?;

    cr.save()?;
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    cr.set_operator(cairo::Operator::Source);
    cr.paint()?;
    cr.restore()?;

    draw_balls(&cr)?;

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

pub fn main() -> ExitCode {
    let mut args = std::env::args_os();

    if args.len() != 2 {
        eprintln!(
            "usage: create_ball_texture <filename>"
        );
        return ExitCode::FAILURE;
    }

    let output_filename = args.nth(1).unwrap();

    let surface = match generate_image() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        },
    };

    if let Err(e) = write_surface(&surface, &output_filename) {
        eprintln!("{}: {}", output_filename.to_string_lossy(), e);
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
