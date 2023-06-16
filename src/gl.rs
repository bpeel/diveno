use std::ffi;
use std::mem::transmute;

pub type Bitfield = ffi::c_uint;

pub const COLOR_BUFFER_BIT: Bitfield = 0x00004000;

pub struct Gl {
    pub clear_color: fn(f32, f32, f32, f32),
    pub clear: fn(Bitfield),
}

impl Gl {
    pub fn new<F>(mut get_proc_addr: F) -> Gl
        where F: FnMut(&str) -> *const ()
    {
        unsafe {
            Gl {
                clear_color: transmute(get_proc_addr("glClearColor")),
                clear: transmute(get_proc_addr("glClear")),
            }
        }
    }
}
