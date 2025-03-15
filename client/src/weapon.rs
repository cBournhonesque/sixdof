use std::time::Duration;
use avian3d::prelude::{LinearVelocity, Position, Rotation, SpatialQuery, SpatialQueryFilter};
use bevy::{prelude::*};
use leafwing_input_manager::prelude::ActionState;
use lightyear::{shared::replication::components::Controlled};
use lightyear::prelude::{is_host_server, TickManager};
use lightyear::prelude::client::*;
use shared::{prelude::{CurrentWeaponIndex, GameLayer, PlayerInput, UniqueIdentity}, weapons::{handle_shooting, Projectile, WeaponFiredEvent, WeaponInventory, WeaponsData}};
use shared::prelude::{DespawnAfter, WeaponsSet};

pub(crate) struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        // do not shoot a bullet twice if we are the host-server!
        app.add_systems(FixedUpdate, shoot_system
            .in_set(WeaponsSet::Shoot)
            .run_if(not(is_host_server)));

        // TODO: currently we run this in FixedUpdate to fire at the exact tick that the server fired the bullet.
        app.add_systems(FixedUpdate, shoot_interpolated_bullets
            .in_set(WeaponsSet::Shoot)
            .run_if(not(is_host_server).and(not(is_in_rollback))));
        // app.add_systems(FixedPostUpdate, (
        //     projectile_predict_hit_detection_system,
        // ));

    }
}


fn shoot_system(
    fixed_time: Res<Time<Fixed>>,
    mut commands: Commands,
    weapons_data: Res<WeaponsData>,
    rollback: Option<Res<Rollback>>,
    non_predicted_controlled_player: Query<(&UniqueIdentity, &CurrentWeaponIndex), (With<Controlled>, Without<Predicted>)>,
    tick_manager: Res<TickManager>,
    mut predicted_player: Query<(
        Entity,
        &Position,
        &Rotation,
        &mut WeaponInventory,
        &ActionState<PlayerInput>,
    ), With<Predicted>>,
) {
    // TODO(cb): we don't shoot again during a rollback because the bullets aren't predicted past the initial replication?
    //  think about it
    let rolling_back = rollback.map_or(false, |r| r.is_rollback());
    if rolling_back {
        return;
    }

    let tick = tick_manager.tick();
    for (shooting_entity, position, rotation, mut inventory, action) in predicted_player.iter_mut() {
        if let Some((identity, current_weapon_idx)) = non_predicted_controlled_player.iter().next() {
            handle_shooting(
                shooting_entity, 
                identity,
                tick,
                false,
                position,
                rotation,
                current_weapon_idx.0,
                &mut inventory, 
                action, 
                &fixed_time, 
                &weapons_data, 
                &mut commands
            );
        }
    }
}

// TODO: should this be an observer? maybe not because there could be many bullets fired per round
/// When we receive an interpolated entity with FiredWeaponEvent from the server, we make sure to fire
/// the bullet at the correct time and position
fn shoot_interpolated_bullets(
    mut commands: Commands,
    tick_manager: Res<TickManager>,
    weapons_data: Res<WeaponsData>,
    connection: Res<ConnectionManager>,
    interpolated_bullet: Query<
        (Entity, &WeaponFiredEvent),
        // optimization trick to avoid using Added
        (With<Interpolated>, Without<Position>)
    >
) {
    let interpolate_tick = connection.interpolation_tick(tick_manager.as_ref());
    interpolated_bullet.iter().for_each(|(entity, fired_event)| {
        // TODO: 2 options
        //  - wait for the correct tick to arrive
        //  - fire the bullet immediately with some position adjustments based on the tick diff
        if interpolate_tick < fired_event.fire_tick {
            return;
        }
        assert_eq!(fired_event.fire_tick, interpolate_tick);
        if let Some(weapon_data) = weapons_data.weapons.get(&fired_event.weapon_index) {
            info!(?fired_event, "Adding components for interpolated bullet!");
            commands.entity(entity)
                .insert((
                    Position(fired_event.fire_origin),
                    LinearVelocity(fired_event.fire_direction.as_vec3() * weapon_data.projectile.speed),
                    DespawnAfter(Timer::new(Duration::from_millis(weapon_data.projectile.lifetime_millis), TimerMode::Once)),
                ));
            commands.send_event(fired_event.clone());
        }
    })
}

/// Clients just predict the hit detection of projectiles for now.
#[allow(dead_code)]
fn projectile_predict_hit_detection_system(
    fixed_time: Res<Time<Fixed>>,
    mut commands: Commands,
    spatial_query: SpatialQuery,
    projectiles: Query<(Entity, &Position, &LinearVelocity, &WeaponFiredEvent), With<Projectile>>,
) {
    for (bullet_entity, current_pos, current_velocity, fired_event) in projectiles.iter() {
        if let Some(_) = spatial_query.cast_ray(
            current_pos.0,
            Dir3::from_xyz(current_velocity.0.x, current_velocity.0.y, current_velocity.0.z).unwrap_or(fired_event.fire_direction),
            current_velocity.length() * fixed_time.delta_secs(),
            true,
            &mut SpatialQueryFilter {
                mask: [GameLayer::Ship, GameLayer::Wall].into(),
                ..default()
            }.with_excluded_entities([fired_event.shooter_entity])
        ) {
            // @todo-brian: do bouncy projectiles!
            commands.entity(bullet_entity).despawn_recursive();
        }
    }
}
