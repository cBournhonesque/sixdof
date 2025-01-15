use crate::player::PlayerInput;
use bevy::prelude::*;
use std::time::SystemTime;

#[derive(Event, Clone, Debug)]
pub struct NetPlayerInputEvent {
    pub input: PlayerInput,
    pub player_id: u8,
}

#[derive(Clone)]
pub struct SavedInput {
    pub input: PlayerInput,
    pub final_translation: Vec3,
    pub final_velocity: Vec3,
    pub delta_seconds: f32,
    pub sent: bool,
}

#[derive(Resource)]
pub struct SavedInputs {
    pub real_rotation: Quat,
    inputs: Vec<SavedInput>,
    pub delta_time_sum: f64,
}

impl Default for SavedInputs {
    fn default() -> Self {
        Self {
            real_rotation: Quat::IDENTITY,
            inputs: Vec::new(),
            delta_time_sum: 0.0,
        }
    }
}

impl SavedInputs {
    pub fn add_new_input(&mut self, input: &PlayerInput, delta_seconds: f32) {
        self.inputs.push(SavedInput {
            input: input.clone(),
            final_translation: Vec3::ZERO,
            final_velocity: Vec3::ZERO,
            delta_seconds: delta_seconds,
            sent: false,
        });
    }

    pub fn clean_old_inputs(&mut self) {
        let time_now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.inputs.retain(|input| {
            let input_time = input.input.id;
            let input_age = time_now - input_time;
            input_age < 1000
        })
    }

    pub fn latest_input(&self) -> Option<&SavedInput> {
        self.inputs.last()
    }

    pub fn latest_input_mut(&mut self) -> Option<&mut SavedInput> {
        self.inputs.last_mut()
    }

    pub fn inputs(&self) -> &Vec<SavedInput> {
        &self.inputs
    }

    pub fn inputs_mut(&mut self) -> &mut Vec<SavedInput> {
        &mut self.inputs
    }
}
