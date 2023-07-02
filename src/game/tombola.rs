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

pub struct Tombola {
    start_time: Timer,
    steps_executed: i64,
    spin_start_steps: Option<i64>,

    rotation: f32,

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

            let collider = ColliderBuilder::ball(BALL_SIZE / 2.0).build();
            collider_set.insert_with_parent(
                collider,
                ball_handle,
                &mut rigid_body_set,
            );

            ball_handle
        }));

        side_handles.extend((0..N_SIDES).map(|side_num| {
            let side_body = RigidBodyBuilder::kinematic_position_based()
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
            spin_start_steps: None,

            rotation: 0.0,

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
            gravity: vector![0.0, -9.81],
            ball_handles,
            side_handles,
        }
    }

    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    fn update_rotation(&mut self) -> bool {
        match self.spin_start_steps {
            Some(start_steps) => {
                let executed = self.steps_executed - start_steps;
                let n_turns = executed * 1000 / STEPS_PER_SECOND / TURN_TIME;

                if n_turns >= N_TURNS {
                    self.spin_start_steps = None;
                    self.rotation = 0.0;
                } else {
                    self.rotation = executed as f32
                        * 1000.0
                        / STEPS_PER_SECOND as f32
                        / TURN_TIME as f32
                        * 2.0 * PI
                }

                true
            },
            None => false,
        }
    }

    fn side_position(side_num: usize, rotation: f32) -> Isometry<Real> {
        const RADIUS: f32 = APOTHEM + SIDE_WIDTH / 2.0;
        let angle = rotation + side_num as f32 * 2.0 * PI / N_SIDES as f32;

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
                    None, // query_pipeline
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

    pub fn start_spin(&mut self) {
        if self.spin_start_steps.is_none() {
            self.spin_start_steps = Some(self.steps_executed);
        }
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
