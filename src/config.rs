use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub enum InputAction {
    Keyboard(KeyCode),
    Mouse(MouseButton),
}

impl InputAction {
    pub fn pressed(
        &self,
        mouse_input: &Res<ButtonInput<MouseButton>>,
        keyboard_input: &Res<ButtonInput<KeyCode>>,
    ) -> bool {
        match self {
            InputAction::Keyboard(key) => keyboard_input.pressed(*key),
            InputAction::Mouse(button) => mouse_input.pressed(*button),
        }
    }
}

#[derive(Resource, Clone, Serialize, Deserialize)]
pub struct Config {
    pub roll_speed: f32,
    pub mouse_sensitivity: f32,
    pub bindings: KeyBindings,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    pub move_forward: InputAction,
    pub move_backward: InputAction,
    pub move_left: InputAction,
    pub move_right: InputAction,
    pub move_up: InputAction,
    pub move_down: InputAction,
    pub move_roll_left: InputAction,
    pub move_roll_right: InputAction,
    pub shoot_primary: InputAction,
    pub shoot_secondary: InputAction,
    pub weapon_1: InputAction,
    pub weapon_2: InputAction,
    pub weapon_3: InputAction,
    pub weapon_4: InputAction,
    pub weapon_5: InputAction,
    pub weapon_6: InputAction,
    pub weapon_7: InputAction,
    pub weapon_8: InputAction,
    pub weapon_9: InputAction,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            roll_speed: 2.5,
            mouse_sensitivity: 0.0012,
            bindings: KeyBindings {
                move_forward: InputAction::Keyboard(KeyCode::KeyW),
                move_backward: InputAction::Keyboard(KeyCode::KeyS),
                move_left: InputAction::Keyboard(KeyCode::KeyA),
                move_right: InputAction::Keyboard(KeyCode::KeyD),
                move_up: InputAction::Keyboard(KeyCode::ShiftLeft),
                move_down: InputAction::Keyboard(KeyCode::ControlLeft),
                move_roll_left: InputAction::Keyboard(KeyCode::KeyQ),
                move_roll_right: InputAction::Keyboard(KeyCode::KeyE),
                shoot_primary: InputAction::Mouse(MouseButton::Left),
                shoot_secondary: InputAction::Mouse(MouseButton::Right),
                weapon_1: InputAction::Keyboard(KeyCode::Digit1),
                weapon_2: InputAction::Keyboard(KeyCode::Digit2),
                weapon_3: InputAction::Keyboard(KeyCode::Digit3),
                weapon_4: InputAction::Keyboard(KeyCode::Digit4),
                weapon_5: InputAction::Keyboard(KeyCode::Digit5),
                weapon_6: InputAction::Keyboard(KeyCode::Digit6),
                weapon_7: InputAction::Keyboard(KeyCode::Digit7),
                weapon_8: InputAction::Keyboard(KeyCode::Digit8),
                weapon_9: InputAction::Keyboard(KeyCode::Digit9),
            },
        }
    }
}
