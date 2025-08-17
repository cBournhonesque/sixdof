mod player;
mod bot;
mod weapons;

use bevy::prelude::*;
use lightyear::prelude::server::*;

pub struct ServerPlugin;


impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {

        // PLUGINS
        app.add_plugins(bot::BotPlugin);
        app.add_plugins(lightyear_avian3d::prelude::LagCompensationPlugin);
        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(weapons::WeaponsPlugin);

        // SYSTEMS
        app.add_systems(Startup, server_start);
    }
}

fn server_start(
    server: Single<Entity, With<Server>>,
    mut commands: Commands) {
    commands.entity(server.into_inner()).trigger(Start);
}