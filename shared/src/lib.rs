use bevy::app::App;
use bevy::prelude::*;

mod network;
mod map;
mod states;
mod physics;
pub mod player;
pub mod projectiles;
pub mod bot;

pub mod prelude {
    pub use crate::network::{protocol::*, settings::*};
    pub use crate::map::*;
    pub use crate::states::*;
    pub use crate::player::Player;
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
        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(projectiles::ProjectilesPlugin);
    }
}
