use bevy::asset::AssetPlugin;
use bevy::DefaultPlugins;
use bevy::prelude::*;
use lightyear::prelude::{client::NetConfig, client::ClientPlugins, server::ServerPlugins, Mode};
use crate::settings;

pub struct HostServer(App);
impl HostServer {
    pub fn new(client_id: u64) -> Self {
        let mut client_config = settings::client_config(client_id);
        client_config.net = NetConfig::Local {
            id: client_id
        };
        client_config.shared.mode = Mode::HostServer;
        let mut server_config = settings::server_config();
        server_config.shared.mode = Mode::HostServer;

        // gui app
        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .build()
                .set(AssetPlugin {
                    // https://github.com/bevyengine/bevy/issues/10157
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    file_path: settings::get_assets_path(),
                    watch_for_changes_override: Some(settings::ASSETS_HOTRELOAD),
                    ..default()
                })
                .set(settings::log_plugin())
                .set(settings::window_plugin()),
        );
        app.add_plugins(ClientPlugins { config: client_config });
        app.add_plugins(ServerPlugins { config: server_config });
        app.add_plugins(shared::SharedPlugin { headless: false });
        app.add_plugins(client::ClientPlugin);
        app.add_plugins(server::ServerPlugin);
        app.add_plugins(renderer::RendererPlugin);
        Self(app)
    }

    pub(crate) fn run(mut self) {
        self.0.run();
    }
}