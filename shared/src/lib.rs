use bevy::app::App;
use bevy::asset::AssetPlugin;
use bevy::prelude::{default, Plugin};

mod network;
mod map;
mod states;
mod physics;
pub mod player;

pub mod prelude {
    pub use crate::network::settings::*;
    pub use crate::map::*;
    pub use crate::states::*;
    pub use crate::player::Player;
}

#[derive(Clone, Default)]
pub struct SharedPlugin {
    pub headless: bool
}

const DEFAULT_UNPROCESSED_FILE_PATH: &'static str = "../assets";

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        // PLUGINS
        app.add_plugins(network::protocol::ProtocolPlugin);
        app.add_plugins(map::MapPlugin { headless: self.headless});
        app.add_plugins(player::PlayerPlugin);
    }
}