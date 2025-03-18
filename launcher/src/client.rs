use crate::settings;
use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use lightyear::client::plugin::ClientPlugins;

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
                    file_path: settings::get_assets_path(),
                    watch_for_changes_override: Some(settings::ASSETS_HOTRELOAD),
                    ..default()
                })
                .set(settings::log_plugin())
                .set(settings::window_plugin())
                // for bevy_trenchbroom
                // .set(ImagePlugin {
                //     #[cfg(not(feature = "server"))]
                //     default_sampler: bevy_trenchbroom::util::repeating_image_sampler(true),
                //     ..default()
                // })
        );
        app.add_plugins(ClientPlugins {
            config: settings::client_config(client_id),
        });
        app.add_plugins(shared::SharedPlugin { headless: false });
        app.add_plugins(client::ClientPlugin);
        app.add_plugins(renderer::RendererPlugin);
        Self(app)
    }

    pub(crate) fn run(mut self) {
        self.0.run();
    }
}
