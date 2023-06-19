use crate::letter_texture;

pub const N_GUESSES: usize = 6;

pub enum Event {
    GridChanged,
}

pub struct Logic {
    word: String,
    word_length: usize,
    in_progress_guess: String,
    grid_changed_queued: bool,
}

impl Logic {
    pub fn new() -> Logic {
        let mut logic = Logic {
            word: String::new(),
            word_length: 0,
            in_progress_guess: String::new(),
            grid_changed_queued: false,
        };

        logic.set_word("POTATO");

        logic
    }

    fn set_word(&mut self, word: &str) {
        self.word.clear();
        self.word.push_str(word);
        self.word_length = word.chars().count();
        self.in_progress_guess.clear();
        self.in_progress_guess.push(word.chars().next().unwrap());
        self.grid_changed_queued = true;
    }

    pub fn word_length(&self) -> usize {
        self.word_length
    }

    pub fn set_in_progress_guess(&mut self, guess: &str) {
        let first_letter = self.word.chars().next().unwrap();

        self.in_progress_guess.clear();
        self.in_progress_guess.push(first_letter);

        let mut added = 1;

        'add_loop: for ch in xsystem::unicode_chars(guess.chars()) {
            for ch in ch.to_uppercase() {
                if (added > 1 || ch != first_letter) && is_valid_letter(ch) {
                    self.in_progress_guess.push(ch);
                    added += 1;

                    if added >= self.word_length {
                        break 'add_loop;
                    }
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

fn is_valid_letter(letter: char) -> bool {
    let letters = &letter_texture::COLORS[0].letters;

    letters.binary_search_by(|probe| probe.ch.cmp(&letter)).is_ok()
}
