//! Maintain a history of colliders for lag compensation

use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::prelude::ProjectileSet;

pub struct LagCompensationPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum LagCompensationSet {
    /// Update the collider history for all colliders
    UpdateHistory,
}

impl Plugin for LagCompensationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (delete_collider_history, update_collider_history).chain().in_set(LagCompensationSet::UpdateHistory));

        app.configure_sets(FixedUpdate, LagCompensationSet::UpdateHistory.before(ProjectileSet::Hits));
    }
}

/// Max number of ticks that we are keeping a collider in the history
// 20 * 16ms = 320ms
pub const MAX_COLLIDER_HISTORY_TICKS: i16 = 20;


/// Tag for colliders that should use lag compensation to estimate collisions with bullets
#[derive(Component)]
struct LagCompensated;


// /// Store a history of c
// #[derive(Component)]
// struct ColliderHistory(Vec<Entity>);

/// Tick at which the collider was spawned
/// The collider shows the exact Position/Rotation of their parent entity at this tick
#[derive(Component)]
pub(crate) struct LagCompensationSpawnTick(pub(crate) Tick);


type ColliderData = (
    &'static Collider,
    &'static Position,
    &'static Rotation,
    &'static CollisionLayers
);


// /// Observer that will add a ColliderHistory component to every entity
// /// that has a LagCompensated component
// fn add_collider_history(
//     trigger: Trigger<OnAdd, LagCompensated>,
//     mut commands: Commands,
// ) {
//
//     commands.entity(trigger.entity()).insert(ColliderHistory(Vec::new()));
// }


/// For each lag-compensated collider, store every tick a copy of the collider
/// that we can use to rewind collisions
///
/// We need: Collider, Position, Rotation, ColliderLayers for a collider to be able
/// to be used in spatial queries
fn update_collider_history(
    mut commands: Commands,
    tick_manager: Res<TickManager>,
    colliders: Query<(Entity, ColliderData), With<LagCompensated>>,
) {
    let tick = tick_manager.tick();
    colliders.iter().for_each(|(entity, (collider, position, rotation, collision_layers))| {
        // spawn a copy collider for the current tick, which will be part of the history
        let child = commands.spawn((
            LagCompensationSpawnTick(tick),
            collider.clone(),
            position.clone(),
            rotation.clone(),
            collision_layers.clone(),
        )).id();
        commands.entity(entity).add_child(child);
    });
}

/// Delete old history elements that are no longer needed
fn delete_collider_history(
    tick_manager: Res<TickManager>,
    mut commands: Commands,
    colliders: Query<(Entity, &LagCompensationSpawnTick)>,
) {
    let tick = tick_manager.tick();
    colliders.iter().for_each(|(entity, spawn_tick)| {
        if tick - spawn_tick.0 > MAX_COLLIDER_HISTORY_TICKS {
            // delete the entity
            commands.entity(entity).despawn_recursive();
        }
    });
}