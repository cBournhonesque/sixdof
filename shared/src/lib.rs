use bevy::app::App;
use bevy::prelude::*;

mod network;
mod map;
mod states;
pub mod physics;
pub mod player;
pub mod weapons;
pub mod identity;
pub mod bot;
pub mod utils;
pub mod damageable;
pub mod data;
pub mod ships;

pub mod prelude {
    pub use crate::network::{protocol::*, settings::*};
    pub use crate::physics::*;
    pub use crate::weapons::*;
    pub use crate::map::*;
    pub use crate::states::*;
    pub use crate::player::Player;
    pub use crate::identity::*;
    pub use crate::utils::DespawnAfter;
    pub use crate::damageable::*;
    pub use crate::data::audio::*;
    pub use crate::data::weapons::*;
    pub use crate::ships::*;
}

#[derive(Clone, Default)]
pub struct SharedPlugin {
    pub headless: bool
}

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        // PLUGINS
        app.add_plugins(network::protocol::ProtocolPlugin);
        app.add_plugins(map::MapPlugin { headless: self.headless});
        app.add_plugins(physics::PhysicsPlugin);
        app.add_plugins(ships::ShipPlugin);
        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(weapons::WeaponsPlugin);
        app.add_plugins(utils::UtilsPlugin);
    }
}
