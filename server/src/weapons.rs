use bevy::prelude::*;
use lightyear::prelude::Replicating;
use shared::{prelude::{Damageable, PlayerInput, UniqueIdentity}, weapons::{handle_shooting, CurrentWeaponIndex, ProjectileHitEvent, WeaponInventory, WeaponsData}};
use leafwing_input_manager::prelude::ActionState;

/// Handles projectiles colliding with walls and enemies
pub(crate) struct WeaponsPlugin;
impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ProjectileHitEvent>();
        app.add_systems(Update, weapon_switch_system);
        app.add_systems(FixedPostUpdate, projectile_hit_system.run_if(resource_exists::<WeaponsData>));
        app.add_systems(FixedUpdate, shoot_system.run_if(resource_exists::<WeaponsData>));
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
                    if damageable.health == 0 {
                        commands.entity(entity_hit).despawn_recursive();
                    }
                }
            }
        }
    }
}

fn shoot_system(
    fixed_time: Res<Time<Fixed>>,
    mut commands: Commands,
    weapons_data: Res<WeaponsData>,
    mut replicated_player: Query<(
        Entity,
        &Transform,
        &UniqueIdentity,
        &CurrentWeaponIndex,
        &mut WeaponInventory,
        &ActionState<PlayerInput>,
    ), With<Replicating>>,
) {
    for (shooting_entity, transform, identity, current_weapon_idx, mut inventory, action) in replicated_player.iter_mut() {
        handle_shooting(
            shooting_entity, 
            identity, 
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