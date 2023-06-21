use std::rc::Rc;
use glow::HasContext;

pub struct Texture {
    id: glow::Texture,
    gl: Rc<glow::Context>,
}

impl Texture {
    fn new(gl: Rc<glow::Context>, id: glow::Texture) -> Texture {
        Texture { gl, id }
    }

    pub fn id(&self) -> glow::Texture {
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

const N_IMAGES: usize = 1;

static IMAGE_FILENAMES: [&'static str; N_IMAGES] = [
    "letters.png",
];

pub struct ImageLoader {
    gl: Rc::<glow::Context>,
    textures: [Option<Texture>; N_IMAGES],
    n_textures: usize,
}

impl ImageLoader {
    pub fn new(gl: Rc::<glow::Context>) -> ImageLoader {
        ImageLoader {
            gl,
            textures: Default::default(),
            n_textures: 0,
        }
    }

    pub fn next_filename(&self) -> Option<&'static str> {
        if self.n_textures < N_IMAGES {
            Some(IMAGE_FILENAMES[self.n_textures])
        } else {
            None
        }
    }

    pub fn loaded(&mut self, texture: glow::Texture) {
        assert!(self.n_textures < N_IMAGES);

        self.textures[self.n_textures] = Some(Texture::new(
            Rc::clone(&self.gl),
            texture,
        ));

        self.n_textures += 1;
    }

    pub fn complete(self) -> ImageSet {
        assert_eq!(self.n_textures, N_IMAGES);

        let [letters] = self.textures.map(|s| s.unwrap());

        ImageSet {
            letters,
        }
    }
}
