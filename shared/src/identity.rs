use bevy::ecs::component::Component;
use lightyear::prelude::PeerId;
use serde::{Deserialize, Serialize};

/// An identity is anything that can be uniquely identified. 
/// Its mostly used for things like determining who owns a projectile, etc.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UniqueIdentity {
    Player(PeerId),
    Bot(u32),
}
