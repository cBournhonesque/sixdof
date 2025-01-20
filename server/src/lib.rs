mod player;
mod bot;
mod projectiles;
mod lag_compensation;

use bevy::app::{App, Startup};
use bevy::prelude::{Commands, Plugin};
use lightyear::prelude::server::ServerCommands;

pub struct ServerPlugin;


impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {

        // PLUGINS
        app.add_plugins(bot::BotPlugin);
        app.add_plugins(lag_compensation::LagCompensationPlugin);
        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(projectiles::ProjectilesPlugin);


        // SYSTEMS
        app.add_systems(Startup, server_start);
    }
}

fn server_start(mut commands: Commands) {
    commands.start_server();
}