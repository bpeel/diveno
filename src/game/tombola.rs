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

use rapier2d::prelude::*;
use super::timer::Timer;
use std::f32::consts::PI;

pub const N_NUMBER_BALLS: usize = 17;
pub const N_BLACK_BALLS: usize = 3;
pub const N_BALLS: usize = N_NUMBER_BALLS + N_BLACK_BALLS;
pub const BALL_SIZE: f32 = 12.0;
const STEPS_PER_SECOND: i64 = 60;

// Distance from the centre of the tombola to the inner part of the
// middle of a side
pub const APOTHEM: f32 = 50.0;
// Number of sides of the tombola shape (it’s a hexagon)
pub const N_SIDES: u32 = 6;

// Length of a side of the tombola
// https://en.wikipedia.org/wiki/Regular_polygon#Circumradius
// Rustc can’t do const trigonometry so this is:
// 2.0 * (PI / N_SIDES as f32).tan() * APOTHEM
pub const SIDE_LENGTH: f32 = 2.0 * 0.5773502691896257 * APOTHEM;
// Width of the side of the tombola
const SIDE_WIDTH: f32 = 10.0;

// Number of milliseconds per turn of the tombola
const TURN_TIME: i64 = 2000;
// Number of turns to do before stopping
const N_TURNS: i64 = 3;

// Time to wait after spinning before moving the claw in milliseconds
const CLAW_WAIT_TIME: i64 = 6000;
// Speed of the claw in length units per second
const CLAW_SPEED: f32 = APOTHEM / 2.0;
// Maximum distance to travel away from the tombola centre
const CLAW_MAX: f32 = APOTHEM / 0.8660254037844387 + BALL_SIZE / 2.0;

pub enum BallType {
    Number(u8),
    Black,
}

pub struct Ball {
    pub ball_type: BallType,
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
}

enum SpinStage {
    None,
    Spinning(i64),
    Waiting(i64),
    Descending(i64),
    Ascending {
        start_steps: i64,
        start_pos: f32,
        ball: Option<usize>,
    },
    SlidingOut(i64, usize),
    SlidingIn(i64),
}

pub struct Tombola {
    start_time: Timer,
    steps_executed: i64,
    spin_stage: SpinStage,

    rotation: f32,
    claw_x: f32,
    claw_y: f32,

    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,
    gravity: Vector<Real>,
    ball_handles: Vec<RigidBodyHandle>,
    side_handles: Vec<RigidBodyHandle>,
}

impl Tombola {
    pub fn new() -> Tombola {
        let mut rigid_body_set = RigidBodySet::new();
        let mut collider_set = ColliderSet::new();
        let mut ball_handles = Vec::with_capacity(N_BALLS);
        let mut side_handles = Vec::with_capacity(N_SIDES as usize);

        let packer = HexagonalPacker::new(
            BALL_SIZE / 2.0,
            (N_BALLS as f32).sqrt().round() as u32,
        ).take(N_BALLS);

        ball_handles.extend(packer.enumerate().map(|(ball_num, (x, y))| {
            let ball_body = RigidBodyBuilder::dynamic()
                .user_data(ball_num as u128)
                .translation(vector![x, y])
                .build();
            let ball_handle = rigid_body_set.insert(ball_body);

            let collider = ColliderBuilder::ball(BALL_SIZE / 2.0)
                .user_data(ball_num as u128)
                .build();
            collider_set.insert_with_parent(
                collider,
                ball_handle,
                &mut rigid_body_set,
            );

            ball_handle
        }));

        side_handles.extend((0..N_SIDES).map(|side_num| {
            let side_body = RigidBodyBuilder::fixed()
                .position(Tombola::side_position(side_num as usize, 0.0))
                .build();
            let side_handle = rigid_body_set.insert(side_body);

            let collider = ColliderBuilder::cuboid(
                // The width is added to the length so that the ends
                // of the sides will overlap. Otherwise the balls can
                // sometimes escape through the single point where the
                // sides touch.
                SIDE_LENGTH / 2.0 + SIDE_WIDTH,
                SIDE_WIDTH / 2.0,
            ).restitution(0.7)
                .user_data(u128::MAX)
                .build();
            collider_set.insert_with_parent(
                collider,
                side_handle,
                &mut rigid_body_set,
            );

            side_handle
        }));

        Tombola {
            start_time: Timer::new(),
            steps_executed: 0,
            spin_stage: SpinStage::None,

            rotation: 0.0,
            claw_x: 0.0,
            claw_y: CLAW_MAX,

            rigid_body_set,
            collider_set,
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
            gravity: vector![0.0, -9.81],
            ball_handles,
            side_handles,
        }
    }

    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    fn update_rotation(&mut self) -> bool {
        if let SpinStage::Spinning(start_steps) = self.spin_stage {
            let executed = self.steps_executed - start_steps;
            let n_turns = executed * 1000 / STEPS_PER_SECOND / TURN_TIME;

            if n_turns >= N_TURNS {
                self.spin_stage = SpinStage::Waiting(self.steps_executed);
                self.rotation = 0.0;
                self.freeze_sides();
            } else {
                self.rotation = executed as f32
                    * 1000.0
                    / STEPS_PER_SECOND as f32
                    / TURN_TIME as f32
                    * 2.0 * PI
            }

            true
        } else {
            false
        }
    }

