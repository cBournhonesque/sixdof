mod menu;
mod player;
mod bot;
mod weapon;

use bevy::prelude::*;
use lightyear::prelude::*;

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

fn connect_client(
    client: Single<Entity, With<Client>>,
    mut commands: Commands) {
    commands.entity(client.into_inner()).trigger(Connect);
}