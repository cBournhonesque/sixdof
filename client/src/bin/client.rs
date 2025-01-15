use bevy::asset::AssetPlugin;
use bevy::DefaultPlugins;
use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::ReplicationConfig;
use lightyear_examples_common::settings::get_client_net_config;
use shared::prelude::{get_settings, shared_config, REPLICATION_INTERVAL};
fn main() {
    let settings = get_settings();

    // TODO: maybe have a separate launcher that is shared across client/server/etc.? instead of a client binary
    //  similar to what is done in lightyear_examples_common

    let client_id = settings.client.client_id;
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

    app.add_systems(Startup, connect_client);

    // run the app
    app.run();
}

fn connect_client(mut commands: Commands) {
    commands.connect_client();
}