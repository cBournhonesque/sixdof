use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use lightyear::connection::host::HostClient;
use lightyear::prelude::*;
use lightyear::utils::ready_buffer::ReadyBuffer;
use shared::{prelude::{CurrentWeaponIndex, GameLayer, PlayerInput, UniqueIdentity}, weapons::{handle_shooting, Projectile, WeaponFiredEvent, WeaponInventory, WeaponsData}};
use shared::prelude::{ProjectileInfo, Ship, WeaponsSet};

pub(crate) struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeaponFiredEventInterpolationBuffer>();
        // do not shoot a bullet twice if we are the host-server!
        app.add_systems(FixedUpdate, predicted_shoot_system
            .in_set(WeaponsSet::Shoot));

        app.add_systems(PreUpdate, buffer_fire_weapon_event
            .after(MessageSet::Receive));
        app.add_systems(PostUpdate, shoot_interpolated_bullets
            .in_set(WeaponsSet::Shoot)
            // the interpolation time is updated in the lightyear SyncSet
            .after(SyncSet::Sync)
            // make sure that any Position is propagated to Transform
            .before(PhysicsSet::Sync)
        );

        // app.add_systems(FixedPostUpdate, (
        //     projectile_predict_hit_detection_system,
        // ));
    }
}


/// When we receive FiredWeaponEvent from the server, we don't want to immediately fire the weapon.
/// Instead, we want to wait for the event tick to match the interpolated timeline.
/// We will store the events in a heap and wait until the interpolation tick matches the event tick.
#[derive(Resource, Default)]
struct WeaponFiredEventInterpolationBuffer {
    buffer: ReadyBuffer<Tick, WeaponFiredEvent>
}


/// System to fire weapon for the local client.
/// The weapon is fired in the prediction timeline.
/// Normally we would PreSpawn the projectile entities and match them with server-replicated entities; but to save
/// bandwidth we will simply spawn the projectile on the client and the server, with no replication.
// TODO(cb): this might not work if the player is 'stunned' or 'dead' on the server but can shoot on the client.
fn predicted_shoot_system(
    fixed_time: Res<Time<Fixed>>,
    mut commands: Commands,
    weapons_data: Res<WeaponsData>,
    // TODO(cb): we don't shoot again during a rollback because the bullets aren't predicted past the initial replication?
    //  think about it
    timeline: Single<&LocalTimeline, (With<Client>, Without<Rollback>, Without<HostClient>)>,
    non_predicted_controlled_player: Query<(&UniqueIdentity, &CurrentWeaponIndex), (With<Controlled>, Without<Predicted>)>,
    mut predicted_player: Query<(
        Entity,
        &Position,
        &Rotation,
        &mut WeaponInventory,
        &ActionState<PlayerInput>,
    ), With<Predicted>>,
) {
    let tick = timeline.tick();
    for (shooting_entity, position, rotation, mut inventory, action) in predicted_player.iter_mut() {
        // TODO: what is this? why don't we check the CurrentWeapon / Identity directly on the predicted entity?
        if let Some((identity, current_weapon_idx)) = non_predicted_controlled_player.iter().next() {
            handle_shooting(
                shooting_entity, 
                identity,
                tick,
                false,
                None,
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

/// Store the events in a buffer until the interpolation tick matches the event tick
fn buffer_fire_weapon_event(
    mut buffer: ResMut<WeaponFiredEventInterpolationBuffer>,
    mut events: Single<&mut MessageReceiver<WeaponFiredEvent>, (With<Client>, With<Connected>)>,
) {
    events.receive().for_each(|event| {
        buffer.buffer.push(event.fire_tick, event);
    })
}

// TODO: should this be an observer? maybe not because there could be many bullets fired per round
/// When we receive an interpolated entity with FiredWeaponEvent from the server, we make sure to fire
/// the bullet at the correct time and position
fn shoot_interpolated_bullets(
    mut commands: Commands,
    timeline: Single<&InterpolationTimeline, (With<Client>, Without<HostClient>)>,
    mut buffer: ResMut<WeaponFiredEventInterpolationBuffer>,
    ship_query: Query<&Confirmed, With<Ship>>,
) {
    let interpolate_tick = timeline.tick();
    // we wait and only pop the fire events that are more recent than the interpolate_tick
    while let Some((_, mut fired_event)) = buffer.buffer.pop_item(&interpolate_tick)  {
        // the entity here is the Confirmed entity, and we need to get the interpolated entity.
        let Ok(confirmed) = ship_query.get(fired_event.shooter_entity) else {
            error!("Could not find Confirmed ship from fired event: {:?}", fired_event);
            continue
        };

        let Some(interpolated_entity) = confirmed.interpolated else {
            error!("Could not find interpolated entity from fired event: {:?}", fired_event);
            continue
        };

        debug!(?interpolate_tick, "Trigger fired event: {:?} on interpolated entity", fired_event);
        // mark the interpolated entity as the shooter
        fired_event.shooter_entity = interpolated_entity;
        // trigger the event so we spawn the projectiles + add audio/vfx
        commands.trigger(fired_event);
    }
}

// TODO: instead of using spatial-query, we can directly use Collisions ?
/// Clients just predict the hit detection of projectiles for now.
#[allow(dead_code)]
fn projectile_predict_hit_detection_system(
    fixed_time: Res<Time<Fixed>>,
    mut commands: Commands,
    spatial_query: SpatialQuery,
    projectiles: Query<(Entity, &Position, &LinearVelocity, &ProjectileInfo), With<Projectile>>,
) {
    for (bullet_entity, current_pos, current_velocity, projectile_info) in projectiles.iter() {
        if let Some(_) = spatial_query.cast_ray(
            current_pos.0,
            Dir3::new_unchecked(current_velocity.0.normalize()),
            current_velocity.length() * fixed_time.delta_secs(),
            true,
            &mut SpatialQueryFilter {
                mask: [GameLayer::Ship, GameLayer::Wall].into(),
                ..default()
            }.with_excluded_entities([projectile_info.shooter_entity])
        ) {
            // @todo-brian: do bouncy projectiles!
            commands.entity(bullet_entity).despawn();
        }
    }
}
