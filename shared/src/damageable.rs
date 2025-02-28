use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// A component that defines the damageable properties of an entity.
/// Apply this to entities that should take damage.
// this is in shared and replicated
#[derive(Component, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Damageable {
    /// Kept as a u16 to keep the network payload small, if you need more health capacity than u16::MAX (lol), consider lowering damages!
    pub health: u16,
}
