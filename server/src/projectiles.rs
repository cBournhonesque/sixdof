use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::prelude::{GameLayer, ProjectileSet};
use shared::projectiles::RayCastBullet;
use crate::collider_history::LagCompensationSpawnTick;


/// Handles projectiles colliding with walls and enemies
pub(crate) struct ProjectilesPlugin;
impl Plugin for ProjectilesPlugin {
    fn build(&self, app: &mut App) {
        // EVENTS
        app.add_event::<BulletHitEvent>();
        // SYSTEMS
        app.add_systems(FixedUpdate, handle_raycast_bullet_hit.in_set(ProjectileSet::Hits));
    }
}

#[derive(Event, Debug)]
struct BulletHitEvent {
    pub shooter: Entity,
    pub target: Entity,
    pub damage: f32,
}

/// Handle potential hits for an infinite speed bullet
fn handle_raycast_bullet_hit(
    tick_manager: Res<TickManager>,
    mut raycast_events: EventReader<RayCastBullet>,
    mut hit_events: EventWriter<BulletHitEvent>,
    spatial_query: SpatialQuery,
    collider_history: Query<(&LagCompensationSpawnTick, &Parent)>
) {
    let tick = tick_manager.tick();
    for event in raycast_events.read() {
        if let Some(hit ) = spatial_query.cast_ray_predicate(
            event.source,
            event.direction,
            1000.0,
            false,
            // TODO: handle collisions with walls
            // TODO: maybe we don't need to exclude `event.shooter` so we can re-use the same filter?
            &SpatialQueryFilter::from_mask([GameLayer::Bot]).with_excluded_entities([event.shooter]),
            &|entity| {
                // TODO: be able to handle cases without lag compensation enabled!
                // Skip entities (return true) that don't belong to the right lag-compensated tick
                let Ok((spawn_tick, _)) = collider_history.get(entity) else {
                    return true;
                };
                (tick - event.interpolation_delay_ticks) != spawn_tick.0
            }
        ) {
            // the target is the parent of the collider history
            let target =  collider_history.get(hit.entity).expect("all collider histories must have a parent").1.get();
            let hit_event = BulletHitEvent {
                shooter: event.shooter,
                target,
                damage: 0.0,
            };
            error!(?tick, "Sending bullet hit event: {:?}", hit_event);
            hit_events.send(hit_event);
        }
    }
}