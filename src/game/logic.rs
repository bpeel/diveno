use std::collections::HashMap;
use super::letter_texture;

pub const N_GUESSES: usize = 6;

pub enum Event {
    WordChanged,
    GridChanged,
    GuessEntered,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum LetterResult {
    Correct,
    WrongPosition,
    Wrong,
}

pub struct Letter {
    pub letter: char,
    pub result: LetterResult,
}

pub struct Logic {
    word: String,
    word_length: usize,
    in_progress_guess: String,
    guesses: [Vec<Letter>; N_GUESSES],
    n_guesses: usize,
    word_changed_queued: bool,
    grid_changed_queued: bool,
    guess_entered_queued: bool,
    letter_counter: LetterCounter,
    // Bitmask of letters from the word that the player can see,
    // either because it was given as a hint or because they guessed
    // the right letter position.
    visible_letters: u32,
}

impl Logic {
    pub fn new() -> Logic {
        let mut logic = Logic {
            word: String::new(),
            word_length: 0,
            in_progress_guess: String::new(),
            guesses: Default::default(),
            n_guesses: 0,
            word_changed_queued: false,
            grid_changed_queued: false,
            guess_entered_queued: false,
            letter_counter: LetterCounter::new(),
            visible_letters: 1,
        };

        logic.set_word("POTATO");

        logic
    }

    fn set_word(&mut self, word: &str) {
        self.word.clear();
        self.word.push_str(word);
        self.word_length = word.chars().count();
        self.in_progress_guess.clear();
        self.word_changed_queued = true;
        self.grid_changed_queued = true;
        self.n_guesses = 0;
        self.visible_letters = 1;
    }

    pub fn word(&self) -> &str {
        &self.word
    }

    pub fn word_length(&self) -> usize {
        self.word_length
    }

    pub fn set_in_progress_guess(&mut self, guess: &str) {
        let first_letter = self.word.chars().next().unwrap();

        self.in_progress_guess.clear();

        let mut added = 0;

        'add_loop: for ch in xsystem::unicode_chars(guess.chars()) {
            for ch in ch.to_uppercase() {
                if is_valid_letter(ch) {
                    if added == 0 && ch != first_letter {
                        self.in_progress_guess.push(first_letter);
                        added += 1;
                    }

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
        if self.guess_entered_queued {
            self.guess_entered_queued = false;
            Some(Event::GuessEntered)
        } else if self.word_changed_queued {
            self.word_changed_queued = false;
            Some(Event::WordChanged)
        } else if self.grid_changed_queued {
            self.grid_changed_queued = false;
            Some(Event::GridChanged)
        } else {
            None
        }
    }

    pub fn enter_guess(&mut self) {
        if self.n_guesses >= N_GUESSES {
            return;
        }

        self.letter_counter.clear();

        let guess = &mut self.guesses[self.n_guesses];

        guess.clear();

        guess.extend(
            self.in_progress_guess
                .chars()
                .zip(self.word.chars())
                .map(|(letter, word_letter)| {
                    let result = if word_letter == letter {
                        LetterResult::Correct
                    } else {
                        self.letter_counter.push(word_letter);
                        LetterResult::Wrong
                    };

                    Letter { letter, result }
                })
        );

        if guess.len() != self.word_length {
            return;
        }

        // Add all of the correct guesses as visible letters
        for (index, &Letter { result, .. }) in guess.iter().enumerate() {
            if result == LetterResult::Correct {
                self.visible_letters |= 1 << index;
            }
        }

        for letter in guess.iter_mut() {
            if letter.result == LetterResult::Wrong
                && self.letter_counter.pop(letter.letter)
            {
                letter.result = LetterResult::WrongPosition;
            }
        }

        self.set_in_progress_guess("");

        self.n_guesses += 1;
        self.grid_changed_queued = true;
        self.guess_entered_queued = true;
    }

    pub fn visible_letters(&self) -> u32 {
        self.visible_letters
    }

    pub fn guesses(&self) -> GuessIter<'_> {
        GuessIter::new(self)
    }

    pub fn n_guesses(&self) -> usize {
        self.n_guesses
    }
}

pub struct GuessIter<'a> {
    iter: std::slice::Iter<'a, Vec<Letter>>,
}

impl<'a> Iterator for GuessIter<'a> {
    type Item = &'a [Letter];

    fn next(&mut self) -> Option<&'a [Letter]> {
        self.iter.next().map(Vec::as_slice)
    }
}

impl<'a> GuessIter<'a> {
    fn new(logic: &Logic) -> GuessIter {
        GuessIter {
            iter: logic.guesses[0..logic.n_guesses].iter()
        }
    }
}

fn is_valid_letter(letter: char) -> bool {
    let letters = &letter_texture::COLORS[0].letters;

    letters.binary_search_by(|probe| probe.ch.cmp(&letter)).is_ok()
}

struct LetterCounter {
    letters: HashMap<char, u32>,
}

impl LetterCounter {
    fn new() -> LetterCounter {
        LetterCounter {
            letters: HashMap::new()
        }
    }

    fn clear(&mut self) {
        self.letters.clear();
    }

    fn push(&mut self, letter: char) {
        self.letters.entry(letter).and_modify(|count| *count += 1).or_insert(1);
    }

    fn pop(&mut self, letter: char) -> bool {
        if let Some(count) = self.letters.get_mut(&letter) {
            *count -= 1;

            if *count <= 0 {
                self.letters.remove(&letter);
            }

            true
        } else {
            false
        }
    }
}
