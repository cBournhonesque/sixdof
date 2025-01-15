use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component)]
pub struct GameplayEntity;

#[derive(Component, Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub enum Team {
    AntiVirus,
    Virus,
    Deathmatch,
    Spectator,
    World,
}

impl From<u8> for Team {
    fn from(value: u8) -> Self {
        match value {
            0 => Team::AntiVirus,
            1 => Team::Virus,
            2 => Team::Deathmatch,
            3 => Team::Spectator,
            4 => Team::World,
            _ => Team::Spectator,
        }
    }
}

#[derive(Component)]
pub struct Headlights;

#[derive(Component)]
pub struct HealthRegen {
    pub delay_before_heal: Timer,
    pub heal_tick_timer: Timer,
    pub amount: f32,
}

impl Default for HealthRegen {
    fn default() -> Self {
        Self {
            delay_before_heal: Timer::from_seconds(5.0, TimerMode::Once),
            heal_tick_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            amount: 1.0,
        }
    }
}

#[derive(Component, Clone, Debug)]
pub struct Health {
    current: i16,
    pub max_health: i16,
}

impl Default for Health {
    fn default() -> Self {
        Self {
            current: 100,
            max_health: 100,
        }
    }
}

impl Health {
    pub fn new(current: i16, max_health: i16) -> Self {
        Self {
            current: current,
            max_health: max_health,
        }
    }

    pub fn dead(&self) -> bool {
        self.current <= 0
    }

    pub fn current(&self) -> i16 {
        self.current
    }

    pub fn set_current(&mut self, amount: i16) {
        self.current = amount;
    }

    pub fn increment(&mut self, amount: i16) {
        self.current = (self.current + amount).min(self.max_health);
    }

    pub fn decrement(&mut self, amount: i16) {
        self.current = (self.current - amount).max(0);
    }
}

#[derive(Component)]
pub struct HealthPickupDropper {
    pub amount: i16,
}

#[derive(Event)]
pub struct DamageEvent {
    pub amount: i16,
    pub victim: Entity,
    pub instigator: Entity,
}

#[derive(Component)]
pub struct Seed(pub u8);

//======================================================================
// movers
//======================================================================
#[derive(Component)]
pub struct ButtonOnce;

//======================================================================
// hud
//======================================================================

//======================================================================
// pickups
//======================================================================
pub enum DoorKey {
    Red,
    Blue,
    Yellow,
}

#[derive(Event)]
pub struct DoorKeyPickupEvent {
    pub key: DoorKey,
}

//======================================================================
// weapons
//======================================================================

#[derive(Component)]
pub struct WantsToUseFlare;

#[derive(Component)]
pub struct WantsToUseWeapon;

#[derive(Component)]
pub struct WantsToUseSecondary;

#[derive(Component)]
pub struct HasBeenReplicatedOnce;
