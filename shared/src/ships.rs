use avian3d::prelude::{AngularVelocity, CoefficientCombine, Collider, CollisionLayers, Friction, LinearVelocity, RigidBody, Rotation};
use bevy::ecs::bundle::Bundle;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_config_stack::prelude::ConfigAssetLoaderPlugin;
use serde::{Deserialize, Serialize};

// NOTE: Everything inside this module is shared code between the player and the bot.
// Since every moveable "character" in our game is a ship of some kind.

use crate::{bot::BotBehavior, physics::GameLayer};
pub type ShipId = u32;

#[derive(Component, Serialize, Deserialize, PartialEq, Debug, Default,Eq, Hash, Clone, Copy)]
pub struct ShipIndex(pub ShipId);

pub struct ShipPlugin;
impl Plugin for ShipPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ConfigAssetLoaderPlugin::<ShipsData>::new("data/ships.ron"));
    }
}

#[derive(Asset, Resource, Default, TypePath, Debug, Deserialize)]
pub struct ShipsData {
    pub player_ship: ShipId,
    pub ships: HashMap<ShipId, ShipBehavior>,
}

#[derive(Default, TypePath, Debug, Deserialize)]
pub struct ShipBehavior {
    pub name: String,
    pub accel_speed: f32,
    pub afterburner_accel_speed: f32,
    pub base_speed: f32,
    pub drag: f32,
    pub look_rotation_force: f32,
    pub max_rotation_speed: f32,
    pub roll_rotation_force: f32,
    pub rotation_damping: f32,
    pub bot_behavior: BotBehavior,
}

pub fn move_ship(
    fixed_time: &Time<Fixed>,
    behavior: &ShipBehavior,
    linear_velocity: &mut LinearVelocity,
    angular_velocity: &mut AngularVelocity,
    wish_dir: Vec3,
    // if we're using afterburners, we need to know the rotation of the ship to accelerate in the correct direction
    after_burners: Option<&Rotation>,
) {
    angular_velocity.0 *= 1.0 - behavior.rotation_damping;
            
    if angular_velocity.length_squared() > behavior.max_rotation_speed * behavior.max_rotation_speed {
        angular_velocity.0 = angular_velocity.normalize() * behavior.max_rotation_speed;
    }
    
    // apply drag
    linear_velocity.0 = apply_drag(
        linear_velocity.0,
        linear_velocity.length(),
        behavior.drag, 
        fixed_time.delta_secs()
    );

    // apply acceleration
    let current_speed = linear_velocity.dot(wish_dir);
    linear_velocity.0 += accelerate(
        wish_dir, 
        behavior.base_speed,
        current_speed,
        behavior.accel_speed,
        fixed_time.delta_secs()
    );

    // afterburners accelerate you forward
    if let Some(rotation) = after_burners {
        let wish_dir = rotation.0 * Vec3::NEG_Z;
        let current_speed = linear_velocity.dot(rotation.0 * Vec3::NEG_Z);
        linear_velocity.0 += accelerate(
            wish_dir, 
            behavior.base_speed,
            current_speed,
            behavior.afterburner_accel_speed,
            fixed_time.delta_secs()
        );
    }
}

/// Shared components for collision, physics, etc.
/// @todo-brian: probably want to load data from a .ron, like the friction values, because we may want that control per ship.
pub fn get_shared_ship_components(shape: Collider) -> impl Bundle {
    (
        shape,
        RigidBody::Dynamic,
        CollisionLayers::new(
            // We belong to the ship layer
            [GameLayer::Ship],
            // We collide with walls, other ships, and projectiles (if they have a collider)
            [
                GameLayer::Wall,
                GameLayer::Ship,
                GameLayer::Projectile,
            ]
        ),
        Friction {
            // We have little friction so that we can slide along walls instead of rolling.
            dynamic_coefficient: 0.01,
            static_coefficient: 0.1,
            combine_rule: CoefficientCombine::Min,
        },
    )
}

fn apply_drag(
    velocity: Vec3, 
    current_speed: f32, 
    drag: f32, 
    delta_seconds: f32
) -> Vec3 {
    let mut new_speed;
    let mut drop = 0.0;

    drop += current_speed * drag * delta_seconds;

    new_speed = current_speed - drop;
    if new_speed < 0.0 {
        new_speed = 0.0;
    }

    if new_speed != 0.0 {
        new_speed /= current_speed;
    }

    velocity * new_speed
}

fn accelerate(
    wish_direction: Vec3,
    wish_speed: f32,
    current_speed: f32,
    accel: f32,
    delta_seconds: f32,
) -> Vec3 {
    let add_speed = wish_speed - current_speed;

    if add_speed <= 0.0 {
        return Vec3::ZERO;
    }

    let mut accel_speed = accel * delta_seconds * wish_speed;
    if accel_speed > add_speed {
        accel_speed = add_speed;
    }

    wish_direction * accel_speed
}
