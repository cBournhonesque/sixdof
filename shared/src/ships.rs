use avian3d::prelude::{CoefficientCombine, Collider, CollisionLayers, Friction, RigidBody};
use bevy::ecs::bundle::Bundle;

use crate::physics::GameLayer;

// Everything inside this module is shared code between the player and the bot.
// Since every moveable "character" in our game is a ship of some kind.


/// Shared components for collision, physics, etc.
/// @todo-brian: probably want to load data from a .ron, like the friction values, because we may want that control per ship.
pub fn shared_ship_components(shape: Collider) -> impl Bundle {
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

