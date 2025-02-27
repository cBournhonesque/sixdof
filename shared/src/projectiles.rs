use avian3d::position::Rotation;
use avian3d::prelude::{Collider, LinearVelocity, Position, RigidBody};
use bevy::prelude::*;
use leafwing_input_manager::action_state::ActionState;
use lightyear::client::prediction::Predicted;
use lightyear::prelude::*;
use lightyear::prelude::client::Rollback;
use lightyear::prelude::server::{Replicate, SyncTarget};
use crate::player::Player;
use crate::prelude::{PlayerInput, PREDICTION_REPLICATION_GROUP_ID};
use crate::weapons::Projectile;

pub(crate) struct ProjectilesPlugin;

impl Plugin for ProjectilesPlugin {
    fn build(&self, app: &mut App) {
        // SYSTEMS
        // TODO: use replicated projectiles for projectiles that can have a non-deterministic trajectory (bouncing on walls, homing missiles)
        // app.add_systems(FixedUpdate, shoot_replicated_projectiles);
        //app.add_systems(FixedUpdate, shoot_projectiles);

        // DEBUG
        // app.add_systems(FixedLast, debug_after_physics);
    }
}

/// Print the transform after physics have been applied
pub fn debug_after_physics(
    tick_manager: Res<TickManager>,
    rollback: Option<Res<Rollback>>,
    query: Query<
        (Entity, (&Position, &Rotation)),
        (With<Projectile>, Or<(With<Predicted>, With<Replicating>)>)
    >
) {
    let tick = rollback.as_ref().map_or(tick_manager.tick(), |r| {
        tick_manager.tick_or_rollback_tick(r.as_ref())
    });
    let is_rollback = rollback.map_or(false, |r| r.is_rollback());
    for (entity, info) in query.iter() {
        info!(
            ?is_rollback,
            ?tick,
            ?entity,
            ?info,
            "After Physics"
        );
    }
}

/// Shoot projectiles from the current weapon when the shoot action is pressed
/// These projectiles are pre-spawned on the client, and replicated from the server
pub(crate) fn shoot_replicated_projectiles(
    tick_manager: Res<TickManager>,
    mut commands: Commands,
    identity: NetworkIdentity,
    query: Query<
        (
            &Player,
            &Transform,
            &ActionState<PlayerInput>,
        ),
        Or<(With<Predicted>, With<Replicating>)>,
    >,
) {
    let tick = tick_manager.tick();
    for (player, transform, action) in query.iter() {
        // NOTE: pressed lets you shoot many bullets, which can be cool
        if action.just_pressed(&PlayerInput::ShootPrimary) {
            let direction = transform.forward().as_vec3();

            // offset a little bit from the player
            let mut new_transform = *transform;
            new_transform.translation += 0.5 * direction;
            let projectile = (
                new_transform,
                Projectile,
                // TODO: change projectile speed
                LinearVelocity(direction * 5.0),
                // TODO: change projectile shape
                Collider::sphere(0.1),
                // the projectile will be spawned on both client (in the predicted timeline) and the server
                PreSpawnedPlayerObject::default(),
                RigidBody::Dynamic,
            );
            info!(?tick, ?new_transform, "SpawnReplicatedBullet");

            // on the server, spawn and replicate the projectile
            if identity.is_server() {
                commands.spawn((
                    projectile,
                    Replicate {
                        sync: SyncTarget {
                            // the bullet is predicted for the client who shot it
                            prediction: NetworkTarget::Single(player.id),
                            // the bullet is interpolated for other clients
                            interpolation: NetworkTarget::AllExceptSingle(player.id),
                        },
                        // NOTE: all predicted entities need to have the same replication group
                        group: ReplicationGroup::new_id(PREDICTION_REPLICATION_GROUP_ID),
                        ..default()
                    },
                ));
            } else {
                commands.spawn(projectile);
            }
        }
    }
}

/// Shoot projectiles from the current weapon when the shoot action is pressed
pub(crate) fn shoot_projectiles(
    mut commands: Commands,
    query: Query<
        (
            &Player,
            &Transform,
            &ActionState<PlayerInput>,
        ),
        Or<(With<Predicted>, With<Replicating>)>,
    >,
) {
    for (_player, transform, action) in query.iter() {
        // NOTE: pressed lets you shoot many bullets, which can be cool
        if action.just_pressed(&PlayerInput::ShootPrimary) {
            let direction = transform.forward().as_vec3();

            // offset a little bit from the player
            let mut new_transform = *transform;
            new_transform.translation += 0.5 * direction;
            commands.spawn((
                new_transform,
                Projectile,
                // TODO: change projectile speed
                LinearVelocity(direction * 5.0),
                // TODO: change projectile shape
                Collider::sphere(0.1),
                RigidBody::Dynamic,
            ));
        }
    }
}
