use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_config_stack::prelude::ConfigAssetLoaderPlugin;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::{*, client::*};
use serde::{Deserialize, Serialize};
use crate::prelude::{PlayerInput, Moveable};

#[derive(Asset, Resource, Default,TypePath, Debug, Deserialize)]
pub struct PlayerShipData {
    pub accel_speed: f32,
    pub afterburner_accel_speed: f32,
    pub max_speed: f32,
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
            "BeforeInputs"
        );
    }
}

/// Print the transform after physics have been applied (and position/rotation have been synced to Transform)
pub fn debug_after_sync(
    tick_manager: Res<TickManager>,
    rollback: Option<Res<Rollback>>,
    query: Query<
        (Entity, (&Transform, &Position, &Rotation)),
        (With<Player>, Or<(With<Predicted>, With<Replicating>)>)
    >
) {
    let tick = rollback.as_ref().map_or(tick_manager.tick(), |r| {
        tick_manager.tick_or_rollback_tick(r.as_ref())
    });
    let is_rollback = rollback.map_or(false, |r| r.is_rollback());
    for (entity, info) in query.iter() {
        info!(
            ?is_rollback,
            ?tick,
            ?entity,
            ?info,
            "After Physics"
        );
    }
}

// TODO: this doesn't work if we modify Position/Rotation, but it works if we modify Transform. Why?
//  ANSWER: because visual interpolation is updated in FixedLast, but we only sync after RunFixedMainLoop,
//  so the VisualInterpolation Transform values are always Zero
//  So either:
//  - run SyncPlugin in FixedPostUpdate, and then we can update Position/Rotation (or Transform)
//  - run SyncPlugin in RunFixedMain, and then we update Transform from inputs. (but we need to make sure that
//    there are no velocities, etc.). But then Position/Rotation always stay at 0.0 so we never detect rollbacks!
// PreUpdate:
//  - receive confirmed Position/Rotation from server
//  - restore non-interpolated Transform
// FixedPreUpdate:
//  - update inputs
// FixedUpdate:
//  - option 1: (WORKS) use inputs to modify Transform
//  - option 2: (DOESN'T WORK) use inputs to modify Position/Rotation
// FixedPostUpdate:
//  - run Physics
// FixedLast:
//  - update VisualInterpolation values
// RunFixedMainLoop:
//  - sync Position/Rotation to Transform
// PostUpdate:
//  - Visually interpolate Transform
//  - Sync Transform to children, and to GlobalTransform
pub fn move_player(
    mut query: Query<(
        &Player,
        &mut Moveable,
        &mut Transform,
        &ActionState<PlayerInput>,
    ),
    Or<(With<Predicted>, With<Replicating>)>>,
    data: Res<PlayerShipData>,
) {
    for (_player, mut moveable, transform, action_state) in query.iter_mut() {
        let mut wish_dir = Vec3::ZERO;

        // @todo-brian: Also send the mouse sensitivity to the server, probably just do it thru PlayerInput
        let mouse_data = action_state.axis_pair(&PlayerInput::Look);
        if mouse_data != Vec2::ZERO {
            let yaw = -mouse_data.x * data.look_rotation_force;
            let pitch = -mouse_data.y * data.look_rotation_force;

            let right = transform.rotation * Vec3::X;
            let up = transform.rotation * Vec3::Y;

            moveable.angular_velocity += up * yaw + right * pitch;
        }

        let mut roll_force = 0.0;
        if action_state.pressed(&PlayerInput::RollLeft) {
            roll_force -= data.roll_rotation_force;
        }
        if action_state.pressed(&PlayerInput::RollRight) {
            roll_force += data.roll_rotation_force;
        }
        
        if roll_force != 0.0 {
            let forward = transform.rotation * Vec3::NEG_Z;
            moveable.angular_velocity += forward * roll_force;
        }

        moveable.angular_velocity *= 1.0 - data.rotation_damping;
        
        if moveable.angular_velocity.length_squared() > data.max_rotation_speed * data.max_rotation_speed {
            moveable.angular_velocity = moveable.angular_velocity.normalize() * data.max_rotation_speed;
        }
        
        // Accelerate in the direction of the input
        if action_state.pressed(&PlayerInput::MoveForward) {
            wish_dir += transform.rotation * Vec3::NEG_Z;
        }
        if action_state.pressed(&PlayerInput::MoveBackward) {
            wish_dir += transform.rotation * Vec3::Z;
        }
        if action_state.pressed(&PlayerInput::MoveLeft) {
            wish_dir += transform.rotation * Vec3::NEG_X;
        }
        if action_state.pressed(&PlayerInput::MoveRight) {
            wish_dir += transform.rotation * Vec3::X;
        }
        if action_state.pressed(&PlayerInput::MoveUp) {
            wish_dir += transform.rotation * Vec3::Y;
        }
        if action_state.pressed(&PlayerInput::MoveDown) {
            wish_dir += transform.rotation * Vec3::NEG_Y;
        }

        let wish_dir = wish_dir.normalize_or_zero();
        let accel = wish_dir * data.accel_speed;
        moveable.velocity += accel;

        // Afterburners push you forward
        if action_state.pressed(&PlayerInput::AfterBurners) {
            let accel = transform.rotation * Vec3::NEG_Z * data.afterburner_accel_speed;
            moveable.velocity += accel;
        } 
        
        // drag
        moveable.velocity *= 1.0 - data.drag;

        // max speed
        if moveable.velocity.length_squared() > data.max_speed * data.max_speed {
            moveable.velocity = moveable.velocity.normalize() * data.max_speed;
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub respawn_timer: Timer,
}
