use avian3d::math::Vector;
use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::action_state::ActionState;
use lightyear::client::prediction::Predicted;
use lightyear::prelude::*;
use lightyear::prelude::client::{InterpolationDelay, Rollback};
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
        app.add_event::<LinearProjectile>();

        // SYSTEMS
        // TODO: shouldn't the projectile be shot from PostUpdate? after physics have run?
        //  (so that the direction in which we're shooting the bullet is correct,
        //  because as it is we are basically using the Transform from the previous frame)

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
#[allow(dead_code)]
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

// TODO: maybe have a separate event for ray-cast vs slow bullets?
/// Bullet that shoots in a straight line
#[derive(Event, Clone, Debug)]
pub struct LinearProjectile {
    pub shooter: Entity,
    pub source: Vector,
    pub direction: Dir3,
    pub speed: f32,
    pub interpolation_delay_ms: u16,
}

impl Default for LinearProjectile {
    fn default() -> Self {
        Self {
            shooter: Entity::PLACEHOLDER,
            source: Vector::ZERO,
            direction: Dir3::Z,
            // the default is to shoot raycast bullets
            speed: 1000.0,
            interpolation_delay_ms: 0,
        }
    }
}

/// Shoot projectiles from the current weapon when the shoot action is pressed
/// The projectiles are moved by physics. This is probably unnecessary and very CPU-intensive?
/// We just need to do a raycast/shapecast from the initial bullet firing point, while tracking the speed of the bullet
pub(crate) fn shoot_projectiles(
    mut commands: Commands,
    mut event_writer: EventWriter<LinearProjectile>,
    query: Query<
        (
            Entity,
            &Player,
            &Transform,
            &ActionState<PlayerInput>,
        ),
        Or<(With<Predicted>, With<Replicating>)>,
    >,
    tick_manager: Res<TickManager>,
    connection_manager: Option<Res<ServerConnectionManager>>,
    client_query: Query<&InterpolationDelay>,
) {
    let tick = tick_manager.tick();
    for (entity, player, transform, action) in query.iter() {
        if action.just_pressed(&PlayerInput::ShootPrimary) || action.just_pressed(&PlayerInput::ShootSecondary) {
            let mut linear_bullet_event = LinearProjectile {
                shooter: entity,
                source: transform.translation,
                direction: transform.forward(),
                ..default()
            };
            // TODO: should the interpolation values be populated here or read on the client entity?
            // on the server, populate the interpolation delay values
            if let Some(Ok(delay)) = connection_manager.as_ref().map(|m| client_query.get(m.client_entity(player.id).unwrap())) {
                // TODO: we should use the delay at the time the bullet was fired, not the latest InterpolationDelay that
                //  we have received
                linear_bullet_event.interpolation_delay_ms = delay.delay_ms;
            }
            // TODO: can we unify raycast and non-raycast bullets?
            if action.just_pressed(&PlayerInput::ShootPrimary) {
                // ray cast bullet (infinite speed)
                // we don't need to spawn an entity, we will just do an instant raycast checkk
                linear_bullet_event.speed = 1000.0;
                debug!(?tick, ?linear_bullet_event, "Shoot raycast LinearBulletEvent");
                // TODO: maybe offset the bullet a little bit from the player to avoid colliding with the player?
                event_writer.send(linear_bullet_event);

            } else {
                // non-raycast bullet, we will spawn a non-networked entity to keep track of the position
                // of the bullet
                let bullet_speed = 10.0;
                linear_bullet_event.speed = bullet_speed;
                debug!(?tick, ?linear_bullet_event, "Shoot non-raycast LinearBullet");
                commands.spawn((
                    RigidBody::Kinematic,
                    Position(transform.translation),
                    // TODO: this is not needed on the client!
                    // we include this component on the entity because we want to use the interpolation_delay
                    // at the time the bullet was fired
                    linear_bullet_event,
                    LinearVelocity(transform.forward() * bullet_speed),
                    Projectile
                ));
            }
            // TODO: maybe offset the bullet a little bit from the player to avoid colliding with the player?
        }
    }
}