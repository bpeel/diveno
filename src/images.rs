use std::rc::Rc;
use glow::HasContext;

pub struct Texture {
    id: glow::NativeTexture,
    gl: Rc<glow::Context>,
}

impl Texture {
    pub fn new(gl: Rc<glow::Context>, id: glow::NativeTexture) -> Texture {
        Texture { gl, id }
    }

    pub fn id(&self) -> glow::NativeTexture {
        self.id
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_texture(self.id);
        }
    }
}

pub struct ImageSet {
    pub letters: Texture,
}

impl ImageSet {
    pub fn new<F>(
        gl: &Rc<glow::Context>,
        mut load_image: F,
    ) -> Result<ImageSet, String>
        where F: FnMut(Rc<glow::Context>, &str) -> Result<Texture, String>
    {
        Ok(ImageSet {
            letters: load_image(Rc::clone(gl), "letters.png")?,
        })
    }
}
