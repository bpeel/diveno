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

mod letter_painter;
mod score_painter;
mod bingo_painter;

use std::rc::Rc;
use super::paint_data::PaintData;
use letter_painter::LetterPainter;
use score_painter::ScorePainter;
use bingo_painter::BingoPainter;
use super::{logic, timer};
use logic::{Team, Page, Logic};
use glow::HasContext;

// Number of millisecends to turn a page
const PAGE_TURN_TIME: i64 = 350;

struct PageAnimation {
    start_time: timer::Timer,
    start_page: Page,
}

enum AnimationPosition {
    OnePage(Page),
    TwoPages {
        left: Page,
        right: Page,
        delta: f32,
    },
}

impl AnimationPosition {
    fn page_visible(&self, page: Page) -> bool {
        match self {
            AnimationPosition::OnePage(other_page) => page == *other_page,
            AnimationPosition::TwoPages { left, right, .. } => {
                page == *left || page == *right
            },
        }
    }
}

pub struct GamePainter {
    paint_data: Rc<PaintData>,
    score_painter: ScorePainter,
    letter_painter: LetterPainter,
    bingo_painters: [BingoPainter; logic::N_TEAMS],
    width: u32,
    height: u32,
    viewport_dirty: bool,
    page_animation: Option<PageAnimation>,
}

impl GamePainter {
    pub fn new(paint_data: Rc<PaintData>) -> Result<GamePainter, String> {
        let gl = &paint_data.gl;

        unsafe {
            gl.enable(glow::CULL_FACE);
        }

        Ok(GamePainter {
            paint_data: Rc::clone(&paint_data),
            score_painter: ScorePainter::new(Rc::clone(&paint_data))?,
            letter_painter: LetterPainter::new(Rc::clone(&paint_data))?,
            bingo_painters: [
                BingoPainter::new(Rc::clone(&paint_data), Team::Left)?,
                BingoPainter::new(paint_data, Team::Right)?,
            ],
            width: 1,
            height: 1,
            viewport_dirty: true,
            page_animation: None,
        })
    }

    fn paint_page(&mut self, logic: &mut Logic, page: Page) -> bool {
        match page {
            Page::Bingo(team) => {
                self.bingo_painters[team as usize].paint(logic)
            },
            Page::Word => {
                self.score_painter.paint(logic)
                    | self.letter_painter.paint(logic)
            },
        }
    }

    pub fn paint(&mut self, logic: &mut Logic) -> bool {
        unsafe {
            let gl = &self.paint_data.gl;
            gl.clear_color(0.0, 0.0, 1.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }

        match self.update_animation_position(logic) {
            AnimationPosition::OnePage(page) => {
                if self.viewport_dirty {
                    unsafe {
                        self.paint_data.gl.viewport(
                            0,
                            0,
                            self.width as i32,
                            self.height as i32
                        );
                    }
                    self.viewport_dirty = false;
                }

                self.paint_page(logic, page)
            },
            AnimationPosition::TwoPages { left, right, delta } => {
                self.viewport_dirty = true;

                let x_pos = (-delta * self.width as f32) as i32;

                unsafe {
                    self.paint_data.gl.viewport(
                        x_pos,
                        0,
                        self.width as i32,
                        self.height as i32,
                    );
                }

                self.paint_page(logic, left);

                unsafe {
                    self.paint_data.gl.viewport(
                        x_pos + self.width as i32,
                        0,
                        self.width as i32,
                        self.height as i32,
                    );
                }

                self.paint_page(logic, right);

                // Redraw always needed while we are animating
                true
            },
        }
    }

    pub fn update_fb_size(&mut self, width: u32, height: u32) {
        self.viewport_dirty = true;
        self.width = width;
        self.height = height;

        self.score_painter.update_fb_size(width, height);
        self.letter_painter.update_fb_size(width, height);

        for painter in self.bingo_painters.iter_mut() {
            painter.update_fb_size(width, height);
        }
    }

    pub fn handle_logic_event(
        &mut self,
        logic: &Logic,
        event: &logic::Event,
    ) -> bool {
        let mut redraw_needed = false;

        if let logic::Event::CurrentPageChanged(old_page) = event {
            self.page_animation = Some(PageAnimation {
                start_time: timer::Timer::new(),
                start_page: *old_page,
            });
            self.viewport_dirty = true;
            redraw_needed = true;
        }

        let animation_position = self.update_animation_position(logic);

        if self.score_painter.handle_logic_event(logic, event)
            && animation_position.page_visible(Page::Word)
        {
            redraw_needed = true;
        }

        if self.letter_painter.handle_logic_event(logic, event)
            && animation_position.page_visible(Page::Word)
        {
            redraw_needed = true;
        }

        for team in [Team::Left, Team::Right] {
            let painter = &mut self.bingo_painters[team as usize];

            if painter.handle_logic_event(logic, event)
                && animation_position.page_visible(Page::Bingo(team))
            {
                redraw_needed = true;
            }
        }

        redraw_needed
    }

    fn update_animation_position(
        &mut self,
        logic: &Logic,
    ) -> AnimationPosition {
        let current_page = logic.current_page();

        match self.page_animation {
            Some(PageAnimation { start_time, start_page }) => {
                let delta = start_time.elapsed() as f32 / PAGE_TURN_TIME as f32;

                if delta >= 1.0 {
                    self.page_animation = None;
                    AnimationPosition::OnePage(current_page)
                } else if current_page.position() < start_page.position() {
                    // Ease-in cubic
                    let delta = delta * delta * delta;

                    AnimationPosition::TwoPages {
                        left: current_page,
                        right: start_page,
                        delta: 1.0 - delta,
                    }
                } else {
                    AnimationPosition::TwoPages {
                        left: start_page,
                        right: current_page,
                        delta: delta,
                    }
                }
            },
            None => AnimationPosition::OnePage(current_page),
        }
    }
}
