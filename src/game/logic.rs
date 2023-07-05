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
use std::collections::VecDeque;
use super::{letter_texture, random, tombola, bingo_grid};
use super::dictionary::Dictionary;
use tombola::Tombola;
use bingo_grid::BingoGrid;

pub const N_GUESSES: usize = 6;

const N_NUMBER_BALLS: usize = bingo_grid::N_SPACES
    - bingo_grid::N_INITIAL_SPACES_COVERED;
const N_BLACK_BALLS: usize = 3;
const N_BALLS: usize = N_NUMBER_BALLS + N_BLACK_BALLS;

#[derive(PartialEq, Eq)]
pub enum Event {
    WordChanged,
    GridChanged,
    GuessEntered,
    WrongGuessEntered,
    GuessRejected,
    Solved,
    ScoreChanged(Team),
    CurrentTeamChanged,
    CurrentPageChanged(Page),
    TombolaStartedSpinning(Team),
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum LetterResult {
    Correct,
    WrongPosition,
    Wrong,
    // Used when a guess is rejected, for example when the word is not
    // in the dictionary
    Rejected,
}

#[derive(Copy, Clone)]
pub enum Key {
    Dead,
    Backspace,
    Delete,
    Enter,
    PageDown,
    Space,
    Home,
    Letter(char),
    Left,
    Right,
    Up,
    Down,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Page {
    Bingo(Team),
    Word,
}

impl Page {
    pub fn position(&self) -> usize {
        match self {
            Page::Bingo(Team::Left) => 0,
            Page::Word => 1,
            Page::Bingo(Team::Right) => 2,
        }
    }
}

pub struct Letter {
    pub letter: char,
    pub result: LetterResult,
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum Team {
    Left,
    Right,
}

pub enum BallType {
    Number(u32),
    Black,
}

pub struct Ball {
    pub ball_type: BallType,
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
}

pub const N_TEAMS: usize = 2;

const MAX_SCORE: u32 = 990;

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
    current_page: Page,
    word_list: Box<[u64]>,
    word: String,
    word_length: usize,
    in_progress_guess: String,
    guesses: [Vec<Letter>; N_GUESSES],
    n_guesses: usize,
    scores: [u32; N_TEAMS],
    tombolas: [Tombola; N_TEAMS],
    bingo_grids: [BingoGrid; N_TEAMS],
    current_team: Team,
    event_queue: VecDeque<Event>,
    letter_counter: LetterCounter,
    // Bitmask of letters from the word that the player can see,
    // either because it was given as a hint or because they guessed
    // the right letter position.
    visible_letters: u32,
    dead_key_queued: bool,
    is_solved: bool,
}

impl Logic {
    fn new(dictionary: Dictionary, word_list: Box<[u64]>) -> Logic {
        let mut logic = Logic {
            dictionary,
            current_page: Page::Word,
            word_list,
            word: String::new(),
            word_length: 0,
            in_progress_guess: String::new(),
            guesses: Default::default(),
            n_guesses: 0,
            scores: Default::default(),
            tombolas: [Tombola::new(N_BALLS), Tombola::new(N_BALLS)],
            bingo_grids: Default::default(),
            current_team: Team::Left,
            event_queue: VecDeque::new(),
            letter_counter: LetterCounter::new(),
            visible_letters: 1,
            dead_key_queued: false,
            is_solved: false,
        };

        logic.pick_word();

        for bingo_grid in logic.bingo_grids.iter_mut() {
            bingo_grid.reset();
        }

        logic
    }

    fn pick_word(&mut self) {
        if !self.word_list.is_empty() {
            let word_num = random::random_range(self.word_list.len());
            let word = self.word_list[word_num];

            if let Some(word) = self.dictionary.extract_word(word) {
                self.set_word(&word);
                return;
            }
        }

        self.set_word("eraro");
    }

