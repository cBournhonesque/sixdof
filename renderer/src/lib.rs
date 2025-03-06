mod player;
mod weapons;
mod bot;
mod audio;

#[cfg(feature = "client")]
mod hud;

mod physics;

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;
use lightyear::client::interpolation::VisualInterpolationPlugin;
use lightyear::prelude::client::{Interpolated, Predicted};
use lightyear::prelude::server::ReplicationTarget;

pub struct RendererPlugin;


/// Convenient for filter for entities that should be visible
/// Works either on the client or the server
#[derive(QueryFilter)]
pub struct VisibleFilter {
    a: Or<(
        With<Predicted>,
        With<Interpolated>,
        With<ReplicationTarget>,
    )>,
}

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        // PLUGINS
        // TODO: add option to disable inspector
        app.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new());
        app.add_plugins(bot::BotPlugin);
        app.add_plugins(physics::PhysicsPlugin);
        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(::audio::SfxAudioPlugin::default());
        app.add_plugins(weapons::WeaponsPlugin);
        app.insert_resource(AmbientLight {
            brightness: 0.0,
            ..default()
        });
        // RESOURCES

        #[cfg(feature = "client")]
        {
            app.add_plugins(VisualInterpolationPlugin::<Transform>::default());
            app.add_plugins(hud::HudPlugin);
        }

        // SYSTEMS
        // TODO: separate client renderer from server renderer? The features cfg are not enough
        //  how do we deal with host-server / listen-server modes where both client and server are enabled?
        // on the server, the camera doesn't follow a player
        #[cfg(not(feature = "client"))]
        app.add_systems(Startup, init);
    }
}


// TODO: spawn a camera that is controllable on the server side to debug issues
#[cfg(not(feature = "client"))]
fn init(mut commands: Commands) {
    commands.spawn(Camera3d::default());
}
