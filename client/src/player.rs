use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::{InputMap, MouseMove};
use lightyear::shared::replication::components::Controlled;
use lightyear::prelude::{client::*, ClientId};
use shared::player::Player;
use shared::prelude::{GameLayer, Moveable, PlayerInput, MoveableShape, UniqueIdentity};
use shared::weapons::{WeaponInventory, WeaponsData};

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
        app.add_systems(Last, handle_predicted_spawn.run_if(is_synced).run_if(resource_exists::<WeaponsData>));
    }
}

/// Handle a newly spawned Predicted player:
fn handle_predicted_spawn(
    weapons_data: Res<WeaponsData>,
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
            .with(PlayerInput::ShootPrimary, MouseButton::Left)
            .with(PlayerInput::AfterBurners, MouseButton::Right)
            .with_dual_axis(PlayerInput::Look, MouseMove::default());

        // Adds an InputMap to Predicted so that the user can control the predicted entity
        commands.entity(entity).insert((input_map, 
            // We add a Moveable component so that we can predict velocity and angular velocity
            Moveable {
                velocity: Vec3::ZERO,
                angular_velocity: Vec3::ZERO,
                collision_shape: MoveableShape::Sphere(0.5),
                collision_mask: [GameLayer::Player, GameLayer::Wall].into(),
            },
        ));
    }
}
