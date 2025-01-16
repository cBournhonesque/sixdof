use bevy::asset::AssetPlugin;
use bevy::diagnostic::DiagnosticsPlugin;
use bevy::hierarchy::HierarchyPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use lightyear::prelude::server::*;
use crate::settings;

pub struct ServerApp(App);

impl ServerApp {
    pub fn new() -> Self {
        let mut app = App::new();
        #[cfg(feature = "gui")]
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
                .set(settings::window_plugin()),
        );
        #[cfg(not(feature = "gui"))]
        app.add_plugins((
            MinimalPlugins,
            log_plugin(),
            StatesPlugin,
            HierarchyPlugin,
            DiagnosticsPlugin,
        ));

        app.add_plugins(ServerPlugins { config: settings::server_config() });
        app.add_plugins(shared::SharedPlugin { headless: !cfg!(feature = "gui") });
        app.add_plugins(server::ServerPlugin);
        #[cfg(feature = "gui")]
        app.add_plugins(renderer::RendererPlugin);
        Self(app)
    }

    pub(crate) fn run(mut self) {
        self.0.run();
    }
}