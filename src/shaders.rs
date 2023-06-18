use std::path::PathBuf;
use std::rc::Rc;
use glow::HasContext;

pub const POSITION_ATTRIB: u32 = 0;
pub const TEX_COORD_ATTRIB: u32 = 1;
pub const COLOR_ATTRIB: u32 = 2;
pub const NORMAL_ATTRIB: u32 = 3;

pub struct Shader {
    id: glow::NativeShader,
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
    id: glow::NativeProgram,
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
                program.gl.uniform_1_u32(Some(&location), 0);
            }
        }

        Ok(program)
    }

    pub fn id(&self) -> glow::NativeProgram {
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

impl Shaders {
    pub fn new(gl: &Rc<glow::Context>) -> Result<Shaders, String> {
        Ok(Shaders {
            letter: create_program(
                Rc::clone(gl),
                "letter-vertex.glsl",
                "letter-fragment.glsl",
            )?,
        })
    }
}

fn create_shader(
    gl: Rc<glow::Context>,
    shader_type: u32,
    filename: &str,
) -> Result<Shader, String> {
    let path: PathBuf = ["data", filename].iter().collect();

    match std::fs::read_to_string(&path) {
        Err(e) => Err(format!("{}: {}", filename, e)),
        Ok(source) => Shader::new(gl, shader_type, &source),
    }
}

fn create_program(
    gl: Rc<glow::Context>,
    vertex_filename: &str,
    fragment_filename: &str,
) -> Result<Program, String> {
    let shaders = [
        create_shader(
            Rc::clone(&gl),
            glow::VERTEX_SHADER,
            vertex_filename)?,
        create_shader(
            Rc::clone(&gl),
            glow::FRAGMENT_SHADER,
            fragment_filename
        )?,
    ];

    Program::new(gl, &shaders)
}
