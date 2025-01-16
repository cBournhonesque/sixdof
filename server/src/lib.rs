mod player;

use bevy::app::{App, Startup};
use bevy::prelude::{Commands, Plugin};
use lightyear::prelude::server::ServerCommands;

pub struct ServerPlugin;


impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        // PLUGINS
        app.add_plugins(player::PlayerPlugin);

        // SYSTEMS
        app.add_systems(Startup, server_start);
    }
}

fn server_start(mut commands: Commands) {
    commands.start_server();
}