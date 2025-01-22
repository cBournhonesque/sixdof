use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::prelude::{GameLayer, ProjectileSet};
use shared::projectiles::RayCastBullet;
use crate::lag_compensation::{LagCompensationHistory, LagCompensationHistoryBroadPhase};

/// Handles projectiles colliding with walls and enemies
pub(crate) struct ProjectilesPlugin;
impl Plugin for ProjectilesPlugin {
    fn build(&self, app: &mut App) {
        // EVENTS
        app.add_event::<BulletHitEvent>();
        // SYSTEMS
        app.add_systems(FixedPostUpdate, handle_raycast_bullet_hit.in_set(ProjectileSet::Hits));
    }
}

#[derive(Event, Debug)]
struct BulletHitEvent {
    pub shooter: Entity,
    pub target: Entity,
    pub damage: f32,
}


// TODO: be able to handle cases without lag compensation enabled! (have another system for non lag compensation?)
// TODO: be able to handle non-raycast bullets
/// Handle potential hits for an infinite speed bullet
/// - broad-phase: check raycast hits between bullet and the AABB envelope
fn handle_raycast_bullet_hit(
    tick_manager: Res<TickManager>,
    mut raycast_events: EventReader<RayCastBullet>,
    mut hit_events: EventWriter<BulletHitEvent>,
    spatial_query: SpatialQuery,
    parent_query: Query<&LagCompensationHistory>,
    // child aabb union colliders
    child_query: Query<&Parent, With<LagCompensationHistoryBroadPhase>>,
) {
    let tick = tick_manager.tick();
    for event in raycast_events.read() {
        if let Some(hit ) = spatial_query.cast_ray_predicate(
            event.source,
            event.direction,
            1000.0,
            false,
            // TODO: handle collisions with walls
            &SpatialQueryFilter::from_mask([GameLayer::LagCompensatedBroadPhase]),
            &|entity| {
                let parent_entity = child_query.get(entity).expect("the broad phase entity must have a parent").get();
                let history = parent_query.get(parent_entity).expect("all lag compensated entities must have a history");
                // the start corresponds to tick `interpolation_tick` (we interpolate between `interpolation_tick` and `interpolation_tick + 1`)
                let (source_idx, (_, (start_collider, start_position, start_rotation, _))) = history.into_iter().enumerate().find(|(_, (history_tick, _))| {
                    *history_tick == event.interpolation_tick
                }).unwrap();
                // TODO: for now, we assume that the collider itself does not change between ticks
                let (_, (_, target_position, target_rotation, _)) = history.into_iter().skip(source_idx + 1).next().unwrap();
                let interpolated_position = start_position.lerp(**target_position, event.interpolation_overstep);
                let interpolated_rotation = start_rotation.slerp(*target_rotation, event.interpolation_overstep);

                let hit = start_collider.cast_ray(
                    interpolated_position,
                    interpolated_rotation,
                    event.source,
                    event.direction.as_vec3(),
                    1000.0,
                    false,
                );
                debug!(?event, ?hit, ?tick, ?interpolated_position, ?interpolated_rotation, "Interpolated collider");
                hit.is_some()
            }
        ) {
            // the target is the parent of the collider history
            let target = child_query.get(hit.entity).expect("the broad phase entity must have a parent").get();
            let hit_event = BulletHitEvent {
                shooter: event.shooter,
                target,
                damage: 0.0,
            };
            info!(?tick, "Sending bullet hit event: {:?}", hit_event);
            hit_events.send(hit_event);
        }
    }
}