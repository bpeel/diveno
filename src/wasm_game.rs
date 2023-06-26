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

use wasm_bindgen::prelude::*;
use web_sys::console;
use std::rc::Rc;
use glow::HasContext;
use super::game;
use game::images::ImageLoader;
use game::logic::{LogicLoader, Logic};
use game::shaders::ShaderLoader;
use game::paint_data::PaintData;
use game::game_painter::GamePainter;

fn show_error(message: &str) {
    console::log_1(&message.into());

    let Some(window) = web_sys::window()
    else {
        return;
    };

    let Some(document) = window.document()
    else {
        return;
    };

    let Some(message_elem) = document.get_element_by_id("message")
    else {
        return;
    };

    message_elem.set_text_content(Some("Eraro okazis"));
}

struct Context {
    gl: Rc<glow::Context>,
    canvas: web_sys::HtmlCanvasElement,
    document: web_sys::Document,
    window: web_sys::Window,
}

impl Context {
    fn new() -> Result<Context, String> {
        let Some(window) = web_sys::window()
        else {
            return Err("failed to get window".to_string());
        };

        let Some(document) = window.document()
        else {
            return Err("failed to get document".to_string());
        };

        let Some(canvas) = document.get_element_by_id("canvas")
            .and_then(|c| c.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        else {
            return Err("failed to get canvas element".to_string());
        };

        let Some(context) = canvas.get_context_with_context_options(
            "webgl",
            web_sys::WebGlContextAttributes::new().alpha(false),
        )
            .unwrap_or(None)
            .and_then(|c| c.dyn_into::<web_sys::WebGlRenderingContext>().ok())
        else {
            return Err("error getting WebGL context".to_string());
        };

        let gl = Rc::new(glow::Context::from_webgl1_context(context));

        Ok(Context {
            gl,
            canvas,
            document,
            window,
        })
    }
}

type PromiseClosure = Closure::<dyn FnMut(JsValue)>;

struct Loader {
    context: Context,

    image_loader: ImageLoader,
    image_load_closure: Option<Closure::<dyn Fn()>>,
    image_error_closure: Option<Closure::<dyn Fn()>>,

    logic_loader: LogicLoader,
    shader_loader: ShaderLoader,

    data_response_closure: Option<PromiseClosure>,
    data_content_closure: Option<PromiseClosure>,
    data_error_closure: Option<PromiseClosure>,

    floating_pointer: Option<*mut Loader>,
}

impl Loader {
    fn new(context: Context) -> Loader {
        let image_loader = ImageLoader::new(Rc::clone(&context.gl));
        let logic_loader = LogicLoader::new();
        let shader_loader = ShaderLoader::new(Rc::clone(&context.gl));

        Loader {
            context,
            image_loader,
            logic_loader,
            shader_loader,
            image_load_closure: None,
            image_error_closure: None,
            data_response_closure: None,
            data_content_closure: None,
            data_error_closure: None,
            floating_pointer: None,
        }
    }

    fn start_floating(self) -> *mut Loader {
        assert!(self.floating_pointer.is_none());

        let floating_pointer = Box::into_raw(Box::new(self));

        unsafe {
            (*floating_pointer).floating_pointer = Some(floating_pointer);
        }

        floating_pointer
    }

    fn stop_floating(&mut self) -> Loader {
        match self.floating_pointer {
            Some(floating_pointer) => unsafe {
                // This should end up destroying the loader and
                // invalidating any closures that it holds
                *Box::from_raw(floating_pointer)
            },
            None => unreachable!(),
        }
    }

    fn queue_image_load(&mut self) {
        let Some(filename) = self.image_loader.next_filename()
        else {
            return;
        };

        let floating_pointer = self.floating_pointer.unwrap();

        let Ok(image) = web_sys::HtmlImageElement::new()
        else {
            show_error("Error creating image element");
            self.stop_floating();
            return;
        };

        let image = Rc::new(image);
        let closure_image = Rc::clone(&image);

        let load_closure = Closure::<dyn Fn()>::new(move || {
            unsafe {
                (*floating_pointer).image_loaded(&closure_image);
            }
        });

        let error_closure = Closure::<dyn Fn()>::new(move || {
            show_error("Error loading image");
            unsafe {
                (*floating_pointer).stop_floating();
            }
        });

        image.set_onload(Some(load_closure.as_ref().unchecked_ref()));
        image.set_onerror(Some(error_closure.as_ref().unchecked_ref()));

        self.image_load_closure = Some(load_closure);
        self.image_error_closure = Some(error_closure);

        image.set_src(&format!("data/{}", filename));
    }

