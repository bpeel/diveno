use std::ffi;
use std::mem::transmute;
use std::rc::Rc;

pub type Enum = ffi::c_uint;
pub type Bitfield = ffi::c_uint;
pub type Sizei = ffi::c_int;

pub const COLOR_BUFFER_BIT: Bitfield = 0x00004000;

pub const COMPILE_STATUS: Enum = 0x8B81;
pub const LINK_STATUS: Enum = 0x8B82;
pub const INFO_LOG_LENGTH: Enum = 0x8B84;
pub const VERTEX_SHADER: Enum = 0x8B31;
pub const FRAGMENT_SHADER: Enum = 0x8B30;
pub const TRIANGLE_STRIP: Enum = 0x0005;

pub struct Gl {
    pub clear_color: extern "C" fn(f32, f32, f32, f32),
    pub clear: extern "C" fn(Bitfield),
    pub create_shader: extern "C" fn(Enum) -> ffi::c_uint,
    pub compile_shader: extern "C" fn(ffi::c_uint),
    pub shader_source: extern "C" fn(
        ffi::c_uint,
        Sizei,
        *const *const ffi::c_char,
        *const ffi::c_int,
    ),
    pub get_shader_iv: extern "C" fn(
        ffi::c_uint,
        Enum,
        *mut ffi::c_int,
    ),
    pub get_shader_info_log: extern "C" fn(
        ffi::c_uint,
        Sizei,
        *const Sizei,
        *mut ffi::c_char,
    ),
    pub delete_shader: extern "C" fn(ffi::c_uint),
    pub create_program: extern "C" fn() -> ffi::c_uint,
    pub attach_shader: extern "C" fn(ffi::c_uint, ffi::c_uint),
    pub link_program: extern "C" fn(ffi::c_uint),
    pub get_program_iv: extern "C" fn(
        ffi::c_uint,
        Enum,
        *mut ffi::c_int,
    ),
    pub get_program_info_log: extern "C" fn(
        ffi::c_uint,
        Sizei,
        *const Sizei,
        *mut ffi::c_char,
    ),
    pub use_program: extern "C" fn(ffi::c_uint),
    pub delete_program: extern "C" fn(ffi::c_uint),
    pub draw_arrays: extern "C" fn(Enum, ffi::c_int, Sizei),
}

impl Gl {
    pub fn new<F>(mut get_proc_addr: F) -> Gl
        where F: FnMut(&str) -> *const ()
    {
        unsafe {
            Gl {
                clear_color: transmute(get_proc_addr("glClearColor")),
                clear: transmute(get_proc_addr("glClear")),
                create_shader: transmute(get_proc_addr("glCreateShader")),
                compile_shader: transmute(get_proc_addr("glCompileShader")),
                shader_source: transmute(get_proc_addr("glShaderSource")),
                get_shader_iv: transmute(get_proc_addr("glGetShaderiv")),
                get_shader_info_log: transmute(
                    get_proc_addr("glGetShaderInfoLog")
                ),
                delete_shader: transmute(get_proc_addr("glDeleteShader")),
                create_program: transmute(get_proc_addr("glCreateProgram")),
                attach_shader: transmute(get_proc_addr("glAttachShader")),
                link_program: transmute(get_proc_addr("glLinkProgram")),
                get_program_iv: transmute(get_proc_addr("glGetProgramiv")),
                get_program_info_log: transmute(
                    get_proc_addr("glGetProgramInfoLog")
                ),
                use_program: transmute(get_proc_addr("glUseProgram")),
                delete_program: transmute(get_proc_addr("glDeleteProgram")),
                draw_arrays: transmute(get_proc_addr("glDrawArrays")),
            }
        }
    }
}

pub struct Shader {
    id: ffi::c_uint,
    gl: Rc<Gl>,
}

