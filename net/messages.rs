use crate::player::PlayerInput;
use crate::weapons::*;
use crate::GameMode;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientMessage {
    PlayerInput(PlayerInput),
}

#[derive(Serialize, Deserialize)]
pub enum GameStateMessage {
    MapLoad(String, GameMode),
    RespawnCounter(u8),
}

#[derive(Serialize, Deserialize)]
pub enum WeaponsMessage {
    Projectile(SpawnProjectileEvent),
    DespawnProjectile(DespawnProjectileEvent),
    ShotgunFire(ShotgunFireEvent),
}