    fn set_word(&mut self, word: &str) {
        let mut word_length = 0;

        self.word.clear();
        self.word.extend(
            word
                .chars()
                .flat_map(char::to_uppercase)
                .filter(|&c| {
                    if is_valid_letter(c) {
                        word_length += 1;
                        true
                    } else {
                        false
                    }
                })
        );

        self.word_length = word_length;

        self.in_progress_guess.clear();
        self.queue_event_once(Event::WordChanged);
        self.queue_event_once(Event::GridChanged);
        self.n_guesses = 0;
        self.visible_letters = 1;
        self.dead_key_queued = false;
        self.is_solved = false;
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
                if self.current_page == Page::Word {
                    if letter == 'x' || letter == 'X' {
                        self.hatify_last_letter();
                    } else {
                        if self.dead_key_queued {
                            letter = hatify(letter).unwrap_or(letter);
                        }

                        self.add_letter(letter);
                    }
                }

                self.dead_key_queued = false;
            },
            Key::Dead => self.dead_key_queued = true,
            Key::Enter => {
                self.dead_key_queued = false;
                match self.current_page {
                    Page::Word => self.enter_guess(),
                    Page::Bingo(team) => self.spin_tombola(team),
                }
            },
            Key::Backspace => {
                self.dead_key_queued = false;
                if self.current_page == Page::Word {
                    self.remove_letter();
                }
            },
            Key::Delete => {
                self.dead_key_queued = false;
                if self.current_page == Page::Word {
                    self.reject_guess();
                }
            },
            Key::PageDown => {
                self.dead_key_queued = false;
                if self.current_page == Page::Word {
                    self.add_hint();
                }
            },
            Key::Space =>  {
                self.dead_key_queued = false;
                self.change_current_team();
            },
            Key::Home => {
                self.dead_key_queued = false;
                if self.current_page == Page::Word {
                    self.pick_word();
                }
            },
            Key::Left =>  {
                self.dead_key_queued = false;
                self.change_page_left();
            },
            Key::Right => {
                self.dead_key_queued = false;
                self.change_page_right();
            },
            Key::Up => self.add_to_score(10),
            Key::Down => self.add_to_score(-10),
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
            self.queue_event_once(Event::GridChanged);
        }
    }