impl Shader {
    pub fn new(
        gl: Rc<Gl>,
        shader_type: Enum,
        source: &str,
    ) -> Result<Shader, String> {
        let id = (gl.create_shader)(shader_type);

        if id == 0 {
            return Err("glCreateShader failed".to_string());
        }

        let shader = Shader { id, gl };

        shader.set_source(source);
        shader.compile()?;

        Ok(shader)
    }

    fn set_source(&self, source: &str) {
        let strings = [source.as_ptr() as *const ffi::c_char];
        let lengths = [source.len() as ffi::c_int];

        (self.gl.shader_source)(
            self.id,
            1,
            strings.as_ptr(),
            lengths.as_ptr(),
        );
    }

    fn integer_param(&self, param: Enum) -> ffi::c_int {
        let mut value: ffi::c_int = 0;

        (self.gl.get_shader_iv)(
            self.id,
            param,
            &mut value as *mut ffi::c_int,
        );

        value
    }

    fn compile(&self) -> Result<(), String> {
        (self.gl.compile_shader)(self.id);

        if self.integer_param(COMPILE_STATUS) == 0 {
            let mut log = self.info_log();

            if log.len() > 0 {
                log.push_str("\n\n");
            }

            log.push_str("Shader failed to compile");

            Err(log)
        } else {
            Ok(())
        }
    }

    fn info_log(&self) -> String {
        let max_length = self.integer_param(INFO_LOG_LENGTH);

        if max_length <= 0 {
            return String::new();
        }

        let mut raw_log = Vec::<u8>::with_capacity(max_length as usize + 1);

        let mut length: Sizei = 0;

        (self.gl.get_shader_info_log)(
            self.id,
            max_length as Sizei,
            &mut length as *mut Sizei,
            raw_log.as_mut_ptr() as *mut ffi::c_char,
        );

        unsafe {
            raw_log.set_len(length as usize);
        }

        String::from_utf8_lossy(&raw_log).to_string()
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        (self.gl.delete_shader)(self.id);
    }
}

pub struct Program {
    id: ffi::c_uint,
    gl: Rc<Gl>,
}

impl Program {
    pub fn new(gl: Rc<Gl>, shaders: &[Shader]) -> Result<Program, String> {
        let id = (gl.create_program)();

        if id == 0 {
            return Err("glCreateProgram failed".to_string());
        }

        let program = Program { id, gl };

        for shader in shaders.iter() {
            program.attach_shader(shader);
        }

        program.link()?;

        Ok(program)
    }

    pub fn id(&self) -> ffi::c_uint {
        self.id
    }

    fn attach_shader(&self, shader: &Shader) {
        (self.gl.attach_shader)(self.id, shader.id)
    }

    fn integer_param(&self, param: Enum) -> ffi::c_int {
        let mut value: ffi::c_int = 0;

        (self.gl.get_program_iv)(
            self.id,
            param,
            &mut value as *mut ffi::c_int,
        );

        value
    }

    fn link(&self) -> Result<(), String> {
        (self.gl.link_program)(self.id);

        if self.integer_param(LINK_STATUS) == 0 {
            let mut log = self.info_log();

            if log.len() > 0 {
                log.push_str("\n\n");
            }

            log.push_str("Program failed to link");

            Err(log)
        } else {
            Ok(())
        }
    }

    fn info_log(&self) -> String {
        let max_length = self.integer_param(INFO_LOG_LENGTH);

        if max_length <= 0 {
            return String::new();
        }

        let mut raw_log = Vec::<u8>::with_capacity(max_length as usize + 1);

        let mut length: Sizei = 0;

        (self.gl.get_program_info_log)(
            self.id,
            max_length as Sizei,
            &mut length as *mut Sizei,
            raw_log.as_mut_ptr() as *mut ffi::c_char,
        );

        unsafe {
            raw_log.set_len(length as usize);
        }

        String::from_utf8_lossy(&raw_log).to_string()
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        (self.gl.delete_program)(self.id);
    }
}