    fn update_claw(&mut self) {
        match self.spin_stage {
            SpinStage::Spinning(_) |
            SpinStage::None => {
                self.claw_x = 0.0;
                self.claw_y = CLAW_MAX;
            }
            SpinStage::Waiting(start_steps) => {
                self.update_waiting_claw(start_steps);
            },
            SpinStage::Descending(start_steps) => {
                self.update_descending_claw(start_steps);
            },
            SpinStage::Ascending { start_steps, start_pos, ball } => {
                self.update_ascending_claw(start_steps, start_pos, ball);
            },
            SpinStage::SlidingOut(start_steps, ball) => {
                self.update_sliding_out_claw(start_steps, ball);
            },
            SpinStage::SlidingIn(start_steps) => {
                self.update_sliding_in_claw(start_steps);
            },
        }
    }

    fn update_waiting_claw(&mut self, start_steps: i64) {
        let millis = (self.steps_executed - start_steps)
            * 1000
            / STEPS_PER_SECOND;

        if millis >= CLAW_WAIT_TIME {
            self.spin_stage = SpinStage::Descending(self.steps_executed);
        }
    }

    fn grab_ball(&self, x: f32, y: f32) -> Option<usize> {
        let mut found_ball = None;

        self.query_pipeline.intersections_with_point(
            &self.rigid_body_set,
            &self.collider_set,
            &Point::new(x, y),
            QueryFilter::default(),
            |handle: ColliderHandle| {
                let collider = &self.collider_set[handle];

                if collider.user_data < N_BALLS as u128 {
                    found_ball = Some(collider.user_data as usize);
                    false
                } else {
                    true
                }
            }
        );

        found_ball
    }

    fn update_descending_claw(&mut self, start_steps: i64) {
        let executed = self.steps_executed - start_steps;
        let seconds = executed as f32 / STEPS_PER_SECOND as f32;

        let claw_pos = CLAW_MAX - seconds * CLAW_SPEED;

        if claw_pos <= -CLAW_MAX {
            self.spin_stage = SpinStage::Ascending {
                start_steps: self.steps_executed,
                start_pos: -CLAW_MAX,
                ball: None,
            };
            self.claw_x = 0.0;
            self.claw_y = -CLAW_MAX;
        } else {
            self.claw_x = 0.0;
            self.claw_y = claw_pos;

            if let Some(ball) = self.grab_ball(0.0, claw_pos) {
                let ball_body =
                    &mut self.rigid_body_set[self.ball_handles[ball]];

                ball_body.set_body_type(
                    RigidBodyType::KinematicPositionBased,
                    true,
                );
                ball_body.set_next_kinematic_translation(
                    vector![0.0, claw_pos]
                );

                self.spin_stage = SpinStage::Ascending {
                    start_steps: self.steps_executed,
                    start_pos: claw_pos,
                    ball: Some(ball),
                }
            }
        }
    }

