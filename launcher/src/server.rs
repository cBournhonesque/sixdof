use crate::settings;
use bevy::asset::AssetPlugin;
use bevy::diagnostic::DiagnosticsPlugin;
use bevy::hierarchy::HierarchyPlugin;
use bevy::prelude::*;
use bevy::render::mesh::MeshPlugin;
use bevy::scene::ScenePlugin;
use bevy::state::app::StatesPlugin;
use lightyear::prelude::server::*;

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
                    file_path: settings::get_assets_path(),
                    watch_for_changes_override: Some(settings::ASSETS_HOTRELOAD),
                    ..default()
                })
                .set(settings::log_plugin())
                .set(settings::window_plugin()),
        );
        #[cfg(not(feature = "gui"))]
        app.add_plugins((
            MinimalPlugins,
            // needed to load the map asset
            AssetPlugin {
                // https://github.com/bevyengine/bevy/issues/10157
                meta_check: bevy::asset::AssetMetaCheck::Never,
                file_path: settings::get_assets_path(),
                ..default()
            },
            // the mesh asset is needed for avian collisions
            MeshPlugin,
            ScenePlugin,
            settings::log_plugin(),
            StatesPlugin,
            HierarchyPlugin,
            DiagnosticsPlugin,
        ));

        app.add_plugins(ServerPlugins {
            config: settings::server_config(),
        });
        app.add_plugins(shared::SharedPlugin {
            headless: !cfg!(feature = "gui"),
        });
        app.add_plugins(server::ServerPlugin);
        #[cfg(feature = "gui")]
        app.add_plugins(renderer::RendererPlugin);
        Self(app)
    }

    pub(crate) fn run(mut self) {
        self.0.run();
    }
}
