use cairo;
use std::f64::consts::PI;
use std::process::ExitCode;

static LETTERS: &'static str = "ABCDEFGHIJKLMNOPQRSTUVWXYZĤŜĜĈĴŬ";

const TILE_SIZE: u32 = 128;
const BORDER_SIZE: u32 = TILE_SIZE / 16;
const CORNER_SIZE: u32 = TILE_SIZE / 4;

static COLORS: [(f64, f64, f64); 4] = [
    (0.000, 0.000, 0.000),
    (0.000, 0.467, 0.780),
    (0.906, 0.000, 0.165),
    (1.000, 0.741, 0.000),
];

fn get_texture_size() -> (u32, u32) {
    let (mut w, mut h) = (1, 1);
    let n_letters = LETTERS.chars().count();

    while w * h < n_letters as u32 {
        if w <= h {
            w *= 2;
        } else {
            h *= 2;
        }
    }

    (w, h)
}

fn curved_rectangle(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    corner_size: f64,
) {
    cr.arc(
        x + corner_size,
        y + corner_size,
        corner_size,
        PI,
        PI / 2.0 * 3.0,
    );
    cr.line_to(x + w - corner_size, y);

    cr.arc(
        x + w - corner_size,
        y + corner_size,
        corner_size,
        PI / 2.0 * 3.0,
        PI * 2.0,
    );
    cr.line_to(x + w, y + h - corner_size);

    cr.arc(
        x + w - corner_size,
        y + h - corner_size,
        corner_size,
        0.0,
        PI / 2.0,
    );
    cr.line_to(x + corner_size, y + h);

    cr.arc(
        x + corner_size,
        y + h - corner_size,
        corner_size,
        PI / 2.0,
        PI,
    );
    cr.line_to(x, y + corner_size);
}

fn generate_tile(
    cr: &cairo::Context,
    letter: &str,
    color: &(f64, f64, f64),
) -> Result<(), cairo::Error> {
    cr.save()?;

    curved_rectangle(
        cr,
        BORDER_SIZE as f64,
        BORDER_SIZE as f64,
        (TILE_SIZE - BORDER_SIZE * 2) as f64,
        (TILE_SIZE - BORDER_SIZE * 2) as f64,
        CORNER_SIZE as f64,
    );

    cr.set_source_rgb(color.0, color.1, color.2);

    cr.fill()?;

    cr.set_font_size((TILE_SIZE as f64 - BORDER_SIZE as f64 * 2.0) * 0.8);
    cr.select_font_face(
        "Noto Sans",
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );

    let extents = cr.text_extents(letter)?;
    cr.move_to((TILE_SIZE as f64 / 2.0
                - extents.x_bearing()
                - extents.width() / 2.0)
               .round(),
               BORDER_SIZE as f64
               + (TILE_SIZE as f64 - BORDER_SIZE as f64 * 2.0)
               * 0.82);

    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.show_text(letter)?;

    cr.restore()?;

    Ok(())
}

fn generate_tiles(
    cr: &cairo::Context,
    tiles_per_row: u32,
    color: &(f64, f64, f64),
) -> Result<(), cairo::Error> {
    let mut letters = LETTERS.char_indices().collect::<Vec<(usize, char)>>();

    letters.sort_unstable_by(|(_, ch_a), (_, ch_b)| ch_a.cmp(ch_b));

    for (tile_num, &(char_offset, ch)) in letters.iter().enumerate() {
        let x = tile_num as u32 % tiles_per_row;
        let y = tile_num as u32 / tiles_per_row;

        cr.save()?;

        cr.translate(x as f64 * TILE_SIZE as f64, y as f64 * TILE_SIZE as f64);

        let letter = &LETTERS[char_offset..char_offset + ch.len_utf8()];

        generate_tile(cr, letter, color)?;

        cr.restore()?;
    }

    Ok(())
}

fn generate_texture() -> Result<cairo::ImageSurface, cairo::Error> {
    let (x_tiles, y_tiles) = get_texture_size();
    let height = y_tiles as i32 * TILE_SIZE as i32;

    let surface = cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        x_tiles as i32 * TILE_SIZE as i32,
        height * COLORS.len() as i32,
    )?;

    let cr = cairo::Context::new(&surface)?;

    cr.save()?;
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    cr.set_operator(cairo::Operator::Source);
    cr.paint()?;
    cr.restore()?;

    for (color_num, color) in COLORS.iter().enumerate() {
        cr.save()?;

        cr.translate(0.0, color_num as f64 * height as f64);

        generate_tiles(&cr, x_tiles, color)?;

        cr.restore()?;
    }

    surface.flush();

    Ok(surface)
}

pub fn write_surface<S: AsRef<cairo::Surface>, P: AsRef<std::path::Path>>(
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
        eprintln!("usage: create_tile_texture <filename>");
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

    match write_surface(&surface, &output_filename) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}: {}", output_filename.to_string_lossy(), e);
            ExitCode::FAILURE
        },
    }
}
