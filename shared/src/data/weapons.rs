use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::audio::SoundEmitterBehavior;

/// A weapon behavior is basically what it sounds like, 
/// it defines all the behaviors of a weapon.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct WeaponBehavior {
    /// The human readable name of the weapon.
    pub name: String,
    /// The description of the weapon.
    pub description: String,
    /// The positions of the barrels of the weapon.
    pub barrel_positions: Vec<Vec3>,
    /// The mode of the weapon.
    pub barrel_mode: BarrelMode,
    /// The mode of the weapon.
    pub fire_mode: FireMode,
    /// The crosshair of the weapon.
    pub crosshair: CrosshairConfiguration,
    /// The projectile behavior of the weapon.
    pub projectile: ProjectileBehavior,
    /// The starting ammo of the weapon.
    pub starting_ammo: u32,
    /// The sound emitter behavior of the firing sound of the weapon.
    pub firing_sound: SoundEmitterBehavior,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ProjectileBehavior {
    pub speed: f32,
    /// The lifetime of the projectile in milliseconds before it is removed from the world. 
    /// Will attempt to apply splash damage upon removal.
    pub lifetime_millis: u64,
    pub direct_damage: u16,
    pub splash_damage_radius: f32,
    pub splash_damage_max: u16,
    pub splash_damage_min: u16,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum BarrelMode {
    /// All barrels fire at the same time.
    Simultaneous,
    /// Barrels fire one after the other.
    Sequential,
}

impl Default for BarrelMode {
    fn default() -> Self {
        Self::Simultaneous
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum FireMode {
    /// An automatic weapon just fires continuously with a delay between each shot.
    Auto {
        delay_millis: u64,
    },
    /// A burst fires a number of shots in a burst, with a delay between each shot.
    Burst {
        /// The number of shots in a burst.
        shots: u32,
        /// The delay between each shot in a burst.
        delay_millis: u64,
        /// The delay after the burst is finished before starting another burst.
        delay_after_burst_millis: u64,
    },
}

impl Default for FireMode {
    fn default() -> Self {
        Self::Auto { delay_millis: 100 }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CrosshairConfiguration {
    pub color: Color,

    /// The image to use for the crosshair. 
    /// Relative to assets/crosshairs/
    pub image: String,
}

impl Default for CrosshairConfiguration {
    fn default() -> Self {
        Self { color: Color::WHITE, image: "kenney_crosshair_pack/crosshair018.png".to_string() }
    }
}
