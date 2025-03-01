use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use lightyear::{prelude::client::{Predicted, Rollback}, shared::replication::components::Controlled};
use shared::{prelude::{CurrentWeaponIndex, PlayerInput, UniqueIdentity}, weapons::{handle_shooting, WeaponInventory, WeaponsData}};

pub(crate) struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, shoot_system);
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
    let rolling_back = rollback.map_or(false, |r| r.is_rollback());
    if rolling_back {
        return;
    }

    for (shooting_entity, transform, mut inventory, action) in predicted_player.iter_mut() {
        if let Some((identity, current_weapon_idx)) = non_predicted_controlled_player.iter().next() {
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
}
