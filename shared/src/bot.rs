use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BotShip {
    pub wish_dir: Vec3,
}

#[derive(Debug, Deserialize)]
pub struct BotBehavior {
    /// the distance it tries to maintain from the walls
    pub wall_avoidance_distance: f32,

    /// speed it takes to change its wish direction, helps smooth over robotic-like decision making
    pub wish_dir_change_speed: f32,
    
    /// the distance at which the bot will actively back away from the target
    pub back_off_distance: f32,

    /// the kind of attack the bot will use
    pub attack_kind: BotAttackKind,
}

impl Default for BotBehavior {
    fn default() -> Self {
        Self {
            wall_avoidance_distance: 2.0,
            wish_dir_change_speed: 10.0,
            back_off_distance: 4.0,
            attack_kind: BotAttackKind::Standard { 
                target_distance: 10.0,
            },
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum BotAttackKind {
    /// Moves to the target distance and tries to stay there while attacking the target
    Standard {
        /// the distance it tries to maintain from the target
        target_distance: f32,
    },
    /// Will attempt to orbit around the target while attacking
    Aggressive {
        /// the bot tries to maintain this distance from the target
        target_distance: f32,

        /// the time it takes to change the direction it takes when orbiting around the target
        change_orbit_dir_interval: f32,

        /// the amount we blend the orbit direction with the direction to the target when further than target_distance
        orbit_dir_blend_amount: f32,

        /// the amount we blend the orbit direction with the direction to the target when backing away from the target
        orbit_dir_back_off_blend_amount: f32,

        /// the amount we blend the target direction with the orbit direction between the target_distance and back_off_distance
        /// this pulls the bot towards the target when orbiting the target, causing a swinging motion
        orbit_dir_target_blend_amount: f32,

        /// we add some randomness to the wish direction to make the bot less predictable, should be a value between 0.0 and 1.0
        wish_dir_random_factor: f32,
    }
}
