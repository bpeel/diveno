use std::rc::Rc;
use glow::HasContext;

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

        Ok(program)
    }

    pub fn id(&self) -> glow::NativeProgram {
        self.id
    }

    fn link(&self) -> Result<(), String> {
        unsafe {
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
