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

use std::rc::Rc;
use super::super::paint_data::PaintData;
use super::super::buffer::Buffer;
use super::super::{shaders, logic, timer, timing, timeout};
use super::super::array_object::ArrayObject;
use timeout::Timeout;
use glow::HasContext;

// Number of digits to show for the score
const N_DIGITS: usize = 3;
// Number of quads needed to draw the frame
const N_FRAME_QUADS: usize = 8;
// Number of quads needed to draw the inner gap
const N_INNER_GAP_QUADS: usize = 4;
// Number of quads needed to draw the bar to show the current team
const N_BAR_QUADS: usize = 1;
// Total number of quads to draw the two score boards
const TOTAL_N_QUADS: usize =
    (N_DIGITS + N_FRAME_QUADS + N_INNER_GAP_QUADS) * 2
    + N_BAR_QUADS;

// The total width allocated to the score
const SCORE_WIDTH: f32 = 2.0 / 4.0;
// The empty gap surrounding the frame
const OUTER_GAP_SIZE: f32 = SCORE_WIDTH / 16.0;
// The width of the frame
const FRAME_WIDTH: f32 = SCORE_WIDTH / 8.0;
// The empty gap inside the frame
const INNER_GAP_SIZE: f32 = SCORE_WIDTH / 16.0;

const TEX_WIDTH: u32 = 1024;
const TEX_HEIGHT: u32 = 128;

// Width of the frame in pixels in the image
const FRAME_PIXEL_WIDTH: u32 = 40;
// Width of the frame in texture coordinates
const FRAME_TEX_WIDTH: u16 = (FRAME_PIXEL_WIDTH * 65535 / TEX_WIDTH) as u16;
// Height of the frame in texture coordinates
const FRAME_TEX_HEIGHT: u16 = (FRAME_PIXEL_WIDTH * 65535 / TEX_HEIGHT) as u16;
// Texture coordinate of the left side of the frame
const FRAME_TEX_LEFT: u16 = ((TEX_WIDTH - 100) * 65535 / TEX_WIDTH) as u16;
// Total width of all the digits in texture coordinates
const DIGITS_TEX_WIDTH: u16 = 56867;

const DIGIT_WIDTH: f32 = (SCORE_WIDTH
                          - (OUTER_GAP_SIZE + FRAME_WIDTH + INNER_GAP_SIZE)
                          * 2.0)
    / N_DIGITS as f32;
const DIGIT_HEIGHT: f32 = DIGIT_WIDTH
    * TEX_HEIGHT as f32
    / (DIGITS_TEX_WIDTH as f32
       / u16::MAX as f32
       / 10.0
       * TEX_WIDTH as f32);

// Tex coords of a known black texel
const GAP_TEX_S: u16 = ((65535 + FRAME_TEX_LEFT as u32) / 2) as u16;
const GAP_TEX_T: u16 = u16::MAX / 2;

// Milliseconds per unit change when animating the score
const SCORE_CHANGE_TIME: i64 = 30;

// Texture coordinates of the bar to show the current team
const BAR_TEX_S1: u16 = (902 * 65535 / TEX_WIDTH) as u16;
const BAR_TEX_S2: u16 = BAR_TEX_S1 + (17 * 65535 / TEX_WIDTH) as u16;
// Height of the bar
const BAR_HEIGHT: f32 = SCORE_WIDTH / 10.0;

/// Used in the constructor to pick which team’s score to display.
pub enum TeamChoice {
    OneTeam(logic::Team),
    AllTeams,
}

#[repr(C)]
struct Vertex {
    x: f32,
    y: f32,
    s: u16,
    t: u16,
}

struct AnimatedScore {
    start_score: u32,
    // Delay before starting to animate the score. This is to avoid
    // animating the score change before the reveal animation starts.
    delay: i64,
    start_time: timer::Timer,
}

