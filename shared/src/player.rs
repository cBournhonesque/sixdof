use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::{*, client::*};
use serde::{Deserialize, Serialize};
use crate::prelude::PlayerInput;

const MOVE_SPEED : f32 = 0.125;
const LOOK_ROTATION_SPEED : f32 = 0.003;
const ROLL_SPEED : f32 = 0.02;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {

        // SYSTEMS

        // DEBUG
        app.add_systems(FixedUpdate, debug_input.before(move_player));
        app.add_systems(FixedLast, debug_after_sync);
        // app.add_systems(RunFixedMainLoop, debug_after_sync.after(RunFixedMainLoopSystem::AfterFixedMainLoop));

        app.add_systems(FixedUpdate, move_player);
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
//    So it doesn't work!
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
    // tick_manager: Res<TickManager>,
    // rollback: Option<Res<Rollback>>,
    mut query: Query<(
        &Player,
        &mut Transform,
        &ActionState<PlayerInput>,
    ),
    Or<(With<Predicted>, With<Replicating>)>>
) {
    // let tick = rollback.as_ref().map_or(tick_manager.tick(), |r| {
    //     tick_manager.tick_or_rollback_tick(r.as_ref())
    // });
    // let is_rollback = rollback.map_or(false, |r| r.is_rollback());
    for (_player, mut transform, action_state) in query.iter_mut() {
        let mut wish_dir = Vec3::ZERO;

        let mouse_data = action_state.axis_pair(&PlayerInput::Look);
        if mouse_data != Vec2::ZERO {
            let yaw = -mouse_data.x * LOOK_ROTATION_SPEED;
            let pitch = -mouse_data.y * LOOK_ROTATION_SPEED;
            
            let right = transform.rotation * Vec3::X;
            let up = transform.rotation * Vec3::Y;
            
            let pitch_rot = Quat::from_axis_angle(right, pitch);
            let yaw_rot = Quat::from_axis_angle(up, yaw);
            
            transform.rotation = pitch_rot * yaw_rot * transform.rotation;
        }
        
        if action_state.pressed(&PlayerInput::MoveRollLeft) {
            let forward = transform.rotation * Vec3::NEG_Z;
            let roll_rot = Quat::from_axis_angle(forward, -ROLL_SPEED);
            transform.rotation = roll_rot * transform.rotation;
        }
        if action_state.pressed(&PlayerInput::MoveRollRight) {
            let forward = transform.rotation * Vec3::NEG_Z;
            let roll_rot = Quat::from_axis_angle(forward, ROLL_SPEED);
            transform.rotation = roll_rot * transform.rotation;
        }
        
        if action_state.pressed(&PlayerInput::MoveForward) {
            wish_dir += Vec3::NEG_Z;
        }
        if action_state.pressed(&PlayerInput::MoveBackward) {
            wish_dir += Vec3::Z;
        }
        if action_state.pressed(&PlayerInput::MoveLeft) {
            wish_dir += Vec3::NEG_X;
        }
        if action_state.pressed(&PlayerInput::MoveRight) {
            wish_dir += Vec3::X;
        }
        if action_state.pressed(&PlayerInput::MoveUp) {
            wish_dir += Vec3::Y;
        }
        if action_state.pressed(&PlayerInput::MoveDown) {
            wish_dir += Vec3::NEG_Y;
        }

        if wish_dir != Vec3::ZERO {
            let wish_dir = wish_dir.normalize();
            let world_wish_dir = transform.rotation * wish_dir;
            let movement = world_wish_dir * MOVE_SPEED;
            transform.translation += movement;
        }

        // TODO: do not run this if transform.rotation did not change to not trigger change detection
        transform.rotation = transform.rotation.normalize();
        // info!(
        //     ?is_rollback,
        //     ?tick,
        //     ?transform,
        //     "AfterInputsApplied"
        // );
    }
}

#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub id: ClientId,
    pub name: String,
    pub frags: u16,
    pub deaths: u16,
    pub score: u16,
    pub ping: u8,
    pub respawn_timer: Timer,
    pub frozen_amount: u8,
}