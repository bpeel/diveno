// Diveno – A word game in Esperanto
// Copyright (C) 2023  Neil Roberts
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::collections::HashMap;
use super::letter_texture;
use super::dictionary::Dictionary;

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

#[derive(Copy, Clone)]
pub enum Key {
    Dead,
    Backspace,
    Enter,
    Letter(char),
}

pub struct Letter {
    pub letter: char,
    pub result: LetterResult,
}

static HATABLE_LETTERS: [(char, char); 12] = [
    ('C', 'Ĉ'),
    ('G', 'Ĝ'),
    ('H', 'Ĥ'),
    ('J', 'Ĵ'),
    ('S', 'Ŝ'),
    ('U', 'Ŭ'),
    ('c', 'ĉ'),
    ('g', 'ĝ'),
    ('h', 'ĥ'),
    ('j', 'ĵ'),
    ('s', 'ŝ'),
    ('u', 'ŭ'),
];

pub struct Logic {
    dictionary: Dictionary,
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
    dead_key_queued: bool,
}

impl Logic {
    pub fn new(dictionary: Dictionary) -> Logic {
        let mut logic = Logic {
            dictionary,
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
            dead_key_queued: false,
        };

        logic.set_word("TERPOMO");

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
        self.dead_key_queued = false;
    }

    pub fn word(&self) -> &str {
        &self.word
    }

    pub fn word_length(&self) -> usize {
        self.word_length
    }

    pub fn press_key(&mut self, key: Key) {
        match key {
            Key::Letter(mut letter) => {
                if letter == 'x' || letter == 'X' {
                    self.hatify_last_letter();
                } else {
                    if self.dead_key_queued {
                        letter = hatify(letter).unwrap_or(letter);
                        self.dead_key_queued = false;
                    }

                    self.add_letter(letter);
                }
            },
            Key::Dead => self.dead_key_queued = true,
            Key::Enter => {
                self.dead_key_queued = false;
                self.enter_guess();
            },
            Key::Backspace => {
                self.dead_key_queued = false;
                self.remove_letter();
            },
        }
    }

    fn hatify_last_letter(&mut self) {
        let mut last_letters = self.in_progress_guess.chars().rev();

        let Some(letter) = last_letters.next()
        else {
            return;
        };

        // Don’t hatify the first letter
        if last_letters.next().is_none() {
            return;
        }

        if let Some(hatted) = hatify(letter) {
            self.in_progress_guess.truncate(
                self.in_progress_guess.len() - letter.len_utf8()
            );
            self.in_progress_guess.push(hatted);
            self.grid_changed_queued = true;
        }
    }

    fn remove_letter(&mut self) {
        if let Some(letter) = self.in_progress_guess.chars().rev().next() {
            self.in_progress_guess.truncate(
                self.in_progress_guess.len() - letter.len_utf8()
            );
            self.grid_changed_queued = true;
        }
    }

    fn add_letter(&mut self, letter: char) {
        let mut guess_length = self.in_progress_guess.chars().count();
        let first_letter = self.word.chars().next().unwrap();

        for ch in letter.to_uppercase() {
            if guess_length >= self.word_length {
                break;
            }

            if is_valid_letter(ch) {
                if guess_length == 0 && ch != first_letter {
                    self.in_progress_guess.push(first_letter);
                    guess_length += 1;
                }

                self.in_progress_guess.push(ch);
                guess_length += 1;
                self.grid_changed_queued = true;
            }
        }
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

    fn enter_guess(&mut self) {
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

        if !self.dictionary.contains(&self.in_progress_guess) {
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

        self.in_progress_guess.clear();

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

fn hatify(letter: char) -> Option<char> {
    match HATABLE_LETTERS.binary_search_by(|probe| probe.0.cmp(&letter)) {
        Ok(index) => Some(HATABLE_LETTERS[index].1),
        Err(_) => None,
    }
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