pub struct ScorePainter {
    team_choice: TeamChoice,
    buffer: Rc<Buffer>,
    array_object: ArrayObject,
    paint_data: Rc<PaintData>,
    width: u32,
    height: u32,
    vertices_dirty: bool,
    // Temporary buffer used for building the vertex buffer
    vertices: Vec<Vertex>,
    animated_scores: [Option<AnimatedScore>; logic::N_TEAMS],
    last_scores: [u32; logic::N_TEAMS],
}

impl ScorePainter {
    pub fn new(
        paint_data: Rc<PaintData>,
        team_choice: TeamChoice,
    ) -> Result<ScorePainter, String> {
        let buffer = create_score_buffer(&paint_data)?;
        let array_object = create_array_object(
            &paint_data,
            Rc::clone(&buffer),
        )?;

        Ok(ScorePainter {
            team_choice,
            buffer,
            array_object,
            paint_data,
            width: 1,
            height: 1,
            vertices_dirty: true,
            vertices: Vec::with_capacity(TOTAL_N_QUADS * 4),
            animated_scores: Default::default(),
            last_scores: Default::default(),
        })
    }

    fn team_is_visible(&self, team: logic::Team) -> bool {
        match self.team_choice {
            TeamChoice::OneTeam(chosen_team) => team == chosen_team,
            TeamChoice::AllTeams => true,
        }
    }

    pub fn paint(&mut self, logic: &logic::Logic) -> Timeout {
        if logic.super_diveno().is_some() {
            return Timeout::Forever;
        }

        if self.vertices_dirty {
            self.update_vertices(logic);
            self.vertices_dirty = false;
        }

        self.array_object.bind();

        let gl = &self.paint_data.gl;

        unsafe {
            gl.bind_texture(
                glow::TEXTURE_2D,
                Some(self.paint_data.images.segments.id()),
            );

            gl.use_program(Some(self.paint_data.shaders.score.id()));

            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.enable(glow::BLEND);

            gl.draw_elements(
                glow::TRIANGLES,
                self.vertices.len() as i32 / 4 * 6,
                glow::UNSIGNED_SHORT,
                0, // offset
            );

            gl.disable(glow::BLEND);
        }

        // Redraw again if any of the scores are animated
        if self.animated_scores.iter().any(|s| s.is_some()) {
            self.vertices_dirty = true;
            timeout::IMMEDIATELY
        } else {
            Timeout::Forever
        }
    }

