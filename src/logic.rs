use crate::letter_texture;

pub enum Event {
    GridChanged,
}

pub struct Logic {
    word: String,
    in_progress_guess: String,
    grid_changed_queued: bool,
}

impl Logic {
    pub fn new() -> Logic {
        Logic {
            word: "POTATO".to_string(),
            in_progress_guess: String::new(),
            grid_changed_queued: false,
        }
    }

    pub fn set_in_progress_guess(&mut self, guess: &str) {
        self.in_progress_guess.clear();

        let letters = &letter_texture::COLORS[0].letters;

        for ch in xsystem::unicode_chars(guess.chars()) {
            for ch in ch.to_uppercase() {
                if letters.binary_search_by(|probe| probe.ch.cmp(&ch)).is_ok() {
                    self.in_progress_guess.push(ch);
                }
            }
        }

        self.grid_changed_queued = true;
    }

    pub fn in_progress_guess(&self) -> &str {
        &self.in_progress_guess
    }

    pub fn get_event(&mut self) -> Option<Event> {
        if self.grid_changed_queued {
            self.grid_changed_queued = false;
            Some(Event::GridChanged)
        } else {
            None
        }
    }
}
