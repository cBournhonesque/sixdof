use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_config_stack::prelude::ConfigAssetLoaderPlugin;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::{*, client::*};
use serde::{Deserialize, Serialize};
use crate::prelude::{MOVRotation, PlayerInput};

#[derive(Asset, Resource, Default,TypePath, Debug, Deserialize)]
pub struct PlayerShipData {
    pub accel_speed: f32,
    pub afterburner_accel_speed: f32,
    pub base_speed: f32,
    pub drag: f32,
    pub look_rotation_force: f32,
    pub max_rotation_speed: f32,
    pub roll_rotation_force: f32,
    pub rotation_damping: f32,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ConfigAssetLoaderPlugin::<PlayerShipData>::new("data/player_ship.ron"));
        app.add_systems(FixedUpdate, move_player.run_if(resource_exists::<PlayerShipData>));
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
        &MOVRotation,
        &mut LinearVelocity,
        &mut AngularVelocity,
        &ActionState<PlayerInput>,
    ),
    Or<(With<Predicted>, With<Replicating>)>>,
    data: Res<PlayerShipData>,
) {
    for (_player, rotation, mut linear_velocity, mut angular_velocity, action_state) in query.iter_mut() {
        let mut wish_dir = Vec3::ZERO;

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

        // TODO: use avian's AngularDamping?
        angular_velocity.0 *= 1.0 - data.rotation_damping;
        
        if angular_velocity.0.length_squared() > data.max_rotation_speed * data.max_rotation_speed {
            angular_velocity.0 = angular_velocity.0.normalize() * data.max_rotation_speed;
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
        
        // apply drag
        linear_velocity.0 = apply_drag(
            linear_velocity.0,
            linear_velocity.0.length(),
            data.drag, 
            fixed_time.delta_secs()
        );

        // apply acceleration
        let current_speed = linear_velocity.0.dot(wish_dir);
        linear_velocity.0 += accelerate(
            wish_dir, 
            data.base_speed,
            current_speed,
            data.accel_speed,
            fixed_time.delta_secs()
        );

        // apply afterburners accelerate you forward
        if action_state.pressed(&PlayerInput::AfterBurners) {
            let wish_dir = rotation.0 * Vec3::NEG_Z;
            let current_speed = linear_velocity.0.dot(rotation.0 * Vec3::NEG_Z);
            linear_velocity.0 += accelerate(
                wish_dir, 
                data.base_speed,
                current_speed,
                data.afterburner_accel_speed,
                fixed_time.delta_secs()
            );
        }

        // // max speed
        // if moveable.velocity.length_squared() > data.base_speed * data.base_speed {
        //     moveable.velocity = moveable.velocity.normalize() * data.base_speed;
        // }
    }
}

fn apply_drag(
    velocity: Vec3, 
    current_speed: f32, 
    drag: f32, 
    delta_seconds: f32
) -> Vec3 {
    let mut new_speed;
    let mut drop = 0.0;

    drop += current_speed * drag * delta_seconds;

    new_speed = current_speed - drop;
    if new_speed < 0.0 {
        new_speed = 0.0;
    }

    if new_speed != 0.0 {
        new_speed /= current_speed;
    }

    velocity * new_speed
}

fn accelerate(
    wish_direction: Vec3,
    wish_speed: f32,
    current_speed: f32,
    accel: f32,
    delta_seconds: f32,
) -> Vec3 {
    let add_speed = wish_speed - current_speed;

    if add_speed <= 0.0 {
        return Vec3::ZERO;
    }

    let mut accel_speed = accel * delta_seconds * wish_speed;
    if accel_speed > add_speed {
        accel_speed = add_speed;
    }

    wish_direction * accel_speed
}

#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub respawn_timer: Timer,
}
