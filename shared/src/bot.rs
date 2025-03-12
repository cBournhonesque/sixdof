use bevy::prelude::{Component};
use lightyear::prelude::{Deserialize, Serialize};

#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bot;


#[derive(Debug, Deserialize)]
pub struct BotBehavior {
    pub attack_kind: BotAttackKind,
}

impl Default for BotBehavior {
    fn default() -> Self {
        Self {
            attack_kind: BotAttackKind::Standard { 
                target_distance: 10.0
            },
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum BotAttackKind {
    /// Moves to the target distance and tries to stay there while attacking the target
    Standard {
        target_distance: f32,
    },
    /// Will attempt to orbit around the target while attacking
    Aggressive {
        target_distance: f32,
        back_off_distance: f32,
        change_attack_direction_interval: f32,
    }        
}
