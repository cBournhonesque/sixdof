use std::time::Duration;
use crate::settings;
use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::DefaultPlugins;
use lightyear::prelude::{client::ClientPlugins, server::ServerPlugins, Client, LinkOf};
use crate::settings::TICK_RATE;

pub struct HostServer(App);
impl HostServer {
    pub fn new(client_id: u64) -> Self {
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
        let tick_duration =  Duration::from_secs_f64(1.0 / TICK_RATE);
        app.add_plugins(ClientPlugins { tick_duration });
        app.add_plugins(ServerPlugins { tick_duration });
        app.add_plugins(shared::SharedPlugin { headless: false });
        app.add_plugins(client::ClientPlugin);
        app.add_plugins(server::ServerPlugin);
        app.add_plugins(renderer::RendererPlugin);

        // spawn server
        let server = app.world_mut().spawn(settings::server()).id();
        // spawn host client
        app.world_mut().spawn((
            Client::default(),
            Name::new("HostClient"),
            LinkOf { server },
        ));
        Self(app)
    }

    pub(crate) fn run(mut self) {
        self.0.run();
    }
}
