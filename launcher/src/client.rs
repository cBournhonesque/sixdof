use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use lightyear::client::plugin::ClientPlugins;
use crate::settings;

pub struct ClientApp(App);

impl ClientApp {
    pub fn new(client_id: u64) -> Self {
        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .build()
                .set(AssetPlugin {
                    // https://github.com/bevyengine/bevy/issues/10157
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    file_path: settings::ASSETS_PATH.to_string(),
                    ..default()
                })
                .set(settings::log_plugin())
                .set(settings::window_plugin())
        );
        app.add_plugins(ClientPlugins { config: settings::client_config(client_id) });
        app.add_plugins(shared::SharedPlugin { headless: false });
        app.add_plugins(client::ClientPlugin);
        app.add_plugins(renderer::RendererPlugin);
        Self(app)
    }

    pub(crate) fn run(mut self) {
        self.0.run();
    }
}