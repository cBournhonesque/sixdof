mod player;
mod weapon;

use bevy::prelude::*;

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        // PLUGINS
        // TODO: add option to disable inspector
        app.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new());
        app.add_plugins(player::PlayerPlugin);

        // SYSTEMS
        // TODO: separate client renderer from server renderer?
        // on the server, the camera doesn't follow a player
        #[cfg(not(feature = "client"))]
        app.add_systems(Startup, init);
    }
}


#[cfg(not(feature = "client"))]
fn init(mut commands: Commands) {
    dbg!("ADD CAM");
    commands.spawn(Camera3d::default());
}