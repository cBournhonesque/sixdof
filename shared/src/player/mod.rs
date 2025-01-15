use bevy::prelude::*;


pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
    }
}

#[derive(Component, Clone)]
pub struct Player {
    pub visuals: Option<Entity>,
    pub id: u8,
    pub name: String,
    pub frags: u16,
    pub deaths: u16,
    pub score: u16,
    pub ping: u8,
    pub respawn_timer: Timer,
    pub frozen_amount: u8,
}