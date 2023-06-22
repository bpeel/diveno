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

use std::rc::Rc;
use glow::HasContext;

pub const POSITION_ATTRIB: u32 = 0;
pub const TEX_COORD_ATTRIB: u32 = 1;
pub const COLOR_ATTRIB: u32 = 2;
pub const NORMAL_ATTRIB: u32 = 3;

pub struct Shader {
    id: glow::Shader,
    gl: Rc<glow::Context>,
}

impl Shader {
    pub fn new(
        gl: Rc<glow::Context>,
        shader_type: u32,
        source: &str,
    ) -> Result<Shader, String> {
        let shader = unsafe {
            let id = gl.create_shader(shader_type)?;

            gl.shader_source(id, source);

            Shader { id, gl }
        };

        shader.compile()?;

        Ok(shader)
    }

    fn compile(&self) -> Result<(), String> {
        unsafe {
            self.gl.compile_shader(self.id);
        }

        if unsafe { self.gl.get_shader_compile_status(self.id) } {
            Ok(())
        } else {
            let mut log = unsafe {
                self.gl.get_shader_info_log(self.id)
            };

            if log.len() > 0 {
                log.push_str("\n\n");
            }

            log.push_str("Shader failed to compile");

            Err(log)
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_shader(self.id);
        }
    }
}

pub struct Program {
    id: glow::Program,
    gl: Rc<glow::Context>,
}

impl Program {
    pub fn new(
        gl: Rc<glow::Context>,
        shaders: &[Shader],
    ) -> Result<Program, String> {
        let program = unsafe {
            let id = gl.create_program()?;

            for shader in shaders.iter() {
                gl.attach_shader(id, shader.id);
            }

            Program { id, gl }
        };

        program.link()?;

        unsafe {
            if let Some(location) = program.gl.get_uniform_location(
                program.id,
                "tex",
            ) {
                program.gl.use_program(Some(program.id));
                program.gl.uniform_1_i32(Some(&location), 0);
            }
        }

        Ok(program)
    }

    pub fn id(&self) -> glow::Program {
        self.id
    }

    fn link(&self) -> Result<(), String> {
        unsafe {
            self.gl.bind_attrib_location(
                self.id,
                POSITION_ATTRIB,
                "position",
            );
            self.gl.bind_attrib_location(
                self.id,
                TEX_COORD_ATTRIB,
                "tex_coord_attrib",
            );
            self.gl.bind_attrib_location(
                self.id,
                NORMAL_ATTRIB,
                "normal_attrib",
            );
            self.gl.bind_attrib_location(
                self.id,
                COLOR_ATTRIB,
                "color_attrib",
            );

            self.gl.link_program(self.id);
        }

        if unsafe { self.gl.get_program_link_status(self.id) } {
            Ok(())
        } else {
            let mut log = unsafe {
                self.gl.get_program_info_log(self.id)
            };

            if log.len() > 0 {
                log.push_str("\n\n");
            }

            log.push_str("Program failed to link");

            Err(log)
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_program(self.id);
        }
    }
}

pub struct Shaders {
    pub letter: Program,
}

struct ShaderFile {
    name: &'static str,
    shader_type: u32,
}

const N_SHADER_FILES: usize = 2;

static SHADER_FILES: [ShaderFile; N_SHADER_FILES] = [
    ShaderFile {
        name: "letter-vertex.glsl",
        shader_type: glow::VERTEX_SHADER
    },
    ShaderFile {
        name: "letter-fragment.glsl",
        shader_type:
        glow::FRAGMENT_SHADER
    },
];

pub struct ShaderLoader {
    gl: Rc<glow::Context>,
    shaders: [Option<Shader>; N_SHADER_FILES],
    n_shaders: usize,
}

impl ShaderLoader {
    pub fn new(gl: Rc<glow::Context>) -> ShaderLoader {
        ShaderLoader {
            gl,
            shaders: Default::default(),
            n_shaders: 0,
        }
    }

    pub fn next_filename(&self) -> Option<&'static str> {
        if self.n_shaders < N_SHADER_FILES {
            Some(SHADER_FILES[self.n_shaders].name)
        } else {
            None
        }
    }

    pub fn loaded(&mut self, source: &[u8]) -> Result<(), String> {
        assert!(self.n_shaders < N_SHADER_FILES);

        let Ok(source) = std::str::from_utf8(source)
        else {
            return Err("Invalid UTF-8 in shader source".to_string())
        };

        self.shaders[self.n_shaders] = Some(Shader::new(
            Rc::clone(&self.gl),
            SHADER_FILES[self.n_shaders].shader_type,
            source,
        )?);

        self.n_shaders += 1;

        Ok(())
    }

    pub fn complete(self) -> Result<Shaders, String> {
        assert_eq!(self.n_shaders, N_SHADER_FILES);

        let shaders = self.shaders.map(|s| s.unwrap());

        let letter = Program::new(
            Rc::clone(&self.gl),
            &shaders[0..2],
        )?;

        Ok(Shaders {
            letter
        })
    }
}
