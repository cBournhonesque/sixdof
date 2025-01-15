use std::net::SocketAddr;
use bevy::asset::AssetPlugin;
use bevy::DefaultPlugins;
use bevy::prelude::*;
use lightyear::client::config::ClientConfig;
use lightyear::client::plugin::ClientPlugins;
use lightyear::prelude::ReplicationConfig;
use lightyear_examples_common::app::{Cli, Mode};
use lightyear_examples_common::settings::get_client_net_config;
use lightyear_examples_common::shared::{shared_config, REPLICATION_INTERVAL};
use shared::prelude::get_settings;
fn main() {
    let settings = get_settings();

    // TODO: maybe have a separate launcher that is shared across all? instead of a client binary

    let client_id = None;
    // use the cli-provided client id if it exists, otherwise use the settings client id
    let client_id = client_id.unwrap_or(settings.client.client_id);
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .set(AssetPlugin {
                // https://github.com/bevyengine/bevy/issues/10157
                meta_check: bevy::asset::AssetMetaCheck::Never,
                file_path: "../assets".to_string(),
                ..default()
            })
    );
    if settings.client.inspector {
        app.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new());
    }

    let config = ClientConfig {
        shared: shared_config(lightyear::shared::config::Mode::Separate),
        net: get_client_net_config(&settings, client_id),
        replication: ReplicationConfig {
            send_interval: REPLICATION_INTERVAL,
            ..default()
        },
        ..default()
    };
    // add lightyear plugins before the protocol
    app.add_plugins(ClientPlugins { config });
    app.add_plugins(shared::SharedPlugin);
    app.add_plugins(client::ClientPlugin);
    app.add_plugins(renderer::RendererPlugin);
    // run the app
    app.run();
}