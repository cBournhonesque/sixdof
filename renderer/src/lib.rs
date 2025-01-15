mod camera;

use bevy::prelude::*;

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init);
    }
}


fn init(mut commands: Commands) {
    commands.spawn(Camera2d);
}