    fn update_ascending_claw(
        &mut self,
        start_steps: i64,
        start_pos: f32,
        ball: Option<usize>,
    ) {
        let executed = self.steps_executed - start_steps;
        let seconds = executed as f32 / STEPS_PER_SECOND as f32;

        let claw_pos = start_pos + seconds * CLAW_SPEED;

        if claw_pos >= CLAW_MAX {
            self.claw_x = 0.0;
            self.claw_y = CLAW_MAX;
            self.spin_stage = ball.map(|ball| {
                SpinStage::SlidingOut(self.steps_executed, ball)
            }).unwrap_or(SpinStage::None);
        } else {
            self.claw_x = 0.0;
            self.claw_y = claw_pos;

            if let Some(ball) = ball {
                let ball_body =
                    &mut self.rigid_body_set[self.ball_handles[ball]];

                ball_body.set_next_kinematic_translation(
                    vector![0.0, claw_pos]
                );
            }
        }
    }

    fn update_sliding_out_claw(&mut self, start_steps: i64, ball: usize) {
        let executed = self.steps_executed - start_steps;
        let seconds = executed as f32 / STEPS_PER_SECOND as f32;
        let claw_pos = seconds * CLAW_SPEED;

        self.claw_y = CLAW_MAX;

        let ball_body =
            &mut self.rigid_body_set[self.ball_handles[ball]];

        if claw_pos >= CLAW_MAX {
            self.claw_x = CLAW_MAX;

            ball_body.set_body_type(RigidBodyType::Dynamic, true);
            self.spin_stage = SpinStage::SlidingIn(self.steps_executed);
        } else {
            self.claw_x = claw_pos;

            ball_body.set_next_kinematic_translation(
                vector![self.claw_x, self.claw_y]
            );
        }
    }

    fn update_sliding_in_claw(&mut self, start_steps: i64) {
        let executed = self.steps_executed - start_steps;
        let seconds = executed as f32 / STEPS_PER_SECOND as f32;
        let claw_pos = CLAW_MAX - seconds * CLAW_SPEED;

        self.claw_y = CLAW_MAX;

        if claw_pos <= 0.0 {
            self.claw_x = 0.0;
            self.spin_stage = SpinStage::None;
        } else {
            self.claw_x = claw_pos;
        }
    }

    fn side_position(side_num: usize, rotation: f32) -> Isometry<Real> {
        const RADIUS: f32 = APOTHEM + SIDE_WIDTH / 2.0;
        let angle = rotation
            + (side_num as f32 + 0.5) * 2.0 * PI
            / N_SIDES as f32;

        let x = -RADIUS * angle.sin();
        let y = RADIUS * angle.cos();

        Isometry::new(vector![x, y], angle)
    }

    fn update_sides(&mut self) {
        if !self.update_rotation() {
            return;
        }

        for (side_num, &side_handle) in self.side_handles.iter().enumerate() {
            let position = Tombola::side_position(side_num, self.rotation);
            let side_body = &mut self.rigid_body_set[side_handle];
            side_body.set_next_kinematic_position(position);
        }
    }

