use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::client::InterpolationDelay;
use shared::prelude::{GameLayer, Projectile, ProjectileSet};
use shared::projectiles::LinearProjectile;
use crate::lag_compensation::{LagCompensationHistory, LagCompensationHistoryBroadPhase};

/// Handles projectiles colliding with walls and enemies
pub(crate) struct ProjectilesPlugin;
impl Plugin for ProjectilesPlugin {
    fn build(&self, app: &mut App) {
        // EVENTS
        app.add_event::<BulletHitEvent>();
        // SYSTEMS
        app.add_systems(FixedPostUpdate, handle_linear_bullet_hit.in_set(ProjectileSet::Hits));
    }
}

#[derive(Event, Debug)]
struct BulletHitEvent {
    pub shooter: Entity,
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
    nonraycast_bullets: Query<(Entity, &Position, &LinearProjectile), With<Projectile>>,
    mut hit_events: EventWriter<BulletHitEvent>,
    spatial_query: SpatialQuery,
    parent_query: Query<&LagCompensationHistory>,
    // child aabb union colliders
    child_query: Query<&Parent, With<LagCompensationHistoryBroadPhase>>,
) {
    let tick = tick_manager.tick();
    raycast_bullets.read()
        .map(|projectile| (None, projectile.source, projectile))
        .chain(nonraycast_bullets.iter().map(|(entity, pos, projectile)| (Some(entity), pos.0, projectile)))
        .for_each(|(bullet_entity, current_pos, projectile)| {
            if let Some(hit ) = spatial_query.cast_ray_predicate(
                current_pos,
                projectile.direction,
                projectile.speed,
                false,
                // TODO: handle collisions with walls
                &SpatialQueryFilter::from_mask([GameLayer::LagCompensatedBroadPhase]),
                &|entity| {
                    let parent_entity = child_query.get(entity).expect("the broad phase entity must have a parent").get();
                    let history = parent_query.get(parent_entity).expect("all lag compensated entities must have a history");
                    let (interpolation_tick, interpolation_overstep) = InterpolationDelay { delay_ms: projectile.interpolation_delay_ms}.tick_and_overstep(tick, tick_manager.config.tick_duration);

                    // the start corresponds to tick `interpolation_tick` (we interpolate between `interpolation_tick` and `interpolation_tick + 1`)
                    let (source_idx, (_, (start_collider, start_position, start_rotation, _))) = history.into_iter().enumerate().find(|(_, (history_tick, _))| {
                        *history_tick == interpolation_tick
                    }).unwrap();

                    // TODO: for now, we assume that the collider itself does not change between ticks, so we don't have
                    //  to interpolate it
                    let (_, (_, target_position, target_rotation, _)) = history.into_iter().skip(source_idx + 1).next().unwrap();
                    let interpolated_position = start_position.lerp(**target_position, interpolation_overstep);
                    let interpolated_rotation = start_rotation.slerp(*target_rotation, interpolation_overstep);

                    let hit = start_collider.cast_ray(
                        interpolated_position,
                        interpolated_rotation,
                        current_pos,
                        projectile.direction.as_vec3(),
                        projectile.speed,
                        false,
                    );
                    debug!(?tick, ?projectile, ?interpolation_tick, ?interpolation_overstep, ?hit, ?interpolated_position, "Interpolated lag-compensation collider");
                    hit.is_some()
                }
            ) {
                // the target is the parent of the collider history
                let target = child_query.get(hit.entity).expect("the broad phase entity must have a parent").get();
                let hit_event = BulletHitEvent {
                    shooter: projectile.shooter,
                    target,
                    damage: 0.0,
                };
                info!(?tick, "Sending bullet hit event: {:?}", hit_event);
                hit_events.send(hit_event);

                // if the bullet was a projectile, despawn it
                if let Some(bullet_entity) = bullet_entity {
                    // TODO: how to make sure that the bullet is visuall despawned on the client?
                    commands.entity(bullet_entity).despawn_recursive();
                }
            }
        });
}