    fn remove_letter(&mut self) {
        if let Some(letter) = self.in_progress_guess.chars().rev().next() {
            self.in_progress_guess.truncate(
                self.in_progress_guess.len() - letter.len_utf8()
            );
            self.queue_event_once(Event::GridChanged);
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
                self.queue_event_once(Event::GridChanged);
            }
        }
    }

    fn reject_guess(&mut self) {
        if self.is_solved || self.n_guesses >= N_GUESSES {
            return;
        }

        let guess = &mut self.guesses[self.n_guesses];

        guess.clear();
        // Copy the letters from the in-progress guess into the guess.
        // If there aren’t enough letters then pad it with spaces
        guess.extend(
            self.in_progress_guess
                .chars()
                .chain(std::iter::repeat(' '))
                .take(self.word_length)
                .map(|letter| {
                    Letter { letter, result: LetterResult::Rejected }
                })
        );

        self.in_progress_guess.clear();

        self.n_guesses += 1;
        self.queue_event_once(Event::GridChanged);
        self.queue_event_once(Event::GuessRejected);
    }

    fn add_hint(&mut self) {
        if self.is_solved || self.n_guesses >= N_GUESSES {
            return;
        }

        // Don’t give a hint if there’s a guess in progress because it
        // won’t be visible and it’d be confusing
        if !self.in_progress_guess.is_empty() {
            return;
        }

        let n_visible_letters = self.visible_letters.count_ones() as usize;

        // Don’t give a hint if it would reveal the entire word
        if n_visible_letters + 1 >= self.word_length {
            return;
        }

        let mut letter_num = random::random_range(
            self.word_length - n_visible_letters
        );

        for i in 0..self.word_length {
            if self.visible_letters & (1 << i) == 0 {
                if letter_num == 0 {
                    self.visible_letters |= 1 << i;
                    break;
                }
                letter_num -= 1;
            }
        }

        self.queue_event_once(Event::GridChanged);
    }

    fn change_current_team(&mut self) {
        let next_team = match self.current_team {
            Team::Left => Team::Right,
            Team::Right => Team::Left,
        };

        self.current_team = next_team;
        self.queue_event_once(Event::CurrentTeamChanged);
    }

    pub fn in_progress_guess(&self) -> &str {
        &self.in_progress_guess
    }

    pub fn get_event(&mut self) -> Option<Event> {
        self.event_queue.pop_front()
    }

    fn queue_event_once(&mut self, event: Event) {
        if !self.event_queue.contains(&event) {
            self.event_queue.push_back(event);
        }
    }

    fn spin_tombola(&mut self, team: Team) {
        self.tombolas[team as usize].start_spin();
        self.queue_event_once(Event::TombolaStartedSpinning(team));
    }

    fn enter_guess(&mut self) {
        if self.is_solved || self.n_guesses >= N_GUESSES {
            return;
        }

        if self.in_progress_guess.chars().count() != self.word_length
            || !self.dictionary.contains(&self.in_progress_guess)
            || self.guess_already_tried(&self.in_progress_guess)
        {
            self.queue_event_once(Event::WrongGuessEntered);
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

        self.is_solved = guess.iter().find(|l| {
            l.result != LetterResult::Correct
        }).is_none();

        self.in_progress_guess.clear();

        self.n_guesses += 1;
        self.queue_event_once(Event::GridChanged);
        self.queue_event_once(Event::GuessEntered);

        if self.is_solved {
            self.scores[self.current_team as usize] += 50;
            self.queue_event_once(Event::ScoreChanged(self.current_team));
            self.queue_event_once(Event::Solved);
        }
    }

    fn guess_matches_word(guess: &[Letter], word: &str) -> bool {
        guess.iter()
            .map(|letter| letter.letter)
            .zip(word.chars())
            .all(|(a, b)| a == b)
    }

    fn guess_already_tried(&self, word: &str) -> bool {
        self.guesses().any(|guess| Logic::guess_matches_word(guess, word))
    }

    fn change_page_left(&mut self) {
        match self.current_page {
            Page::Bingo(Team::Left) => (),
            Page::Word => self.set_page(Page::Bingo(Team::Left)),
            Page::Bingo(Team::Right) => self.set_page(Page::Word),
        }
    }

    fn change_page_right(&mut self) {
        match self.current_page {
            Page::Bingo(Team::Left) => self.set_page(Page::Word),
            Page::Word => self.set_page(Page::Bingo(Team::Right)),
            Page::Bingo(Team::Right) => (),
        }
    }

    fn set_page(&mut self, page: Page) {
        if page != self.current_page {
            let old_page = self.current_page;
            self.current_page = page;
            self.queue_event_once(Event::CurrentPageChanged(old_page));
        }
    }

    fn team_to_edit(&self) -> Team {
        match self.current_page {
            Page::Word => self.current_team,
            Page::Bingo(team) => team,
        }
    }

    fn add_to_score(&mut self, diff: i32) {
        let team = self.team_to_edit();
        let score = &mut self.scores[team as usize];

        match score.checked_add_signed(diff) {
            Some(new_score) => {
                if new_score <= MAX_SCORE {
                    *score = new_score;
                    self.queue_event_once(Event::ScoreChanged(team));
                }
            },
            None => (),
        }
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

    pub fn is_finished(&self) -> bool {
        self.is_solved || self.n_guesses >= N_GUESSES
    }

    pub fn is_solved(&self) -> bool {
        self.is_solved
    }

    pub fn team_score(&self, team: Team) -> u32 {
        self.scores[team as usize]
    }

    pub fn step_tombola(&mut self, team: Team) {
        self.tombolas[team as usize].step();
    }

    pub fn balls(&self, team: Team) -> BallIter {
        BallIter {
            iter: self.tombolas[team as usize].balls(),
            bingo_grid: &self.bingo_grids[team as usize],
        }
    }

    pub fn tombola_rotation(&self, team: Team) -> f32 {
        self.tombolas[team as usize].rotation()
    }

    pub fn tombola_is_sleeping(&self, team: Team) -> bool {
        self.tombolas[team as usize].is_sleeping()
    }

    pub fn claw_pos(&self, team: Team) -> (f32, f32) {
        self.tombolas[team as usize].claw_pos()
    }

    pub fn current_team(&self) -> Team {
        self.current_team
    }

    pub fn current_page(&self) -> Page {
        self.current_page
    }

    pub fn bingo_grid(&self, team: Team) -> &BingoGrid {
        &self.bingo_grids[team as usize]
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

pub struct BallIter<'a> {
    iter: tombola::BallIter<'a>,
    bingo_grid: &'a BingoGrid,
}

impl<'a> Iterator for BallIter<'a> {
    type Item = Ball;

    fn next(&mut self) -> Option<Ball> {
        self.iter.next().map(|ball| {
            let ball_type = if (ball.ball_index as usize) < N_NUMBER_BALLS {
                let space = self.bingo_grid.space(ball.ball_index as usize);
                BallType::Number(space.ball as u32)
            } else {
                BallType::Black
            };

            Ball {
                ball_type,
                x: ball.x,
                y: ball.y,
                rotation: ball.rotation,
            }
        })
    }
}

fn is_valid_letter(letter: char) -> bool {
    let letters = &letter_texture::LETTERS;

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

pub struct LogicLoader {
    dictionary: Option<Dictionary>,
    word_list: Option<Box<[u64]>>,
}

impl LogicLoader {
    pub fn new() -> LogicLoader {
        LogicLoader {
            dictionary: None,
            word_list: None,
        }
    }

    pub fn next_filename(&self) -> Option<&'static str> {
        if self.dictionary.is_none() {
            Some("dictionary.bin")
        } else if self.word_list.is_none() {
            Some("wordlist.bin")
        } else {
            None
        }
    }

    pub fn loaded(&mut self, source: Box<[u8]>) {
        if self.dictionary.is_none() {
            self.dictionary = Some(Dictionary::new(source));
        } else if self.word_list.is_none() {
            const WORD_SIZE: usize =  std::mem::size_of::<u64>();
            let n_words = source.len() / WORD_SIZE;
            let mut words = Vec::<u64>::with_capacity(n_words);

            for index in (0..source.len()).step_by(WORD_SIZE) {
                let mut bytes = [0u8; WORD_SIZE];
                bytes.copy_from_slice(&source[index..index + WORD_SIZE]);
                words.push(u64::from_le_bytes(bytes));
            }

            self.word_list = Some(words.into_boxed_slice());
        } else {
            unreachable!("too many data files loaded!");
        }
    }

    pub fn complete(self) -> Logic {
        Logic::new(self.dictionary.unwrap(), self.word_list.unwrap())
    }
}
