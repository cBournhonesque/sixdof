mod player;
mod projectiles;
mod bot;
#[cfg(feature = "client")]
mod hud;

#[cfg(feature = "server")]
mod lag_compensation;
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
        app.add_plugins(projectiles::ProjectilesPlugin);
        app.insert_resource(AmbientLight {
            brightness: 0.0,
            ..default()
        });



        // RESOURCES
        let mut store = app.world_mut().resource_mut::<GizmoConfigStore>();
        let (config, _) = store.config_mut::<DefaultGizmoConfigGroup>();
        config.line_width = 20.0;
        config.depth_bias = -0.1;

        #[cfg(feature = "client")]
        {
            app.add_plugins(VisualInterpolationPlugin::<Transform>::default());
            app.add_plugins(hud::HudPlugin);
        }

        // SYSTEMS
        // TODO: separate client renderer from server renderer? The features cfg are not enough
        // on the server, the camera doesn't follow a player
        #[cfg(not(feature = "client"))]
        app.add_systems(Startup, init);
    }
}


#[cfg(not(feature = "client"))]
fn init(mut commands: Commands) {
    commands.spawn(Camera3d::default());
}
