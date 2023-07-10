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

use game::{logic, shaders, images, game_painter, paint_data, sound_queue};
use game::{timer, timeout};

use sdl2;
use sdl2::event::{Event, WindowEvent};
use sdl2::mixer::{Channel, Chunk};
use sdl2::keyboard::Keycode;
use sdl2::video::FullscreenType;
use std::process::ExitCode;
use std::rc::Rc;
use glow::HasContext;
use timeout::Timeout;

struct Context {
    _audio_subsystem: sdl2::AudioSubsystem,
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

        let audio_subsystem = sdl.audio()?;

        sdl2::mixer::open_audio(
            44100, // frequency
            sdl2::mixer::DEFAULT_FORMAT,
            2, // channels
            512, // chunk_size
        )?;

        Ok(Context {
            _audio_subsystem: audio_subsystem,
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
    start_time: timer::Timer,
    sound_files: Vec<Chunk>,
    sound_queue: sound_queue::SoundQueue,
    game_painter: game_painter::GamePainter,
    redraw_time: Option<i64>,
    should_quit: bool,
    is_fullscreen: bool,
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

        let logic = load_logic()?;

        let sound_files = load_sound_files()?;

        Ok(GameData {
            context,
            logic,
            start_time: timer::Timer::new(),
            sound_files,
            sound_queue: sound_queue::SoundQueue::new(),
            game_painter,
            redraw_time: Some(0),
            should_quit: false,
            is_fullscreen: false,
        })
    }
}

fn toggle_fullscreen(game_data: &mut GameData) {
    if game_data.context.window.set_fullscreen(
        if !game_data.is_fullscreen {
            FullscreenType::True
        } else {
            FullscreenType::Off
        }
    ).is_ok() {
        game_data.is_fullscreen = !game_data.is_fullscreen;
    }
}

fn handle_keycode_down(game_data: &mut GameData, code: Keycode) {
    match code {
        Keycode::Backspace => game_data.logic.press_key(logic::Key::Backspace),
        Keycode::Delete => game_data.logic.press_key(logic::Key::Delete),
        Keycode::Return => game_data.logic.press_key(logic::Key::Enter),
        Keycode::PageDown => game_data.logic.press_key(logic::Key::PageDown),
        Keycode::Space => game_data.logic.press_key(logic::Key::Space),
        Keycode::Home => game_data.logic.press_key(logic::Key::Home),
        Keycode::Left => game_data.logic.press_key(logic::Key::Left),
        Keycode::Right => game_data.logic.press_key(logic::Key::Right),
        Keycode::Up => game_data.logic.press_key(logic::Key::Up),
        Keycode::Down => game_data.logic.press_key(logic::Key::Down),
        Keycode::Backquote => game_data.logic.press_key(logic::Key::Backtick),
        Keycode::Dollar => game_data.logic.press_key(logic::Key::Dollar),
        Keycode::F11 => toggle_fullscreen(game_data),
        code => {
            if let Some(ch) = char::from_u32(code as u32) {
                if ch.is_alphabetic() {
                    game_data.logic.press_key(logic::Key::Letter(ch));
                }
            }
        }
    }
}

fn queue_redraw(game_data: &mut GameData) {
    game_data.redraw_time = Some(0);
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
                WindowEvent::Exposed => queue_redraw(game_data),
                WindowEvent::Shown => {
                    let (width, height) = game_data.context.window.size();
                    game_data.game_painter.update_fb_size(width, height);
                    queue_redraw(game_data);
                },
                WindowEvent::SizeChanged(width, height) => {
                    game_data.game_painter.update_fb_size(
                        width as u32,
                        height as u32
                    );
                    queue_redraw(game_data);
                },
                _ => {},
            }
        },
        _ => {}
    }
}

fn flush_sounds(game_data: &mut GameData) {
    while let Some(sound) = game_data.sound_queue.next_ready_sound() {
        let _ = Channel::all().play(&game_data.sound_files[sound as usize], 0);
    }
}

fn flush_logic_events(game_data: &mut GameData) {
    while let Some(event) = game_data.logic.get_event() {
        if game_data.game_painter.handle_logic_event(&game_data.logic, &event) {
            queue_redraw(game_data);
        }

        game_data.sound_queue.handle_logic_event(&game_data.logic, &event);
    }
}

fn redraw(game_data: &mut GameData) {
    match game_data.game_painter.paint(&mut game_data.logic) {
        Timeout::Milliseconds(ms) => {
            let time = game_data.start_time.elapsed() + ms;
            game_data.redraw_time = Some(time);
        },
        Timeout::Forever => game_data.redraw_time = None,
    }

    game_data.context.window.gl_swap_window();
}

fn redraw_delay(game_data: &GameData) -> Timeout {
    match game_data.redraw_time {
        Some(time) => {
            let delay = (time - game_data.start_time.elapsed()).max(0);
            Timeout::Milliseconds(delay)
        },
        None => Timeout::Forever,
    }
}

fn main_loop(game_data: &mut GameData) {
    while !game_data.should_quit {
        let redraw_delay = redraw_delay(game_data);
        let sound_delay = game_data.sound_queue.next_delay();

        match redraw_delay.min(sound_delay) {
            Timeout::Forever => {
                let event = game_data.context.event_pump.wait_event();
                handle_event(game_data, event);
            },
            Timeout::Milliseconds(ms) if ms <= 0 => {
                while let Some(event) = game_data
                    .context
                    .event_pump
                    .poll_event()
                {
                    handle_event(game_data, event);
                }
            },
            Timeout::Milliseconds(ms) => {
                if let Some(event) = game_data
                    .context
                    .event_pump
                    .wait_event_timeout(ms as u32)
                {
                    handle_event(game_data, event);
                }
            },
        }

        flush_logic_events(game_data);

        if let Timeout::Milliseconds(ms) = redraw_delay {
            if ms <= 0 {
                redraw(game_data);
            }
        }

        flush_sounds(game_data);
    }
}

fn data_filename(filename: &str) -> std::path::PathBuf {
    ["data", filename].iter().collect()
}

fn load_data_file(filename: &str) -> Result<Vec<u8>, String> {
    let path = data_filename(filename);

    std::fs::read(&path).map_err(|e| format!("{}: {}", filename, e))
}

fn load_logic() -> Result<logic::Logic, String> {
    let mut loader = logic::LogicLoader::new();

    while let Some(filename) = loader.next_filename() {
        loader.loaded(load_data_file(filename)?.into_boxed_slice());
    }

    Ok(loader.complete())
}

fn load_shaders(gl: Rc<glow::Context>) -> Result<shaders::Shaders, String> {
    let mut loader = shaders::ShaderLoader::new(gl);

    while let Some(filename) = loader.next_filename() {
        loader.loaded(&load_data_file(filename)?)?;
    }

    loader.complete()
}

fn load_sound_files() -> Result<Vec<Chunk>, String> {
    let mut sound_files = Vec::with_capacity(sound_queue::SOUND_FILES.len());

    for filename in sound_queue::SOUND_FILES.iter() {
        sound_files.push(Chunk::from_file(data_filename(filename))?);
    }

    Ok(sound_files)
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
