use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::{*, client::*};
use serde::{Deserialize, Serialize};
use crate::prelude::PlayerInput;

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
    // apply inputs either on predicted entities on the client, or replicating entities on the server
    Or<(With<Predicted>, With<Replicating>)>>
) {
    for (player, mut position, mut rotation, action_state) in query.iter_mut() {
        // TODO: handle inputs
        if action_state.pressed(&PlayerInput::MoveForward) {
            info!("Move forward!");
        }
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