use avian3d::prelude::{LinearVelocity, PhysicsStepSet, Position, Rotation, SpatialQueryFilter};
use bevy::math::NormedVectorSpace;
use bevy::prelude::*;
use lightyear::prelude::{Replicating, ServerConnectionManager, TickManager, ToClients};
use shared::{prelude::{Damageable, PlayerInput, UniqueIdentity}, weapons::{handle_shooting, CurrentWeaponIndex, ProjectileHitEvent, WeaponInventory, WeaponsData}};
use leafwing_input_manager::prelude::ActionState;
use lightyear::prelude::client::InterpolationDelay;
use lightyear_avian::prelude::LagCompensationSpatialQuery;
use shared::prelude::{GameLayer, Projectile, ProjectileInfo, WeaponFiredEvent, WeaponsSet};

/// Handles projectiles colliding with walls and enemies
pub(crate) struct WeaponsPlugin;


impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ProjectileHitEvent>();
        app.add_systems(Update, weapon_switch_system);
        app.add_systems(FixedPostUpdate, projectile_hit_system.run_if(resource_exists::<WeaponsData>));
        // lag compensation collisions must run after the SpatialQuery has been updated
        app.add_systems(FixedPostUpdate, bullet_hit_detection.after(PhysicsStepSet::SpatialQuery));
        app.add_systems(FixedUpdate, shoot_system
            .in_set(WeaponsSet::Shoot)
            .run_if(resource_exists::<WeaponsData>)
        );
    }
}

fn weapon_switch_system(
    mut current_weapon_idx: Query<(&mut CurrentWeaponIndex, &WeaponInventory, &ActionState<PlayerInput>)>,
) {
    for (mut current_weapon_idx, inventory, action) in current_weapon_idx.iter_mut() {
        if action.just_pressed(&PlayerInput::NextWeapon) {
            current_weapon_idx.next_weapon(&inventory.weapons);
        }
        if action.just_pressed(&PlayerInput::PreviousWeapon) {
            current_weapon_idx.previous_weapon(&inventory.weapons);
        }
        if action.just_pressed(&PlayerInput::Weapon1) {
            if inventory.weapons.get(&0).is_some() {
                current_weapon_idx.0 = 0;
            }
        }
        if action.just_pressed(&PlayerInput::Weapon2) {
            if inventory.weapons.get(&1).is_some() {
                current_weapon_idx.0 = 1;
            }
        }
        if action.just_pressed(&PlayerInput::Weapon3) {
            if inventory.weapons.get(&2).is_some() {
                current_weapon_idx.0 = 2;
            }
        }
        if action.just_pressed(&PlayerInput::Weapon4) {
            if inventory.weapons.get(&3).is_some() {
                current_weapon_idx.0 = 3;
            }
        }
        if action.just_pressed(&PlayerInput::Weapon5) {
            if inventory.weapons.get(&4).is_some() {
                current_weapon_idx.0 = 4;
            }
        }
    }
}

fn projectile_hit_system(
    mut commands: Commands,
    mut events: EventReader<ProjectileHitEvent>,
    mut weapons_data: ResMut<WeaponsData>,
    mut damageables: Query<&mut Damageable>,
) {
    for event in events.read() {
        // by this point the projectile itself has already been queued for despawn so we dont need to worry about the projectile itself
        if let Some(weapon_data) = weapons_data.weapons.get_mut(&event.weapon_index) {
            // @todo-brian: apply splash damage
            if let Some(entity_hit) = event.entity_hit {
                if let Ok(mut damageable) = damageables.get_mut(entity_hit) {
                    damageable.health = damageable.health.saturating_sub(weapon_data.projectile.direct_damage);
                    if damageable.health <= 0 {
                        commands.entity(entity_hit).despawn_recursive();
                    }
                }
            }
        }
    }
}

// TODO: be able to handle cases without lag compensation enabled! (have another system for non lag compensation?)
/// Handle potential hits for a linear projectile. The projectile is not actually spawned
/// - broad-phase: check hits via raycast between bullet and the AABB envelope history
/// - narrow-phase: if there is a broadphase hit, check hits via raycast between bullet and the interlated history collider
fn bullet_hit_detection(
    mut commands: Commands,
    tick_manager: Res<TickManager>,
    nonraycast_bullets: Query<(Entity, &Position, &LinearVelocity, &ProjectileInfo), With<Projectile>>,
    mut hit_events: EventWriter<ProjectileHitEvent>,
    query: LagCompensationSpatialQuery,
    manager: Res<ServerConnectionManager>,
    client_query: Query<&InterpolationDelay>,
) {
    let tick = tick_manager.tick();
    nonraycast_bullets.iter()
        .for_each(|(bullet_entity, current_pos, current_velocity, projectile_info)| {

        let delay = match projectile_info.shooter_id {
            UniqueIdentity::Player(client_id) => {
                let Ok(delay) = manager
                    .client_entity(client_id)
                    .map(|client_entity| client_query.get(client_entity).unwrap())
                else {
                    error!("Could not retrieve InterpolationDelay for client {client_id:?}");
                    return;
                };
                *delay
            }
            UniqueIdentity::Bot(_) => InterpolationDelay {
                delay_ms: 0,
            }
        };

        if let Some(hit) = query.cast_ray(
            delay,
            current_pos.0,
            Dir3::new_unchecked(current_velocity.0.normalize()),
            current_velocity.norm(),
            false,
            &mut SpatialQueryFilter {
                mask: [GameLayer::Ship, GameLayer::Wall].into(),
                ..default()
            }.with_excluded_entities([projectile_info.shooter_entity])
        ) {
            let hit_event = ProjectileHitEvent {
                shooter_id: projectile_info.shooter_id,
                weapon_index: projectile_info.weapon_index,
                projectile_entity: bullet_entity,
                entity_hit: Some(hit.entity),
            };
            info!(?tick, "Sending bullet hit event: {:?}", hit_event);
            hit_events.send(hit_event);

            // if the bullet was a projectile, despawn it
            // TODO: how to make sure that the bullet is visually despawned on the client?
            //      @comment-brian: check out client/src/weapon.rs, I now predict the hit detection of projectiles for now.
            //                      And I think that's what you want probably? I'll have to think about it more.
            commands.entity(bullet_entity).despawn_recursive();
        }
    });
}

fn shoot_system(
    fixed_time: Res<Time<Fixed>>,
    mut commands: Commands,
    weapons_data: Res<WeaponsData>,
    tick_manager: Res<TickManager>,
    mut to_clients: ResMut<Events<ToClients<WeaponFiredEvent>>>,
    mut replicated_player: Query<(
        Entity,
        &Position,
        &Rotation,
        &UniqueIdentity,
        &CurrentWeaponIndex,
        &mut WeaponInventory,
        &ActionState<PlayerInput>,
    ), With<Replicating>>,
) {
    let tick = tick_manager.tick();
    for (shooting_entity, position, rotation, identity, current_weapon_idx, mut inventory, action) in replicated_player.iter_mut() {
        handle_shooting(
            shooting_entity, 
            identity,
            tick,
            true,
            Some(to_clients.as_mut()),
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