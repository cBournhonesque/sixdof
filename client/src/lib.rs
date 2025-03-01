mod menu;
mod player;
mod bot;
mod weapon;

use bevy::app::{App, Startup};
use bevy::prelude::{Commands, Plugin};
use lightyear::prelude::client::ClientCommandsExt;

pub struct ClientPlugin;


impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        // PLUGINS
        app.add_plugins(bot::BotPlugin);
        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(weapon::WeaponPlugin);

        // SYSTEMS
        app.add_systems(Startup, connect_client);
    }
}

fn connect_client(mut commands: Commands) {
    commands.connect_client();
}