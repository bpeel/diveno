mod data;

pub use data::N_LETTERS;
pub use data::N_COLORS;
pub use data::COLORS;

pub struct Color {
    pub letters: [Letter; N_LETTERS],
}

pub struct Letter {
    pub ch: char,
    pub s1: u16,
    pub t1: u16,
    pub s2: u16,
    pub t2: u16,
}
