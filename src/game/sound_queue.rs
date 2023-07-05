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

use std::collections::binary_heap::BinaryHeap;
use std::cmp::{Ord, Ordering};
use super::{timer, logic, timing};

#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
pub enum Sound {
    CorrectLetter,
    WrongPosition,
    WrongLetter,
    BadWord,
    Solved,
}

pub static SOUND_FILES: [&'static str; 5] = [
    "correct-letter.wav",
    "wrong-position.wav",
    "wrong-letter.wav",
    "bad-word.wav",
    "solved.wav",
];

pub struct SoundQueue {
    start_time: timer::Timer,
    heap: BinaryHeap<QueuedSound>,
}

#[derive(PartialEq, Eq)]
struct QueuedSound {
    play_time: i64,
    sound: Sound,
}

impl Ord for QueuedSound {
    fn cmp(&self, other: &QueuedSound) -> Ordering {
        // Flip the order because we want the lowest play time to have
        // the highest priority
        other.play_time.cmp(&self.play_time)
            .then_with(|| self.sound.cmp(&other.sound))
    }
}

impl PartialOrd for QueuedSound {
    fn partial_cmp(&self, other: &QueuedSound) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl SoundQueue {
    pub fn new() -> SoundQueue {
        SoundQueue {
            start_time: timer::Timer::new(),
            heap: BinaryHeap::new(),
        }
    }

    pub fn queue_sound(&mut self, sound: Sound, delay: i64) {
        self.heap.push(QueuedSound {
            play_time: self.start_time.elapsed() + delay,
            sound
        });
    }

    pub fn next_ready_sound(&mut self) -> Option<Sound> {
        if let Some(qs) = self.heap.peek() {
            if self.start_time.elapsed() >= qs.play_time {
                let sound = qs.sound;
                self.heap.pop();
                Some(sound)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn next_delay(&self) -> Option<i64> {
        self.heap.peek().map(|qs| {
            (qs.play_time - self.start_time.elapsed()).max(0)
        })
    }

    pub fn handle_logic_event(
        &mut self,
        logic: &logic::Logic,
        event: &logic::Event,
    ) {
        match event {
            logic::Event::WordChanged => (),
            logic::Event::GridChanged => (),
            logic::Event::GuessEntered => self.queue_guess_sounds(logic),
            logic::Event::WrongGuessEntered => {
                self.queue_sound(Sound::BadWord, 0);
            },
            logic::Event::GuessRejected => (),
            logic::Event::Solved => self.queue_solved(logic),
            logic::Event::ScoreChanged(_) => (),
            logic::Event::CurrentTeamChanged => (),
            logic::Event::CurrentPageChanged(_) => (),
            logic::Event::TombolaStartedSpinning(_) => (),
            logic::Event::BingoReset(_) => (),
        }
    }

    fn queue_guess_sounds(
        &mut self,
        logic: &logic::Logic,
    ) {
        if let Some(guess) = logic.guesses().last() {
            for (letter_num, letter) in guess.iter().enumerate() {
                let sound = match letter.result {
                    logic::LetterResult::Correct =>
                        Sound::CorrectLetter,
                    logic::LetterResult::WrongPosition =>
                        Sound::WrongPosition,
                    logic::LetterResult::Wrong =>
                        Sound::WrongLetter,
                    logic::LetterResult::Rejected =>
                        continue,
                };

                self.queue_sound(
                    sound,
                    timing::MILLIS_PER_LETTER * letter_num as i64,
                );
            }
        }
    }

    fn queue_solved(
        &mut self,
        logic: &logic::Logic,
    ) {
        self.queue_sound(
            Sound::Solved,
            logic.word_length() as i64 * timing::MILLIS_PER_LETTER
        );
    }
}
