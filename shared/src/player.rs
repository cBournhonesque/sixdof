use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_config_stack::prelude::ConfigAssetLoaderPlugin;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::{*, client::*};
use serde::{Deserialize, Serialize};
use crate::{prelude::PlayerInput, ships::{move_ship, ShipIndex, ShipsData}};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, move_player.run_if(resource_exists::<ShipsData>));
        // app.add_systems(FixedUpdate, debug_input);
        // app.add_systems(FixedPostUpdate, debug_after_sync.after(PhysicsSet::Sync));
    }
}

/// Print the inputs at FixedUpdate, after they have been updated on the client/server
/// Also prints the Transform before `move_player` is applied (inputs handled)
pub fn debug_input(
    tick_manager: Res<TickManager>,
    rollback: Option<Res<Rollback>>,
    query: Query<(Entity, &ActionState<PlayerInput>, (&Transform, &Position, &Rotation)),
        Or<(With<Predicted>, With<Replicating>)>>
) {
    let tick = rollback.as_ref().map_or(tick_manager.tick(), |r| {
        tick_manager.tick_or_rollback_tick(r.as_ref())
    });
    let is_rollback = rollback.map_or(false, |r| r.is_rollback());
    for (entity, action_state, info) in query.iter() {
        let look = action_state.axis_pair(&PlayerInput::Look);
        info!(
            ?is_rollback,
            ?tick,
            ?entity,
            ?look,
            ?info,
            "FixedUpdate"
        );
    }
}

/// Print the transform after physics have been applied (and position/rotation have been synced to Transform)
pub fn debug_after_sync(
    tick_manager: Res<TickManager>,
    rollback: Option<Res<Rollback>>,
    query: Query<
        (Entity, &ActionState<PlayerInput>, (&Transform, &Position, &Rotation, &LinearVelocity, &AngularVelocity)),
        (With<Player>, Or<(With<Predicted>, With<Replicating>)>)
    >
) {
    let tick = rollback.as_ref().map_or(tick_manager.tick(), |r| {
        tick_manager.tick_or_rollback_tick(r.as_ref())
    });
    let is_rollback = rollback.map_or(false, |r| r.is_rollback());
    for (entity, action_state, info) in query.iter() {
        let look = action_state.axis_pair(&PlayerInput::Look);
        info!(
            ?is_rollback,
            ?tick,
            ?entity,
            ?look,
            ?info,
            "After Physics"
        );
    }
}

/// Sets the player's velocity based on their inputs.
/// Actual transform manipulation is handled by the MoveablePlugin.
pub fn move_player(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &Player,
        &Rotation,
        &mut LinearVelocity,
        &mut AngularVelocity,
        &ShipIndex,
        &ActionState<PlayerInput>,
    ),
    Or<(With<Predicted>, With<Replicating>)>>,
    ships_data: Res<ShipsData>,
) {
    for (_player, rotation, mut linear_velocity, mut angular_velocity, ship_index, action_state) in query.iter_mut() {
        let mut wish_dir = Vec3::ZERO;

        if let Some(data) = ships_data.ships.get(&ship_index.0) {

            // @todo-brian: Also send the mouse sensitivity to the server, probably just do it thru PlayerInput
            let mouse_data = action_state.axis_pair(&PlayerInput::Look);
            if mouse_data != Vec2::ZERO {
                let yaw = -mouse_data.x * data.look_rotation_force;
                let pitch = -mouse_data.y * data.look_rotation_force;

                let right = rotation.0 * Vec3::X;
                let up = rotation.0 * Vec3::Y;

                angular_velocity.0 += up * yaw + right * pitch;
            }

            let mut roll_force = 0.0;
            if action_state.pressed(&PlayerInput::RollLeft) {
                roll_force -= data.roll_rotation_force;
            }
            if action_state.pressed(&PlayerInput::RollRight) {
                roll_force += data.roll_rotation_force;
            }
            
            if roll_force != 0.0 {
                let forward = rotation.0 * Vec3::NEG_Z;
                angular_velocity.0 += forward * roll_force;
            }

            // Accelerate in the direction of the input
            if action_state.pressed(&PlayerInput::MoveForward) {
                wish_dir += rotation.0 * Vec3::NEG_Z;
            }
            if action_state.pressed(&PlayerInput::MoveBackward) {
                wish_dir += rotation.0 * Vec3::Z;
            }
            if action_state.pressed(&PlayerInput::MoveLeft) {
                wish_dir += rotation.0 * Vec3::NEG_X;
            }
            if action_state.pressed(&PlayerInput::MoveRight) {
                wish_dir += rotation.0 * Vec3::X;
            }
            if action_state.pressed(&PlayerInput::MoveUp) {
                wish_dir += rotation.0 * Vec3::Y;
            }
            if action_state.pressed(&PlayerInput::MoveDown) {
                wish_dir += rotation.0 * Vec3::NEG_Y;
            }

            let wish_dir = wish_dir.normalize_or_zero();
            let after_burners = if action_state.pressed(&PlayerInput::AfterBurners) {
                Some(rotation)
            } else {
                None
            };

            move_ship(&fixed_time, &data, &mut linear_velocity, &mut angular_velocity, wish_dir, after_burners);
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub respawn_timer: Timer,
}
