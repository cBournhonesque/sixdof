use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::{InputMap, MouseMove};
use lightyear::shared::replication::components::Controlled;
use lightyear::prelude::client::*;
use shared::player::Player;
use shared::prelude::PlayerInput;

pub(crate) struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {

        // app.add_systems(
        //     FixedPreUpdate,
        //     // make sure this runs after the other leafwing systems
        //     // mouse_to_world_space.in_set(InputManagerSystem::ManualControl),
        //
        //     // TODO: think about system ordering in the case of input delay!
        //     // make sure we update the ActionState before buffering them
        //     capture_input
        //         .before(InputSystemSet::BufferClientInputs)
        //         .run_if(not(is_in_rollback)),
        // );

        // make sure that client cannot apply inputs before the connection is synced
        // we add the system in Last so that on the first time the InputMap is spawned, we don't immediately
        // send an InputMessage to the server
        app.add_systems(Last, handle_predicted_spawn.run_if(is_synced));
    }
}


/// Handle a newly spawned Predicted player:
/// - adds an InputMap to Predicted so that the user can control the predicted entity
/// - adds RigidBody::Kinematic so that physics are also applied to that entity on the client side
fn handle_predicted_spawn(
    mut commands: Commands,
    predicted_player: Query<Entity, (With<Controlled>, With<Player>, With<Predicted>, Without<InputMap<PlayerInput>>)>
) {
    for entity in predicted_player.iter() {
        let input_map = InputMap::<PlayerInput>::default()
            .with_multiple([
                (PlayerInput::MoveForward, KeyCode::KeyW),
                (PlayerInput::MoveBackward, KeyCode::KeyS),
                (PlayerInput::MoveLeft, KeyCode::KeyA),
                (PlayerInput::MoveRight, KeyCode::KeyD),
                (PlayerInput::MoveUp, KeyCode::Space),
                (PlayerInput::MoveDown, KeyCode::ShiftLeft),
                (PlayerInput::MoveRollLeft, KeyCode::KeyQ),
                (PlayerInput::MoveRollRight, KeyCode::KeyE),
                (PlayerInput::Weapon1, KeyCode::Digit1),
                (PlayerInput::Weapon2, KeyCode::Digit2),
                (PlayerInput::Weapon3, KeyCode::Digit3),
                (PlayerInput::Weapon4, KeyCode::Digit4),
                (PlayerInput::Weapon5, KeyCode::Digit5),
            ])
            .with(PlayerInput::ShootPrimary, MouseButton::Left)
            .with(PlayerInput::ShootSecondary, MouseButton::Right)
            .with_dual_axis(PlayerInput::Look, MouseMove::default());

        commands.entity(entity).insert((input_map, RigidBody::Kinematic));
    }
}

// /// Capture the mouse data and use it to update the ActionState
// fn capture_input(
//     mut action_state_query: Query<
//         (&Position, &mut ActionState<PlayerInput>),
//         (With<Predicted>, With<Controlled>, With<Player>)
//     >,
//     // query to get the window (so we can read the current cursor position)
//     q_window: Query<&Window, With<PrimaryWindow>>,
//     // query to get camera transform
//     q_camera: Query<(&Camera, &GlobalTransform)>,
// ) {
//     let Ok((camera, camera_transform)) = q_camera.get_single() else {
//         error!("Expected to find only one camera");
//         return;
//     };
//     let window = q_window.single();
//
//     if let Some(world_position) = window
//         .cursor_position()
//         .map(|cursor| camera.viewport_to_world(camera_transform, cursor).unwrap())
//         .map(|ray| ray.origin)
//     {
//         if let Ok((position, mut action_state)) = action_state_query.get_single_mut() {
//             let mouse_position_relative = world_position - position.0;
//             action_state.press(&PlayerInput::Look);
//             action_state
//                 .action_data_mut(&PlayerInput::Look)
//                 .unwrap()
//                 .axis_pair = Some(DualAxisData::from_xy(
//                 mouse_position_relative * CAMERA_SCALE,
//             ));
//             trace!(tick = ?tick_manager.tick(), ?mouse_position_relative, "Relative mouse position");
//         }
//     }
// }