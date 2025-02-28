use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::client::InterpolationDelay;
use lightyear_avian::prelude::LagCompensationSpatialQuery;
use shared::prelude::{GameLayer, Projectile};
use shared::projectiles::{LinearProjectile, Shooter};

/// Handles projectiles colliding with walls and enemies
pub(crate) struct ProjectilesPlugin;
impl Plugin for ProjectilesPlugin {
    fn build(&self, app: &mut App) {
        // EVENTS
        app.add_event::<BulletHitEvent>();
        // SYSTEMS
        app.add_systems(FixedPostUpdate, handle_linear_bullet_hit.after(PhysicsStepSet::SpatialQuery));
    }
}

#[derive(Event, Debug)]
struct BulletHitEvent {
    pub shooter: ClientId,
    pub target: Entity,
    pub damage: f32,
}


// TODO: be able to handle cases without lag compensation enabled! (have another system for non lag compensation?)
/// Handle potential hits for a linear projectile. The projectile is not actually spawned
/// - broad-phase: check hits via raycast between bullet and the AABB envelope history
/// - narrow-phase: if there is a broadphase hit, check hits via raycast between bullet and the interlated history collider
fn handle_linear_bullet_hit(
    mut commands: Commands,
    tick_manager: Res<TickManager>,
    mut raycast_bullets: EventReader<LinearProjectile>,
    nonraycast_bullets: Query<(Entity, &Shooter, &Position, &LinearProjectile), With<Projectile>>,
    mut hit_events: EventWriter<BulletHitEvent>,
    query: LagCompensationSpatialQuery,
    manager: Res<ServerConnectionManager>,
    client_query: Query<&InterpolationDelay>,
) {
    let tick = tick_manager.tick();
    raycast_bullets.read()
        .map(|projectile| (None, projectile.shooter, projectile.source, projectile))
        .chain(nonraycast_bullets.iter().map(|(entity, shooter, pos, projectile)| (Some(entity), shooter.0, pos.0, projectile)))
        .for_each(|(bullet_entity, shooter, current_pos, projectile)| {

            let Ok(delay) = manager
                .client_entity(shooter)
                .map(|client_entity| client_query.get(client_entity).unwrap())
            else {
                error!("Could not retrieve InterpolationDelay for client {shooter:?}");
                return;
            };
            //dbg!(&delay);
            if let Some(hit) = query.cast_ray(
                *delay,
                current_pos,
                projectile.direction,
                projectile.speed,
                false,
                &mut SpatialQueryFilter::from_mask(GameLayer::Player),
            ) {
                let hit_event = BulletHitEvent {
                    shooter,
                    target: hit.entity,
                    damage: 0.0,
                };
                info!(?tick, "Sending bullet hit event: {:?}", hit_event);
                dbg!("HIT");
                hit_events.send(hit_event);

                // if the bullet was a projectile, despawn it
                if let Some(bullet_entity) = bullet_entity {
                    // TODO: how to make sure that the bullet is visuall despawned on the client?
                    commands.entity(bullet_entity).despawn_recursive();
                }
            }
        });
}