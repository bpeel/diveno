use std::rc::Rc;
use glow::HasContext;

pub struct Buffer {
    gl: Rc<glow::Context>,
    id: glow::Buffer,
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_buffer(self.id);
        }
    }
}

impl Buffer {
    pub fn new(gl: Rc<glow::Context>) -> Result<Buffer, String> {
        let id = unsafe {
            gl.create_buffer()?
        };

        Ok(Buffer { id, gl })
    }

    pub fn id(&self) -> glow::Buffer {
        self.id
    }
}
