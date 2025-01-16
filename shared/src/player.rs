use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::{*, client::*};
use serde::{Deserialize, Serialize};
use crate::prelude::PlayerInput;

const MOVE_SPEED : f32 = 0.125;
const LOOK_ROTATION_SPEED : f32 = 0.003;
const ROLL_SPEED : f32 = 0.02;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        // SYSTEMS
        app.add_systems(FixedUpdate, move_player);
    }
}

pub fn move_player(
    mut query: Query<(
        &Player,
        &mut Position,
        &mut Rotation,
        &ActionState<PlayerInput>,
    ),
    Or<(With<Predicted>, With<Replicating>)>>
) {
    for (player, mut position, mut rotation, action_state) in query.iter_mut() {
        let mut wish_dir = Vec3::ZERO;
        
        if let Some(data) = action_state.dual_axis_data(&PlayerInput::Look) {
            let yaw = -data.fixed_update_pair.x * LOOK_ROTATION_SPEED;
            let pitch = -data.fixed_update_pair.y * LOOK_ROTATION_SPEED;
            
            let right = rotation.0 * Vec3::X;
            let up = rotation.0 * Vec3::Y;
            
            let pitch_rot = Quat::from_axis_angle(right, pitch);
            let yaw_rot = Quat::from_axis_angle(up, yaw);
            
            rotation.0 = pitch_rot * yaw_rot * rotation.0;
        }
        
        if action_state.pressed(&PlayerInput::MoveRollLeft) {
            let forward = rotation.0 * Vec3::NEG_Z;
            let roll_rot = Quat::from_axis_angle(forward, -ROLL_SPEED);
            rotation.0 = roll_rot * rotation.0;
        }
        if action_state.pressed(&PlayerInput::MoveRollRight) {
            let forward = rotation.0 * Vec3::NEG_Z;
            let roll_rot = Quat::from_axis_angle(forward, ROLL_SPEED);
            rotation.0 = roll_rot * rotation.0;
        }
        
        if action_state.pressed(&PlayerInput::MoveForward) {
            wish_dir += Vec3::NEG_Z;
        }
        if action_state.pressed(&PlayerInput::MoveBackward) {
            wish_dir += Vec3::Z;
        }
        if action_state.pressed(&PlayerInput::MoveLeft) {
            wish_dir += Vec3::NEG_X;
        }
        if action_state.pressed(&PlayerInput::MoveRight) {
            wish_dir += Vec3::X;
        }
        if action_state.pressed(&PlayerInput::MoveUp) {
            wish_dir += Vec3::Y;
        }
        if action_state.pressed(&PlayerInput::MoveDown) {
            wish_dir += Vec3::NEG_Y;
        }

        if wish_dir != Vec3::ZERO {
            let wish_dir = wish_dir.normalize();
            let world_wish_dir = rotation.0 * wish_dir;
            let movement = world_wish_dir * MOVE_SPEED;
            position.0 += movement;
        }
        
        rotation.0 = rotation.0.normalize();
    }
}

#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub id: ClientId,
    pub name: String,
    pub frags: u16,
    pub deaths: u16,
    pub score: u16,
    pub ping: u8,
    pub respawn_timer: Timer,
    pub frozen_amount: u8,
}