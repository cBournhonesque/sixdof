use bevy::utils::HashMap;
use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy_renet::renet::ClientId;
use std::time::SystemTime;

use crate::net::input::*;
use crate::net::LocallyOwned;
use crate::net::AUTHORITY_ID;
use crate::physics::*;
use crate::snapshot::history::{SnapshotHistory, SnapshotInterpolation};
use crate::weapons::*;
use crate::{components::*, PlayingSubState};
use crate::{config::*, is_paused};
use crate::{hud::*, AppState};
use bevy_rapier3d::prelude::*;
use serde::{Deserialize, Serialize};

const VISUALS_LERP_SPEED: f32 = 24.0;
const VISUALS_MAX_LERP_DISTANCE: f32 = 5.0;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerInput {
    pub id: u64,
    pub snapshot_id: Option<u64>,
    pub move_direction: Vec3,
    pub look_rotation: Quat,
    pub holding_down_fire: bool,
    pub weapon_key: Option<u8>,
    pub num_left_in_burst: u8,
    pub wish_weapon_key: Option<u8>,
}

impl PlayerInput {
    pub fn merge_important_props(&mut self, other: &PlayerInput) {
        if other.holding_down_fire {
            self.holding_down_fire = true;
        }
        if other.weapon_key.is_some() {
            self.weapon_key = other.weapon_key.clone();
        }
        if other.wish_weapon_key.is_some() {
            self.wish_weapon_key = other.wish_weapon_key.clone();
        }
    }
}

#[derive(Resource)]
pub struct LocalPlayer {
    pub client_id: ClientId,
    pub player_id: u8,
}

impl Default for LocalPlayer {
    fn default() -> Self {
        Self {
            client_id: ClientId::from_raw(0),
            player_id: 0,
        }
    }
}

impl LocalPlayer {
    pub fn has_authority(&self) -> bool {
        self.client_id.raw() == AUTHORITY_ID
    }
    pub fn equals(&self, player: &Player) -> bool {
        player.id == self.player_id
    }
}

#[derive(Component, Clone)]
pub struct Player {
    pub visuals: Option<Entity>,
    pub latest_processed_input: Option<PlayerInput>,
    pub id: u8,
    pub name: String,
    pub frags: u16,
    pub deaths: u16,
    pub score: u16,
    pub ping: u8,
    pub respawn_timer: Timer,
    pub frozen_amount: u8,
}

#[derive(Component)]
pub struct LocalPlayerVisuals;

#[derive(Component)]
pub struct PlayerVisuals;

