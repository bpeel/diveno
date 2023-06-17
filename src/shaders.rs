use crate::gl;

use std::path::PathBuf;
use std::rc::Rc;

pub struct Shaders {
    pub test: gl::Program,
}

impl Shaders {
    pub fn new(gl: &Rc<gl::Gl>) -> Result<Shaders, String> {
        Ok(Shaders {
            test: create_program(
                Rc::clone(gl),
                "test-vertex.glsl",
                "test-fragment.glsl",
            )?,
        })
    }
}

fn create_shader(
    gl: Rc<gl::Gl>,
    shader_type: gl::Enum,
    filename: &str,
) -> Result<gl::Shader, String> {
    let path: PathBuf = ["data", filename].iter().collect();

    match std::fs::read_to_string(&path) {
        Err(e) => Err(format!("{}: {}", filename, e)),
        Ok(source) => gl::Shader::new(gl, shader_type, &source),
    }
}

fn create_program(
    gl: Rc<gl::Gl>,
    vertex_filename: &str,
    fragment_filename: &str,
) -> Result<gl::Program, String> {
    let shaders = [
        create_shader(Rc::clone(&gl), gl::VERTEX_SHADER, vertex_filename)?,
        create_shader(Rc::clone(&gl), gl::FRAGMENT_SHADER, fragment_filename)?,
    ];

    gl::Program::new(gl, &shaders)
}