    fn image_loaded(&mut self, image: &web_sys::HtmlImageElement) {
        let gl = &self.context.gl;

        let texture = unsafe {
            match gl.create_texture() {
                Ok(t) => t,
                Err(e) => {
                    show_error(&e);
                    self.stop_floating();
                    return;
                }
            }
        };

        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
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
            gl.tex_image_2d_with_html_image(
                glow::TEXTURE_2D,
                0, // level
                glow::RGBA as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                &image,
            );
            gl.generate_mipmap(glow::TEXTURE_2D);
        }

        self.image_loader.loaded(texture);

        if self.image_loader.next_filename().is_none() {
            self.maybe_start_game();
        } else {
            self.queue_image_load();
        }
    }

    fn next_data_filename(&self) -> Option<&'static str> {
        self.logic_loader.next_filename()
            .or_else(|| self.shader_loader.next_filename())
    }

    fn queue_data_load(&mut self) {
        let Some(filename) = self.next_data_filename()
        else {
            return;
        };

        let floating_pointer = self.floating_pointer.unwrap();

        let response_closure = PromiseClosure::new(move |v: JsValue| {
            let (content_closure, error_closure) = unsafe {
                (
                    (*floating_pointer).data_content_closure.as_ref().unwrap(),
                    (*floating_pointer).data_error_closure.as_ref().unwrap(),
                )
            };

            let response: web_sys::Response = v.dyn_into().unwrap();
            let promise = match response.array_buffer() {
                Ok(p) => p,
                Err(_) => {
                    show_error("Error fetching array buffer from data");
                    unsafe {
                        (*floating_pointer).stop_floating();
                    }
                    return;
                },
            };
            let _ = promise.then2(content_closure, error_closure);
        });

        let content_closure = PromiseClosure::new(move |v| {
            let data = js_sys::Uint8Array::new(&v).to_vec();

            unsafe {
                (*floating_pointer).data_loaded(data);
            }
        });

        let error_closure = PromiseClosure::new(move |_| {
            show_error("Error loading data");
            unsafe {
                (*floating_pointer).stop_floating();
            }
        });

        let promise = self.context.window.fetch_with_str(
            &format!("data/{}", filename)
        );

        let _ = promise.then2(&response_closure, &error_closure);

        self.data_response_closure = Some(response_closure);
        self.data_content_closure = Some(content_closure);
        self.data_error_closure = Some(error_closure);
    }

    fn data_loaded(&mut self, data: Vec<u8>) {
        if self.logic_loader.next_filename().is_some() {
            self.logic_loader.loaded(data.into_boxed_slice());
        } else if let Err(e) = self.shader_loader.loaded(&data) {
            show_error(&e);
            self.stop_floating();
            return;
        }

        if self.next_data_filename().is_none() {
            self.maybe_start_game();
        } else {
            self.queue_data_load();
        }
    }

    fn maybe_start_game(&mut self) {
        if self.next_data_filename().is_some()
            || self.image_loader.next_filename().is_some()
        {
            return;
        }

        let Loader { context, image_loader, logic_loader, shader_loader, .. } =
            self.stop_floating();

        let images = image_loader.complete();
        let shaders = match shader_loader.complete() {
            Ok(s) => s,
            Err(e) => {
                show_error(&e);
                return;
            },
        };

        let has_vertex_array_object =
            context.gl
            .supported_extensions()
            .contains("OES_vertex_array_object");

        let paint_data = Rc::new(PaintData::new(
            Rc::clone(&context.gl),
            has_vertex_array_object,
            shaders,
            images,
        ));

        let logic = logic_loader.complete();

        match GamePainter::new(paint_data) {
            Ok(painter) => {
                let _ = context.canvas.style().set_property("display", "block");
                let diveno = Diveno::new(context, painter, logic);
                // Leak the main diveno object so that it will live as
                // long as the web page
                std::mem::forget(diveno);
            },
            Err(e) => show_error(&e),
        }
    }
}

struct Diveno {
    context: Context,
    painter: GamePainter,
    logic: Logic,

    animation_frame_handle: Option<i32>,
    redraw_closure: Option<Closure<dyn Fn()>>,

