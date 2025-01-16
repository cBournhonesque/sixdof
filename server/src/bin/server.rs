use bevy::diagnostic::DiagnosticsPlugin;
use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::state::app::StatesPlugin;
use lightyear::prelude::server::*;
use lightyear::prelude::ReplicationConfig;
use lightyear_examples_common::settings::get_server_net_configs;
use shared::prelude::{get_settings, shared_config, ASSETS_PATH, REPLICATION_INTERVAL};

fn main() {
    let settings = get_settings();

    // TODO: maybe have a separate launcher that is shared across client/server/etc.? instead of a client binary
    //  similar to what is done in lightyear_examples_common

    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        LogPlugin {
            level: Level::INFO,
            filter: "wgpu=error,bevy_render=info,bevy_ecs=warn,bevy_time=warn".to_string(),
            ..default()
        },
        AssetPlugin {
            // https://github.com/bevyengine/bevy/issues/10157
            meta_check: bevy::asset::AssetMetaCheck::Never,
            file_path: ASSETS_PATH.to_string(),
            ..default()
        },
        StatesPlugin,
        HierarchyPlugin,
        DiagnosticsPlugin,
    ));

    if settings.server.inspector && app.is_plugin_added::<RenderPlugin>() {
        app.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new());
    }
    let mut net_configs = get_server_net_configs(&settings);

    let config = ServerConfig {
        shared: shared_config(lightyear::shared::config::Mode::Separate),
        net: net_configs,
        replication: ReplicationConfig {
            send_interval: REPLICATION_INTERVAL,
            ..default()
        },
        ..default()
    };
    // add lightyear plugins before the protocol
    app.add_plugins(ServerPlugins { config });
    app.add_plugins(shared::SharedPlugin { headless: true});
    app.add_plugins(server::ServerPlugin);
    app.add_plugins(renderer::RendererPlugin);

    app.add_systems(Startup, server_start);

    // run the app
    app.run();
}

fn server_start(mut commands: Commands) {
    commands.start_server();
}