use avian3d::math::Vector;
use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::action_state::ActionState;
use lightyear::client::prediction::Predicted;
use lightyear::prelude::*;
use lightyear::prelude::client::Rollback;
use lightyear::prelude::server::{Replicate, SyncTarget};
use crate::player::Player;
use crate::prelude::{PlayerInput, PREDICTION_REPLICATION_GROUP_ID};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProjectileSet {
    /// Spawn projectiles
    Spawn,
    /// Handle projectile hits
    Hits,
}

pub(crate) struct ProjectilesPlugin;

impl Plugin for ProjectilesPlugin {
    fn build(&self, app: &mut App) {
        // EVENTS
        app.add_event::<RayCastBullet>();

        // SYSTEMS
        // TODO: use replicated projectiles for projectiles that can have a non-deterministic trajectory (bouncing on walls, homing missiles)
        // app.add_systems(FixedUpdate, shoot_replicated_projectiles);
        app.add_systems(FixedUpdate, shoot_projectiles.in_set(ProjectileSet::Spawn));

        // DEBUG
        // app.add_systems(FixedLast, debug_after_physics);
    }
}

// TODO: maybe make this an enum with the type of projectile?
#[derive(Component, Debug, Clone)]
pub struct Projectile;

/// Print the transform after physics have been applied
pub fn debug_after_physics(
    tick_manager: Res<TickManager>,
    rollback: Option<Res<Rollback>>,
    query: Query<
        (Entity, (&Transform, &Position, &Rotation)),
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

/// Infinite-speed bullet
/// (we make it a component so that we can display the visuals for more than 1 frame using gizmos)
#[derive(Event, Clone, Debug)]
pub struct RayCastBullet {
    pub shooter: Entity,
    pub source: Vector,
    pub direction: Dir3,
    pub interpolation_delay_ticks: u16,
    pub interpolation_overstep: f32,
}

impl Default for RayCastBullet {
    fn default() -> Self {
        Self {
            shooter: Entity::PLACEHOLDER,
            source: Vector::ZERO,
            direction: Dir3::Z,
            interpolation_delay_ticks: 0,
            interpolation_overstep: 0.0,
        }
    }
}

/// Shoot projectiles from the current weapon when the shoot action is pressed
/// The projectiles are moved by physics. This is probably unnecessary and very CPU-intensive?
/// We just need to do a raycast/shapecast from the initial bullet firing point, while tracking the speed of the bullet
pub(crate) fn shoot_projectiles(
    mut _commands: Commands,
    mut raycast_writer: EventWriter<RayCastBullet>,
    query: Query<
        (
            Entity,
            &Player,
            &Transform,
            &ActionState<PlayerInput>,
        ),
        Or<(With<Predicted>, With<Replicating>)>,
    >,
) {
    for (entity, _player, transform, action) in query.iter() {
        // NOTE: pressed lets you shoot many bullets, which can be cool

        if action.just_pressed(&PlayerInput::ShootPrimary) {
            // TODO: maybe offset the bullet a little bit from the player to avoid colliding with the player?
            raycast_writer.send(RayCastBullet {
                shooter: entity,
                source: transform.translation,
                direction: transform.forward(),
                // TODO: use values sent by the client! right now we hardcode
                interpolation_delay_ticks: 7,
                interpolation_overstep: 0.0,
            });

            // let direction = transform.forward().as_vec3();
            // // offset a little bit from the player
            // let mut new_transform = *transform;
            // new_transform.translation += 0.5 * direction;
            // commands.spawn((
            //     new_transform,
            //     Projectile,
            //     // TODO: change projectile speed
            //     LinearVelocity(direction * 5.0),
            //     // TODO: change projectile shape
            //     Collider::sphere(0.05),
            //     RigidBody::Dynamic,
            // ));


        }
    }
}