    resize_closure: Option<Closure<dyn Fn()>>,

    keydown_closure: Option<Closure::<dyn Fn(JsValue)>>,
}

impl Diveno {
    fn new(
        context: Context,
        painter: GamePainter,
        logic: Logic
    ) -> Box<Diveno> {
        let mut diveno = Box::new(Diveno {
            context,
            painter,
            logic,
            animation_frame_handle: None,
            redraw_closure: None,
            resize_closure: None,
            keydown_closure: None,
        });

        let diveno_pointer = diveno.as_mut() as *mut Diveno;

        let resize_closure = Closure::<dyn Fn()>::new(move || {
            let diveno = unsafe { &mut *diveno_pointer };
            diveno.handle_size_change();
        });

        diveno.context.window.set_onresize(
            Some(resize_closure.as_ref().unchecked_ref())
        );

        diveno.resize_closure = Some(resize_closure);

        diveno.handle_size_change();

        let keydown_closure = Closure::<dyn Fn(JsValue)>::new(
            move |event: JsValue| {
                let diveno = unsafe { &mut *diveno_pointer };
                let event: web_sys::KeyboardEvent = event.dyn_into().unwrap();
                diveno.handle_key_event(event);
            }
        );

        let _ = diveno.context.document.add_event_listener_with_callback(
            "keydown",
            keydown_closure.as_ref().unchecked_ref(),
        );

        diveno.keydown_closure = Some(keydown_closure);

        diveno
    }

    fn flush_logic_events(&mut self) -> bool {
        let mut redraw_queued = false;

        while let Some(event) = self.logic.get_event() {
            match event {
                game::logic::Event::Solved |
                game::logic::Event::GuessEntered |
                game::logic::Event::WrongGuessEntered |
                game::logic::Event::WordChanged |
                game::logic::Event::GridChanged => {
                    redraw_queued = true;
                },
            }

            self.painter.handle_logic_event(&event);
        }

        redraw_queued
    }

    fn redraw(&mut self) -> bool {
        let mut redraw_queued = self.flush_logic_events();

        redraw_queued |= self.painter.paint(&self.logic);

        redraw_queued
    }

    fn queue_redraw(&mut self) {
        if self.animation_frame_handle.is_some() {
            return;
        }

        let diveno_pointer = self as *mut Diveno;

        let redraw_closure = self.redraw_closure.get_or_insert_with(|| {
            Closure::<dyn Fn()>::new(move || {
                let diveno = unsafe { &mut *diveno_pointer };
                diveno.animation_frame_handle = None;

                if diveno.redraw() {
                    diveno.queue_redraw();
                }
            })
        });

        match self.context.window.request_animation_frame(
            redraw_closure.as_ref().unchecked_ref()
        ) {
            Ok(handle) => self.animation_frame_handle = Some(handle),
            Err(_) => {
                console::log_1(&"Error requesting animation frame".into());
            },
        }
    }

    fn handle_size_change(&mut self) {
        let rect = self.context.canvas.get_bounding_client_rect();

        let width = rect.width() as u32;
        let height = rect.height() as u32;

        self.context.canvas.set_width(width);
        self.context.canvas.set_height(height);

        self.painter.update_fb_size(width, height);

        self.queue_redraw();
    }

    fn handle_key_event(&mut self, event: web_sys::KeyboardEvent) {
        let key = match event.key().as_str() {
            "Enter" => game::logic::Key::Enter,
            "Backspace" => game::logic::Key::Backspace,
            "Dead" => game::logic::Key::Dead,
            s => {
                let mut chars = s.chars();

                let ch = chars.next().and_then(|ch| {
                    if chars.next().is_none() && ch.is_alphabetic() {
                        Some(ch)
                    } else {
                        None
                    }
                });

                match ch {
                    Some(ch) => game::logic::Key::Letter(ch),
                    None => return,
                }
            },
        };

        self.logic.press_key(key);

        if self.flush_logic_events() {
            self.queue_redraw();
        }
    }
}

#[wasm_bindgen]
pub fn init_diveno() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let context = match Context::new() {
        Ok(c) => c,
        Err(e) => {
            show_error(&e);
            return;
        }
    };

    let loader = Loader::new(context);

    let floating_pointer = loader.start_floating();

    unsafe {
        (*floating_pointer).queue_image_load();
        (*floating_pointer).queue_data_load();
    }
}
