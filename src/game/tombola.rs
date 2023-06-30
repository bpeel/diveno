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

pub const N_NUMBER_BALLS: usize = 17;
pub const N_BLACK_BALLS: usize = 3;
pub const N_BALLS: usize = N_NUMBER_BALLS + N_BLACK_BALLS;
pub const BALL_SIZE: f32 = 10.0;
const STEPS_PER_SECOND: i64 = 60;

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
}

impl Tombola {
    pub fn new() -> Tombola {
        let mut rigid_body_set = RigidBodySet::new();
        let mut collider_set = ColliderSet::new();
        let mut ball_handles = Vec::with_capacity(N_BALLS);

        ball_handles.extend((0..N_BALLS).map(|ball_num| {
            let ball_body = RigidBodyBuilder::dynamic()
                .user_data(ball_num as u128)
                .translation(vector![
                    ball_num as f32 * BALL_SIZE as f32 * 0.25 - 100.0,
                    ball_num as f32 * BALL_SIZE as f32
                ])
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

        let collider = ColliderBuilder::cuboid(1000.0, 10.0)
            .translation(vector![0.0, -100.0])
            .build();
        collider_set.insert(collider);

        Tombola {
            start_time: Timer::new(),
            steps_executed: 0,

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
            }

            self.steps_executed += n_steps;
        }
    }

    pub fn balls(&self) -> BallIter {
        BallIter {
            handle_iter: self.ball_handles.iter().enumerate(),
            rigid_body_set: &self.rigid_body_set,
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