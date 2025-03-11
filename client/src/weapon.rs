use avian3d::prelude::{LinearVelocity, Position, SpatialQuery, SpatialQueryFilter};
use bevy::{math::NormedVectorSpace, prelude::*};
use leafwing_input_manager::prelude::ActionState;
use lightyear::{prelude::client::{Predicted, Rollback}, shared::replication::components::Controlled};
use lightyear::prelude::{is_host_server, NetworkIdentity};
use shared::{prelude::{CurrentWeaponIndex, GameLayer, PlayerInput, UniqueIdentity}, weapons::{handle_shooting, Projectile, ProjectileHitEvent, WeaponFiredEvent, WeaponInventory, WeaponsData}};

pub(crate) struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        // do not shoot a bullet twice if we are the host-server!
        app.add_systems(FixedUpdate, shoot_system.run_if(not(is_host_server)));
        app.add_systems(FixedPostUpdate, (
            projectile_predict_hit_detection_system,
        ));
    }
}


fn shoot_system(
    fixed_time: Res<Time<Fixed>>,
    mut commands: Commands,
    weapons_data: Res<WeaponsData>,
    rollback: Option<Res<Rollback>>,
    non_predicted_controlled_player: Query<(&UniqueIdentity, &CurrentWeaponIndex), (With<Controlled>, Without<Predicted>)>,
    mut predicted_player: Query<(
        Entity,
        &Transform,
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

    for (shooting_entity, transform, mut inventory, action) in predicted_player.iter_mut() {
        if let Some((identity, current_weapon_idx)) = non_predicted_controlled_player.iter().next() {
            handle_shooting(
                shooting_entity, 
                identity,
                false,
                transform, 
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

/// Clients just predict the hit detection of projectiles for now.
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
