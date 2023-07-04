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
use std::process::ExitCode;

const SPACES_X: u32 = 8;
const SPACES_Y: u32 = 4;
const BINGO: &'static str = "BINGO";

const SPACE_SIZE: u32 = 128;

fn generate_space(
    cr: &cairo::Context,
    text: &str,
) -> Result<(), cairo::Error> {
    cr.save()?;

    cr.set_font_size(SPACE_SIZE as f64 * 0.7);
    cr.select_font_face(
        "Noto Sans",
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );

    let extents = cr.text_extents(text)?;
    cr.move_to((SPACE_SIZE as f64 / 2.0
                - extents.x_bearing()
                - extents.width() / 2.0)
               .round(),
               SPACE_SIZE as f64 * 0.75);

    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.show_text(text)?;

    cr.restore()?;

    Ok(())
}

fn generate_spaces(cr: &cairo::Context) -> Result<(), cairo::Error> {
    let n_bingo = BINGO.chars().count();
    let mut bingo = BINGO.chars();

    for y in 0..SPACES_Y {
        for x in 0..SPACES_X {
            cr.save()?;

            cr.translate(
                x as f64 * SPACE_SIZE as f64,
                y as f64 * SPACE_SIZE as f64,
            );

            let space_num = y * SPACES_X + x;

            if space_num < SPACES_X * SPACES_Y - n_bingo as u32 {
                generate_space(cr, &format!("{}", space_num + 1))?;
            } else {
                let letter_start = bingo.as_str();
                let letter_len = bingo.next().unwrap().len_utf8();
                generate_space(cr, &letter_start[0..letter_len])?;
            }

            cr.restore()?;
        }
    }

    Ok(())
}

fn generate_texture() -> Result<cairo::ImageSurface, cairo::Error> {
    let surface = cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        (SPACES_X * SPACE_SIZE) as i32,
        (SPACES_Y * SPACE_SIZE) as i32,
    )?;

    let cr = cairo::Context::new(&surface)?;

    cr.save()?;
    cr.set_source_rgba(0.0, 1.0, 0.0, 1.0);
    cr.set_operator(cairo::Operator::Source);
    cr.paint()?;
    cr.restore()?;

    generate_spaces(&cr)?;

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
            "usage: create_bingo_texture <filename>"
        );
        return ExitCode::FAILURE;
    }

    let output_filename = args.nth(1).unwrap();

    let surface = match generate_texture() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        },
    };

    if let Err(e) = write_surface(&surface, &output_filename) {
        eprintln!("{}: {}", output_filename.to_string_lossy(), e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