    pub fn update_fb_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.vertices_dirty = true;
    }

    pub fn handle_logic_event(
        &mut self,
        logic: &logic::Logic,
        event: &logic::Event,
    ) -> bool {
        match event {
            logic::Event::WordChanged => false,
            logic::Event::GridChanged => false,
            logic::Event::GuessEntered => false,
            logic::Event::WrongGuessEntered => false,
            logic::Event::GuessRejected => false,
            logic::Event::CurrentPageChanged(_) => false,
            logic::Event::TombolaStartedSpinning(_) => false,
            logic::Event::BingoReset(_) => false,
            logic::Event::BingoChanged(..) => false,
            logic::Event::Bingo(team, _) => {
                if logic.super_diveno().is_none() &&
                    self.team_is_visible(*team)
                {
                    self.animate_bingo_score_change(*team);
                    true
                } else {
                    false
                }
            },
            logic::Event::Solved => {
                if logic.super_diveno().is_none()
                    && self.team_is_visible(logic.current_team())
                {
                    self.animate_solved_score_change(logic);
                    true
                } else {
                    false
                }
            },
            logic::Event::ScoreChanged(team) => {
                if logic.super_diveno().is_none()
                    && self.team_is_visible(*team)
                {
                    self.vertices_dirty = true;
                    true
                } else {
                    false
                }
            },
            logic::Event::CurrentTeamChanged => {
                if logic.super_diveno().is_none() {
                    self.vertices_dirty = true;
                    true
                } else {
                    false
                }
            },
        }
    }

    fn animate_solved_score_change(&mut self, logic: &logic::Logic) {
        self.animate_score_change(
            logic.current_team(),
            timing::MILLIS_PER_LETTER * logic.word_length() as i64,
        );
    }

    fn animate_bingo_score_change(&mut self, team: logic::Team) {
        self.animate_score_change(
            team,
            0, // delay
        );
    }

    fn animate_score_change(&mut self, team: logic::Team, delay: i64) {
        let slot = &mut self.animated_scores[team as usize];

        match slot.as_mut() {
            None => {
                *slot = Some(AnimatedScore {
                    start_score: self.last_scores[team as usize],
                    start_time: timer::Timer::new(),
                    delay,
                });

                self.vertices_dirty = true;
            },
            Some(animated_score) => {
                let delay = delay + animated_score.start_time.elapsed();
                animated_score.delay = animated_score.delay.max(delay);
            },
        }
    }

    fn update_animated_score(
        &mut self,
        logic: &logic::Logic,
        team: logic::Team
    ) -> u32 {
        let target_score = logic.team_score(team);

        let paint_score = match self.animated_scores[team as usize] {
            Some(ref animated_score) => {
                let score_diff = animated_score.start_score
                    .abs_diff(target_score) as i64;
                let total_time = score_diff * SCORE_CHANGE_TIME;
                let elapsed = (animated_score.start_time.elapsed()
                               - animated_score.delay)
                    .max(0);

                if elapsed >= total_time {
                    self.animated_scores[team as usize] = None;
                    target_score
                } else {
                    (animated_score.start_score as i64
                     + (target_score as i64 - animated_score.start_score as i64)
                     * elapsed
                     / total_time)
                        as u32
                }
            },
            None => {
                target_score
            }
        };

        self.last_scores[team as usize] = paint_score;

        paint_score
    }

    fn add_quad_rotated_tex(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        s1: u16,
        t1: u16,
        s2: u16,
        t2: u16,
    ) {
        self.vertices.push(Vertex { x: x1, y: y1, s: s2, t: t1, });
        self.vertices.push(Vertex { x: x1, y: y2, s: s1, t: t1, });
        self.vertices.push(Vertex { x: x2, y: y1, s: s2, t: t2, });
        self.vertices.push(Vertex { x: x2, y: y2, s: s1, t: t2, });
    }

    fn add_quad(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        s1: u16,
        t1: u16,
        s2: u16,
        t2: u16,
    ) {
        self.vertices.push(Vertex { x: x1, y: y1, s: s1, t: t1, });
        self.vertices.push(Vertex { x: x1, y: y2, s: s1, t: t2, });
        self.vertices.push(Vertex { x: x2, y: y1, s: s2, t: t1, });
        self.vertices.push(Vertex { x: x2, y: y2, s: s2, t: t2, });
    }

    fn add_frame(&mut self, x: f32) {
        let y_scale = self.width as f32 / self.height as f32;

        let left = x + OUTER_GAP_SIZE;
        let right = x + SCORE_WIDTH - OUTER_GAP_SIZE;
        let top = (DIGIT_HEIGHT / 2.0 + INNER_GAP_SIZE + FRAME_WIDTH) * y_scale;
        let bottom = -top;

        // Left side
        self.add_quad(
            left,
            top - FRAME_WIDTH * y_scale,
            left + FRAME_WIDTH,
            bottom + FRAME_WIDTH * y_scale,
            FRAME_TEX_LEFT,
            u16::MAX / 2,
            FRAME_TEX_LEFT + FRAME_TEX_WIDTH,
            u16::MAX / 2,
        );
        // Right side
        self.add_quad(
            right - FRAME_WIDTH,
            top - FRAME_WIDTH * y_scale,
            right,
            bottom + FRAME_WIDTH * y_scale,
            u16::MAX - FRAME_TEX_WIDTH,
            u16::MAX / 2,
            u16::MAX,
            u16::MAX / 2,
        );
        // Top side
        self.add_quad(
            left + FRAME_WIDTH,
            top,
            right - FRAME_WIDTH,
            top - FRAME_WIDTH * y_scale,
            GAP_TEX_S,
            0,
            GAP_TEX_S,
            FRAME_TEX_HEIGHT,
        );
        // Bottom side
        self.add_quad(
            left + FRAME_WIDTH,
            bottom + FRAME_WIDTH * y_scale,
            right - FRAME_WIDTH,
            bottom,
            GAP_TEX_S,
            u16::MAX - FRAME_TEX_HEIGHT,
            GAP_TEX_S,
            u16::MAX,
        );

        // Top-left corner
        self.add_quad(
            left,
            top,
            left + FRAME_WIDTH,
            top - FRAME_WIDTH * y_scale,
            FRAME_TEX_LEFT,
            0,
            FRAME_TEX_LEFT + FRAME_TEX_WIDTH,
            FRAME_TEX_HEIGHT,
        );
        // Top-right corner
        self.add_quad(
            right - FRAME_WIDTH,
            top,
            right,
            top - FRAME_WIDTH * y_scale,
            u16::MAX - FRAME_TEX_WIDTH,
            0,
            u16::MAX,
            FRAME_TEX_HEIGHT,
        );
        // Bottom-left corner
        self.add_quad(
            left,
            bottom + FRAME_WIDTH * y_scale,
            left + FRAME_WIDTH,
            bottom,
            FRAME_TEX_LEFT,
            u16::MAX - FRAME_TEX_HEIGHT,
            FRAME_TEX_LEFT + FRAME_TEX_WIDTH,
            u16::MAX,
        );
        // Bottom-right corner
        self.add_quad(
            right - FRAME_WIDTH,
            bottom + FRAME_WIDTH * y_scale,
            right,
            bottom,
            u16::MAX - FRAME_TEX_WIDTH,
            u16::MAX - FRAME_TEX_HEIGHT,
            u16::MAX,
            u16::MAX,
        );
    }

    fn add_gap_quad(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.add_quad(
            x1,
            y1,
            x2,
            y2,
            GAP_TEX_S,
            GAP_TEX_T,
            GAP_TEX_S,
            GAP_TEX_T,
        );
    }

    fn add_inner_gap(&mut self, x: f32) {
        let y_scale = self.width as f32 / self.height as f32;

        let left = x + OUTER_GAP_SIZE + FRAME_WIDTH;
        let right = x + SCORE_WIDTH - OUTER_GAP_SIZE - FRAME_WIDTH;
        let top = (DIGIT_HEIGHT / 2.0 + INNER_GAP_SIZE) * y_scale;
        let bottom = -top;

        // Left side
        self.add_gap_quad(
            left,
            top,
            left + INNER_GAP_SIZE,
            bottom,
        );
        // Right side
        self.add_gap_quad(
            right - INNER_GAP_SIZE,
            top,
            right,
            bottom,
        );
        // Top side
        self.add_gap_quad(
            left + INNER_GAP_SIZE,
            top,
            right - INNER_GAP_SIZE,
            top - INNER_GAP_SIZE * y_scale,
        );
        // Bottom side
        self.add_gap_quad(
            left + INNER_GAP_SIZE,
            bottom + INNER_GAP_SIZE * y_scale,
            right - INNER_GAP_SIZE,
            bottom,
        );
    }

    fn add_digits(&mut self, x: f32, mut score: u32) {
        let y_scale = self.width as f32 / self.height as f32;

        let right = x
            + SCORE_WIDTH
            - OUTER_GAP_SIZE
            - FRAME_WIDTH
            - INNER_GAP_SIZE;
        let top = DIGIT_HEIGHT / 2.0 * y_scale;
        let bottom = -top;

        for digit_num in 0..N_DIGITS {
            let (s1, t1, s2, t2) = if score <= 0 && digit_num > 0{
                (GAP_TEX_S, GAP_TEX_T, GAP_TEX_S, GAP_TEX_T)
            } else {
                let digit = score % 10;

                (
                    (DIGITS_TEX_WIDTH as u32 * digit / 10) as u16,
                    0,
                    (DIGITS_TEX_WIDTH as u32 * (digit + 1) / 10) as u16,
                    u16::MAX,
                )
            };

            self.add_quad(
                right - DIGIT_WIDTH * (digit_num + 1) as f32,
                top,
                right - DIGIT_WIDTH * digit_num as f32,
                bottom,
                s1,
                t1,
                s2,
                t2,
            );

            score /= 10;
        }
    }

    fn add_scoreboard(&mut self, x: f32, score: u32) {
        self.add_frame(x);
        self.add_inner_gap(x);
        self.add_digits(x, score);
    }

    fn add_current_team(&mut self, logic: &logic::Logic) {
        let x = match logic.current_team() {
            logic::Team::Left => -1.0,
            logic::Team::Right => 1.0 - SCORE_WIDTH,
        };

        let y_scale = self.width as f32 / self.height as f32;

        let y = (-DIGIT_HEIGHT / 2.0
                 - INNER_GAP_SIZE
                 - FRAME_WIDTH
                 - OUTER_GAP_SIZE)
            * y_scale;

        self.add_quad_rotated_tex(
            x + OUTER_GAP_SIZE,
            y,
            x + SCORE_WIDTH - OUTER_GAP_SIZE,
            y - BAR_HEIGHT * y_scale,
            BAR_TEX_S1,
            0,
            BAR_TEX_S2,
            65535,
        );
    }

    fn fill_vertices_array(&mut self, logic: &logic::Logic) {
        self.vertices.clear();

        if self.team_is_visible(logic::Team::Left) {
            let left_score = self.update_animated_score(
                logic,
                logic::Team::Left
            );
            self.add_scoreboard(-1.0, left_score);
        }

        if self.team_is_visible(logic::Team::Right) {
            let right_score = self.update_animated_score(
                logic,
                logic::Team::Right
            );
            self.add_scoreboard(1.0 - SCORE_WIDTH, right_score);
        }

        if self.team_is_visible(logic.current_team()) {
            self.add_current_team(logic);
        }

        assert!(self.vertices.len() <= TOTAL_N_QUADS * 4);
    }

    fn update_vertices(&mut self, logic: &logic::Logic) {
        self.fill_vertices_array(logic);

        let gl = &self.paint_data.gl;

        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.buffer.id()));

            let buffer_data = std::slice::from_raw_parts(
                self.vertices.as_ptr() as *const u8,
                self.vertices.len() * std::mem::size_of::<Vertex>(),
            );

            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                buffer_data,
                glow::DYNAMIC_DRAW,
            );
        }
    }
}

fn create_array_object(
    paint_data: &Rc<PaintData>,
    buffer: Rc<Buffer>,
) -> Result<ArrayObject, String> {
    let mut array_object = ArrayObject::new(Rc::clone(paint_data))?;
    let mut offset = 0;

    array_object.set_attribute(
        shaders::POSITION_ATTRIB,
        2, // size
        glow::FLOAT,
        false, // normalized
        std::mem::size_of::<Vertex>() as i32,
        Rc::clone(&buffer),
        offset,
    );
    offset += std::mem::size_of::<f32>() as i32 * 2;

    array_object.set_attribute(
        shaders::TEX_COORD_ATTRIB,
        2, // size
        glow::UNSIGNED_SHORT,
        true, // normalized
        std::mem::size_of::<Vertex>() as i32,
        Rc::clone(&buffer),
        offset,
    );

    paint_data.quad_tool.set_element_buffer(
        &mut array_object,
        TOTAL_N_QUADS as u32,
    )?;

    Ok(array_object)
}

fn create_score_buffer(paint_data: &PaintData) -> Result<Rc<Buffer>, String> {
    let buffer = Buffer::new(Rc::clone(&paint_data.gl))?;

    Ok(Rc::new(buffer))
}
