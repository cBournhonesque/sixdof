mod player;

use bevy::app::App;
use bevy::prelude::Plugin;

pub struct ServerPlugin;


impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(player::PlayerPlugin);
    }
}