use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::process::ExitCode;

struct Context {
    canvas: sdl2::render::Canvas<sdl2::video::Window>,
    _video_subsystem: sdl2::VideoSubsystem,
    event_pump: sdl2::EventPump,
    _sdl: sdl2::Sdl,
    redraw_queued: bool,
    should_quit: bool,
}

impl Context {
    fn new() -> Result<Context, String> {
        let sdl = sdl2::init()?;

        let event_pump = sdl.event_pump()?;

        let video_subsystem = sdl.video()?;

        let window = match video_subsystem.window("Diveno", 800, 600).build() {
            Ok(w) => w,
            Err(e) => return Err(e.to_string()),
        };

        let canvas = match window.into_canvas().build() {
            Ok(c) => c,
            Err(e) => return Err(e.to_string()),
        };

        Ok(Context {
            canvas,
            _video_subsystem: video_subsystem,
            event_pump,
            _sdl: sdl,
            redraw_queued: true,
            should_quit: false,
        })
    }
}

fn handle_event(context: &mut Context, event: Event) {
    match event {
        Event::Quit {..} |
        Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
            context.should_quit = true;
        },
        _ => {}
    }
}

fn redraw(context: &mut Context) {
    context.redraw_queued = false;

    context.canvas.clear();
    context.canvas.present();
}

fn main_loop(context: &mut Context) {
    while !context.should_quit {
        if context.redraw_queued {
            while let Some(event) = context.event_pump.poll_event() {
                handle_event(context, event);
            }

            redraw(context);
        } else {
            let event = context.event_pump.wait_event();
            handle_event(context, event);
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

    main_loop(&mut context);

    ExitCode::SUCCESS
}
