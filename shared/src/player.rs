use bevy::prelude::*;
use lightyear::prelude::ClientId;
use serde::{Deserialize, Serialize};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
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