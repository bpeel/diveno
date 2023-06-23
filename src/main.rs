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

mod game;
mod sdl_images;

use game::{logic, shaders, images, game_painter, paint_data};
use game::dictionary::Dictionary;

use sdl2;
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use std::process::ExitCode;
use std::rc::Rc;
use glow::HasContext;

struct Context {
    gl: Rc<glow::Context>,
    _gl_context: sdl2::video::GLContext,
    window: sdl2::video::Window,
    _video_subsystem: sdl2::VideoSubsystem,
    event_pump: sdl2::EventPump,
    _sdl: sdl2::Sdl,
}

impl Context {
    fn new() -> Result<Context, String> {
        let sdl = sdl2::init()?;

        let event_pump = sdl.event_pump()?;

        let video_subsystem = sdl.video()?;

        let gl_attr = video_subsystem.gl_attr();

        gl_attr.set_red_size(8);
        gl_attr.set_green_size(8);
        gl_attr.set_blue_size(8);
        gl_attr.set_alpha_size(0);
        gl_attr.set_depth_size(0);
        gl_attr.set_double_buffer(true);
        gl_attr.set_context_major_version(2);
        gl_attr.set_context_minor_version(0);
        gl_attr.set_context_profile(sdl2::video::GLProfile::GLES);

        let window = match video_subsystem.window("Diveno", 800, 600)
            .resizable()
            .opengl()
            .build()
        {
            Ok(w) => w,
            Err(e) => return Err(e.to_string()),
        };

        let gl_context = window.gl_create_context()?;

        window.gl_make_current(&gl_context)?;

        let gl = unsafe {
            glow::Context::from_loader_function(|s| {
                video_subsystem.gl_get_proc_address(s) as *const _
            })
        };

        Ok(Context {
            gl: Rc::new(gl),
            _gl_context: gl_context,
            window,
            _video_subsystem: video_subsystem,
            event_pump,
            _sdl: sdl,
        })
    }
}

fn check_extension(context: &Context, name: &str) -> bool {
    let extensions = unsafe {
        context.gl.get_parameter_string(glow::EXTENSIONS)
    };

    extensions.split(' ').find(|&ext| ext == name).is_some()
}

struct GameData<'a> {
    context: &'a mut Context,
    logic: logic::Logic,
    game_painter: game_painter::GamePainter,
    redraw_queued: bool,
    should_quit: bool,
}

impl<'a> GameData<'a> {
    fn new(
        context: &'a mut Context,
        shaders: shaders::Shaders,
        images: images::ImageSet,
    ) -> Result<GameData<'a>, String> {
        let paint_data = Rc::new(paint_data::PaintData::new(
            Rc::clone(&context.gl),
            check_extension(context, "GL_OES_vertex_array_object"),
            shaders,
            images,
        ));

        let game_painter = game_painter::GamePainter::new(paint_data)?;

        let dictionary = load_dictionary()?;

        Ok(GameData {
            context,
            logic: logic::Logic::new(dictionary),
            game_painter,
            redraw_queued: true,
            should_quit: false,
        })
    }
}

fn handle_keycode_down(game_data: &mut GameData, code: Keycode) {
    match code {
        Keycode::Escape => game_data.should_quit = true,
        Keycode::Backspace => game_data.logic.press_key(logic::Key::Backspace),
        Keycode::Return => game_data.logic.press_key(logic::Key::Enter),
        code => {
            if let Some(ch) = char::from_u32(code as u32) {
                if ch.is_alphabetic() {
                    game_data.logic.press_key(logic::Key::Letter(ch));
                }
            }
        }
    }
}

fn handle_event(game_data: &mut GameData, event: Event) {
    match event {
        Event::Quit {..} => game_data.should_quit = true,
        Event::KeyDown { keycode: Some(code), .. } => {
            handle_keycode_down(game_data, code);
        },
        Event::Window { win_event, .. } => {
            match win_event {
                WindowEvent::Close => game_data.should_quit = true,
                WindowEvent::Exposed => game_data.redraw_queued = true,
                WindowEvent::Shown => {
                    let (width, height) = game_data.context.window.size();
                    game_data.game_painter.update_fb_size(width, height);
                    game_data.redraw_queued = true;
                },
                WindowEvent::SizeChanged(width, height) => {
                    game_data.game_painter.update_fb_size(
                        width as u32,
                        height as u32
                    );
                    game_data.redraw_queued = true;
                },
                _ => {},
            }
        },
        _ => {}
    }
}

fn flush_logic_events(game_data: &mut GameData) {
    while let Some(event) = game_data.logic.get_event() {
        match event {
            logic::Event::GuessEntered |
            logic::Event::WrongGuessEntered |
            logic::Event::WordChanged |
            logic::Event::GridChanged => {
                game_data.redraw_queued = true;
            },
        }

        game_data.game_painter.handle_logic_event(&event);
    }
}

fn redraw(game_data: &mut GameData) {
    if !game_data.game_painter.paint(&game_data.logic) {
        game_data.redraw_queued = false;
    }

    game_data.context.window.gl_swap_window();
}

fn main_loop(game_data: &mut GameData) {
    while !game_data.should_quit {
        if game_data.redraw_queued {
            while let Some(event) = game_data.context.event_pump.poll_event() {
                handle_event(game_data, event);
            }

            flush_logic_events(game_data);

            redraw(game_data);
        } else {
            let event = game_data.context.event_pump.wait_event();
            handle_event(game_data, event);
            flush_logic_events(game_data);
        }
    }
}

fn load_data_file(filename: &str) -> Result<Vec<u8>, String> {
    let path: std::path::PathBuf = ["data", filename].iter().collect();

    std::fs::read(&path).map_err(|e| format!("{}: {}", filename, e))
}

fn load_dictionary() -> Result<Dictionary, String> {
    load_data_file("dictionary.bin")
        .map(|d| Dictionary::new(d.into_boxed_slice()))
}

fn load_shaders(gl: Rc<glow::Context>) -> Result<shaders::Shaders, String> {
    let mut loader = shaders::ShaderLoader::new(gl);

    while let Some(filename) = loader.next_filename() {
        loader.loaded(&load_data_file(filename)?)?;
    }

    loader.complete()
}

pub fn main() -> ExitCode {
    let mut context = match Context::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to initialise SDL: {}", e);
            return ExitCode::FAILURE;
        },
    };

    let shaders = match load_shaders(Rc::clone(&context.gl)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    };

    let images = match sdl_images::load_image_set(&context.gl) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    };

    let mut game_data = match GameData::new(&mut context, shaders, images) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        },
    };

    main_loop(&mut game_data);

    ExitCode::SUCCESS
}
