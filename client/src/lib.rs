mod menu;
mod player;

use bevy::app::App;
use bevy::prelude::Plugin;

pub struct ClientPlugin;


impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(player::PlayerPlugin);
    }
}