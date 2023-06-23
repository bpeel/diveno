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

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::console;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use glow::HasContext;

#[cfg(target_arch = "wasm32")]
mod game;

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
struct Context {
    gl: Rc<glow::Context>,
    canvas: web_sys::HtmlCanvasElement,
}

#[cfg(target_arch = "wasm32")]
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
            canvas,
            gl,
        })
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct Diveno {
    context: Option<Context>,
    shader_loader: Option<game::shaders::ShaderLoader>,
    logic_loader: Option<game::logic::LogicLoader>,
    shaders: Option<game::shaders::Shaders>,
    image_loader: Option<game::images::ImageLoader>,
    images: Option<game::images::ImageSet>,
    logic: Option<game::logic::Logic>,
    game_painter: Option<game::game_painter::GamePainter>,
    width: u32,
    height: u32,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl Diveno {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Diveno {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));

        let (context, shader_loader, image_loader) = match Context::new() {
            Ok(context) => {
                let shader_loader = game::shaders::ShaderLoader::new(
                    Rc::clone(&context.gl),
                );

                let image_loader = game::images::ImageLoader::new(
                    Rc::clone(&context.gl),
                );

                (Some(context), Some(shader_loader), Some(image_loader))
            },
            Err(e) => {
                show_error(&e.to_string());
                (None, None, None)
            },
        };

        Diveno {
            context,
            shader_loader,
            logic_loader: Some(game::logic::LogicLoader::new()),
            shaders: None,
            image_loader,
            images: None,
            logic: None,
            game_painter: None,
            width: 1,
            height: 1,
        }
    }

    pub fn next_data_filename(&self) -> Option<String> {
        self.shader_loader
            .as_ref()
            .and_then(|s| s.next_filename().map(str::to_string))
            .or_else(|| {
                self.logic_loader
                    .as_ref()
                    .and_then(|s| s.next_filename().map(str::to_string))
            })
    }

    pub fn data_loaded(&mut self, contents: &[u8]) {
        match self.shader_loader.as_mut() {
            Some(shader_loader) => {
                if let Err(e) = shader_loader.loaded(contents) {
                    show_error(&e);
                    return;
                }

                if shader_loader.next_filename().is_none() {
                    match self.shader_loader.take().unwrap().complete() {
                        Ok(shaders) => self.shaders = Some(shaders),
                        Err(e) => show_error(&e),
                    }
                }
            },
            None => if let Some(logic_loader) = self.logic_loader.as_mut() {
                logic_loader.loaded(Box::from(contents));

                if logic_loader.next_filename().is_none() {
                    self.logic = Some(
                        self.logic_loader
                            .take()
                            .unwrap()
                            .complete()
                    );
                    self.maybe_start_game();
                }
            },
        }
    }

    pub fn next_image_filename(&self) -> Option<String> {
        let Some(image_loader) = self.image_loader.as_ref()
        else {
            return None;
        };

        image_loader.next_filename().map(str::to_string)
    }

    pub fn image_loaded(&mut self, image: web_sys::HtmlImageElement) {
        let Some(image_loader) = self.image_loader.as_mut()
        else {
            return;
        };

        let Some(ref context) = self.context
        else {
            return;
        };

        let gl = &context.gl;

        let texture = unsafe {
            match gl.create_texture() {
                Ok(t) => t,
                Err(e) => {
                    show_error(&e);
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

        image_loader.loaded(texture);

        if image_loader.next_filename().is_none() {
            self.images = Some(self.image_loader.take().unwrap().complete());
            self.maybe_start_game();
        }
    }

    fn maybe_start_game(&mut self) {
        if self.shaders.is_some()
            && self.images.is_some()
            && self.logic.is_some()
        {
            let shaders = self.shaders.take().unwrap();
            let images = self.images.take().unwrap();

            self.start_game(shaders, images);
        }
    }

    fn start_game(
        &mut self,
        shaders: game::shaders::Shaders,
        images: game::images::ImageSet,
    ) {
        let gl = if let Some(ref context) = self.context {
            &context.gl
        } else {
            return;
        };

        let has_vertex_array_object =
            gl.supported_extensions().contains("OES_vertex_array_object");

        let paint_data = Rc::new(game::paint_data::PaintData::new(
            Rc::clone(gl),
            has_vertex_array_object,
            shaders,
            images,
        ));

        match game::game_painter::GamePainter::new(paint_data) {
            Ok(mut painter) => {
                painter.update_fb_size(self.width, self.height);
                self.game_painter.replace(painter);
            },
            Err(e) => show_error(&e),
        }
    }

    pub fn update_fb_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        if let Some(ref mut game_painter) = self.game_painter {
            game_painter.update_fb_size(width, height);
        }
    }

    fn flush_logic_events(&mut self) -> bool {
        let mut redraw_queued = false;

        while let Some(event) = self.logic
            .as_mut()
            .and_then(|l| l.get_event())
        {
            match event {
                game::logic::Event::Solved |
                game::logic::Event::GuessEntered |
                game::logic::Event::WrongGuessEntered |
                game::logic::Event::WordChanged |
                game::logic::Event::GridChanged => {
                    redraw_queued = true;
                },
            }

            if let Some(ref mut game_painter) = self.game_painter {
                game_painter.handle_logic_event(&event);
            }
        }

        redraw_queued
    }

    pub fn redraw(&mut self) -> bool {
        let mut redraw_queued = self.flush_logic_events();

        if let Some(ref mut game_painter) = self.game_painter {
            if let Some(ref logic) = self.logic {
                redraw_queued |= game_painter.paint(logic);
            }
        }

        redraw_queued
    }

    pub fn is_ready(&self) -> bool {
        self.game_painter.is_some()
    }

    fn press_key(&mut self, key: game::logic::Key) -> bool {
        if let Some(ref mut logic) = self.logic {
            logic.press_key(key);
            self.flush_logic_events()
        } else {
            false
        }
    }

    pub fn press_enter(&mut self) -> bool {
        self.press_key(game::logic::Key::Enter)
    }

    pub fn press_backspace(&mut self) -> bool {
        self.press_key(game::logic::Key::Backspace)
    }

    pub fn press_dead_key(&mut self) -> bool {
        self.press_key(game::logic::Key::Dead)
    }

    pub fn press_letter_key(&mut self, letter: char) -> bool {
        self.press_key(game::logic::Key::Letter(letter))
    }
}
