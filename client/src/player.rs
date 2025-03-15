use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::shared::replication::components::Controlled;
use lightyear::prelude::{client::*};
use shared::player::Player;
use shared::prelude::{PlayerInput};
use shared::ships::get_shared_ship_components;

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
                (PlayerInput::RollLeft, KeyCode::KeyQ),
                (PlayerInput::RollRight, KeyCode::KeyE),
                (PlayerInput::Weapon1, KeyCode::Digit1),
                (PlayerInput::Weapon2, KeyCode::Digit2),
                (PlayerInput::Weapon3, KeyCode::Digit3),
                (PlayerInput::Weapon4, KeyCode::Digit4),
                (PlayerInput::Weapon5, KeyCode::Digit5),
                (PlayerInput::ToggleMousePointer, KeyCode::Tab),
            ])
            .with(PlayerInput::NextWeapon, MouseScrollDirection::UP)
            .with(PlayerInput::PreviousWeapon, MouseScrollDirection::DOWN)
            .with(PlayerInput::ShootPrimary, MouseButton::Left)
            .with(PlayerInput::AfterBurners, MouseButton::Right)
            .with_dual_axis(PlayerInput::Look, MouseMove::default());

        // Adds an InputMap to Predicted so that the user can control the predicted entity
        commands.entity(entity).insert((input_map,
            get_shared_ship_components(Collider::sphere(0.5))
        ));
    }
}