    pub fn step(&mut self) {
        // Try to run enough steps to catch to 60 steps per second,
        // but if too much time has passed then assume the simulation
        // has paused and start counting from scratch

        let elapsed = self.start_time.elapsed();
        let target_steps = elapsed * STEPS_PER_SECOND / 1000;
        let n_steps = target_steps - self.steps_executed;

        if n_steps < 0 || n_steps > 4 {
            self.steps_executed = target_steps;
        } else {
            for _ in 0..n_steps {
                self.update_sides();
                self.update_claw();

                self.physics_pipeline.step(
                    &self.gravity,
                    &self.integration_parameters,
                    &mut self.island_manager,
                    &mut self.broad_phase,
                    &mut self.narrow_phase,
                    &mut self.rigid_body_set,
                    &mut self.collider_set,
                    &mut self.impulse_joint_set,
                    &mut self.multibody_joint_set,
                    &mut self.ccd_solver,
                    Some(&mut self.query_pipeline),
                    &(), // physics_hooks
                    &(), // event handler
                );

                self.steps_executed += 1;
            }
        }
    }

    pub fn balls(&self) -> BallIter {
        BallIter {
            handle_iter: self.ball_handles.iter().enumerate(),
            rigid_body_set: &self.rigid_body_set,
        }
    }

    fn unfreeze_sides(&mut self) {
        for &side_handle in self.side_handles.iter() {
            self.rigid_body_set[side_handle].set_body_type(
                RigidBodyType::KinematicPositionBased,
                true, // wake up
            );
        }
    }

    fn freeze_sides(&mut self) {
        for &side_handle in self.side_handles.iter() {
            self.rigid_body_set[side_handle].set_body_type(
                RigidBodyType::Fixed,
                false, // wake up
            );
        }
    }

    pub fn start_spin(&mut self) {
        if matches!(self.spin_stage, SpinStage::None) {
            self.spin_stage = SpinStage::Spinning(self.steps_executed);
            self.unfreeze_sides();
        }
    }

    pub fn is_sleeping(&self) -> bool {
        if !matches!(self.spin_stage, SpinStage::None) {
            return false;
        }

        for &ball_handle in self.ball_handles.iter() {
            if !self.rigid_body_set[ball_handle].is_sleeping() {
                return false;
            }
        }

        true
    }
}

pub struct BallIter<'a> {
    handle_iter: std::iter::Enumerate<std::slice::Iter<'a, RigidBodyHandle>>,
    rigid_body_set: &'a RigidBodySet,
}

impl<'a> Iterator for BallIter<'a> {
    type Item = Ball;

    fn next(&mut self) -> Option<Ball> {
        self.handle_iter.next().map(|(ball_num, &ball_handle)| {
            let ball_type = if ball_num < N_NUMBER_BALLS {
                BallType::Number(ball_num as u8 + 1)
            } else {
                BallType::Black
            };

            let ball_body = &self.rigid_body_set[ball_handle];
            let translation = ball_body.translation();

            Ball {
                ball_type,
                x: translation.x,
                y: translation.y,
                rotation: ball_body.rotation().angle(),
            }
        })
    }
}

pub struct HexagonalPacker {
    radius: f32,
    vertical_distance: f32,
    next_circle_num: u32,
    n_circles_per_row: u32,
}

impl HexagonalPacker {
    fn new(radius: f32, n_circles_per_row: u32) -> HexagonalPacker {
        // Vertical distance between the packed circles. This is the
        // apothem of the hexagon.
        let vertical_distance = BALL_SIZE * (PI / 6.0).cos();

        HexagonalPacker {
            radius,
            vertical_distance,
            next_circle_num: 0,
            n_circles_per_row,
        }
    }
}

impl Iterator for HexagonalPacker {
    type Item = (f32, f32);

    fn next(&mut self) -> Option<(f32, f32)> {
        let x_index = self.next_circle_num % self.n_circles_per_row;
        let y_index = self.next_circle_num / self.n_circles_per_row;
        self.next_circle_num += 1;

        let mut x = if x_index & 1 == 0 {
            (x_index / 2) as f32
        } else {
            -((x_index / 2) as f32) - 1.0
        };

        if y_index > 0 && (y_index - 1) & 2 == 0 {
            x += 0.5
        }

        let y = if y_index & 1 == 0 {
            (y_index / 2) as f32
        } else {
            -((y_index / 2) as f32) - 1.0
        };

        Some((x * self.radius * 2.0, y * self.vertical_distance))
    }
}