pub fn player_input_system(
    mut saved_inputs: ResMut<SavedInputs>,
    time: Res<Time>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    config: Res<Config>,
    app_state: Res<State<AppState>>,
    weapon_container: Query<&WeaponContainer, With<LocallyOwned>>,
    snapshot_interpolation: Option<Res<SnapshotInterpolation>>,
    mut mouse_motion: EventReader<MouseMotion>,
    chat: Res<Chat>,
) {
    let mut last_input = saved_inputs.latest_input();
    let mut latest_input = PlayerInput {
        id: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        snapshot_id: {
            if let Some(snapshot_interpolation) = snapshot_interpolation {
                snapshot_interpolation.latest_reconciled_snapshot_id
            } else {
                None
            }
        },
        holding_down_fire: false,
        num_left_in_burst: 0, // filled out later
        weapon_key: None,     // filled out later
        wish_weapon_key: last_input.and_then(|i| i.input.wish_weapon_key.clone()),
        move_direction: Vec3::ZERO,
        look_rotation: if let Some(latest_input) = saved_inputs.latest_input() {
            latest_input.input.look_rotation
        } else {
            Quat::IDENTITY
        },
    };

    if !chat.show && !is_paused(app_state) {
        // Accumulate mouse rotation
        if mouse_motion.len() > 0 {
            for mouse_motion in mouse_motion.read() {
                let delta = -mouse_motion.delta * config.mouse_sensitivity;

                // Apply horizontal rotation
                let yaw = Quat::from_rotation_y(delta.x);
                saved_inputs.real_rotation *= -yaw;

                // Apply vertical rotation
                let pitch = Quat::from_rotation_x(delta.y);
                saved_inputs.real_rotation *= pitch;
            }
        }

        if config
            .bindings
            .move_roll_left
            .pressed(&mouse_input, &keyboard_input)
        {
            // Apply roll rotation
            let roll = Quat::from_rotation_z(config.roll_speed * time.delta_seconds());
            saved_inputs.real_rotation *= roll;
        }

        if config
            .bindings
            .move_roll_right
            .pressed(&mouse_input, &keyboard_input)
        {
            // Apply roll rotation in the opposite direction
            let roll = Quat::from_rotation_z(-config.roll_speed * time.delta_seconds());
            saved_inputs.real_rotation *= roll;
        }

        // lerp rotation
        latest_input.look_rotation = latest_input
            .look_rotation
            .slerp(saved_inputs.real_rotation, 10.0 * time.delta_seconds());

        // movement
        let rotation = latest_input.look_rotation;
        latest_input.move_direction = Vec3::ZERO;

        if config
            .bindings
            .move_forward
            .pressed(&mouse_input, &keyboard_input)
        {
            latest_input.move_direction -= rotation * Vec3::Z;
        }
        if config
            .bindings
            .move_backward
            .pressed(&mouse_input, &keyboard_input)
        {
            latest_input.move_direction += rotation * Vec3::Z;
        }
        if config
            .bindings
            .move_left
            .pressed(&mouse_input, &keyboard_input)
        {
            latest_input.move_direction -= rotation * Vec3::X;
        }
        if config
            .bindings
            .move_right
            .pressed(&mouse_input, &keyboard_input)
        {
            latest_input.move_direction += rotation * Vec3::X;
        }
        if config
            .bindings
            .move_up
            .pressed(&mouse_input, &keyboard_input)
        {
            latest_input.move_direction += rotation * Vec3::Y;
        }
        if config
            .bindings
            .move_down
            .pressed(&mouse_input, &keyboard_input)
        {
            latest_input.move_direction -= rotation * Vec3::Y;
        }

        if latest_input.move_direction.length_squared() > 0.0 {
            latest_input.move_direction = latest_input.move_direction.normalize();
        }

        // weapons
        latest_input.holding_down_fire = config
            .bindings
            .shoot_primary
            .pressed(&mouse_input, &keyboard_input);

        // weapon switching
        if config
            .bindings
            .weapon_1
            .pressed(&mouse_input, &keyboard_input)
        {
            latest_input.wish_weapon_key = Some(1);
        }
        if config
            .bindings
            .weapon_2
            .pressed(&mouse_input, &keyboard_input)
        {
            latest_input.wish_weapon_key = Some(2);
        }
        if config
            .bindings
            .weapon_3
            .pressed(&mouse_input, &keyboard_input)
        {
            latest_input.wish_weapon_key = Some(3);
        }
        if config
            .bindings
            .weapon_4
            .pressed(&mouse_input, &keyboard_input)
        {
            latest_input.wish_weapon_key = Some(4);
        }
        if config
            .bindings
            .weapon_5
            .pressed(&mouse_input, &keyboard_input)
        {
            latest_input.wish_weapon_key = Some(5);
        }
    }

    saved_inputs.add_new_input(&latest_input, time.delta_seconds());
}

pub fn player_input_reader_system(
    time: Res<Time>,
    physics_context: Res<RapierContext>,
    saved_inputs: Res<SavedInputs>,
    mut query: Query<
        (
            Entity,
            &Health,
            &mut WeaponContainer,
            &mut Transform,
            &mut MovementState,
            &Collider,
        ),
        (With<LocallyOwned>, With<Player>),
    >,
) {
    for (entity, health, mut weapon_container, mut transform, mut movement_state, collider) in
        query.iter_mut()
    {
        if health.dead() {
            continue;
        }

        if let Some(latest_input) = saved_inputs.latest_input() {
            crate::physics::move_entity(
                &entity,
                latest_input.input.move_direction,
                latest_input.input.look_rotation,
                &mut transform,
                &mut movement_state,
                &collider,
                &physics_context,
                time.delta_seconds(),
            );
        }
    }
}

pub fn local_player_visuals_system(
    time: Res<Time>,
    mut visuals: Query<&mut Transform, With<LocalPlayerVisuals>>,
) {
    if let Ok(mut transform) = visuals.get_single_mut() {
        if transform.translation.length() < VISUALS_MAX_LERP_DISTANCE {
            transform.translation = transform.translation.lerp(
                Vec3::ZERO,
                (VISUALS_LERP_SPEED * time.delta_seconds()).min(1.0),
            );

            // if it's close enough just snap it
            if transform.translation.length_squared() < 0.001 {
                transform.translation = Vec3::ZERO;
            }
        } else {
            transform.translation = Vec3::ZERO;
        }
    }
}

pub fn player_camera_system(
    mut camera: Query<&mut Transform, With<Camera>>,
    player: Query<&Transform, (With<Player>, Without<Camera>)>,
    visuals: Query<
        (&GlobalTransform, &Parent),
        (With<LocalPlayerVisuals>, Without<Player>, Without<Camera>),
    >,
) {
    if let Ok(mut camera_transform) = camera.get_single_mut() {
        if let Ok((visuals_transform, parent)) = visuals.get_single() {
            let entity = parent.get();
            if let Ok(player_transform) = player.get(entity) {
                camera_transform.translation = visuals_transform.translation();
                camera_transform.rotation = player_transform.rotation;
            }
        }
    }
}
