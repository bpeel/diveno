mod shaders;
mod images;
mod sdl_images;
mod paint_data;
mod game_painter;
mod logic;
mod buffer;
mod letter_texture;
mod array_object;
mod quad_tool;

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

        Ok(GameData {
            context,
            logic: logic::Logic::new(),
            game_painter,
            redraw_queued: true,
            should_quit: false,
        })
    }
}

fn handle_event(game_data: &mut GameData, event: Event) {
    match event {
        Event::Quit {..} |
        Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
            game_data.should_quit = true;
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

fn redraw(game_data: &mut GameData) {
    game_data.redraw_queued = false;

    game_data.game_painter.paint(&game_data.logic);

    game_data.context.window.gl_swap_window();
}

fn main_loop(game_data: &mut GameData) {
    while !game_data.should_quit {
        if game_data.redraw_queued {
            while let Some(event) = game_data.context.event_pump.poll_event() {
                handle_event(game_data, event);
            }

            redraw(game_data);
        } else {
            let event = game_data.context.event_pump.wait_event();
            handle_event(game_data, event);
        }
    }
}

pub fn main() -> ExitCode {
    let mut context = match Context::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to initialise SDL: {}", e);
            return ExitCode::FAILURE;
        },
    };

    let shaders = match shaders::Shaders::new(&context.gl) {
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
