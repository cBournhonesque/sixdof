/// HostServer where one thread runs the client and one thread runs the server
use core::time::Duration;
use crate::settings;
use bevy::asset::AssetPlugin;
use bevy::diagnostic::DiagnosticsPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use lightyear::prelude::client::{ClientPlugins};
use lightyear::prelude::server::{ServerPlugins};
use crate::settings::TICK_RATE;

pub struct Separate {
    client: App,
    server: App,
}
impl Separate {
    pub fn new(client_id: u64) -> Self {
        // we will communicate between the client and server apps via channels
        let (crossbeam_client, crossbeam_server) = lightyear::crossbeam::CrossbeamIo::new_pair();

        // gui app
        let mut client_app = App::new();
        let mut server_app = App::new();
        client_app.add_plugins(
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
        server_app.add_plugins((
            MinimalPlugins,
            settings::log_plugin(),
            StatesPlugin,
            DiagnosticsPlugin,
        ));
        let tick_duration =  Duration::from_secs_f64(1.0 / TICK_RATE);
        client_app.add_plugins(ClientPlugins { tick_duration });
        server_app.add_plugins(ServerPlugins { tick_duration });
        client_app.add_plugins(shared::SharedPlugin { headless: false });
        server_app.add_plugins(shared::SharedPlugin { headless: true });
        client_app.add_plugins(renderer::RendererPlugin);

        // spawn server
        server_app.world_mut().spawn(settings::server());

        // TODO: spawn local client and client_of using crossbeam IO

        Self {
            client: client_app,
            server: server_app,
        }
    }

    pub(crate) fn run(mut self) {
        let mut send_app = SendApp(self.server);
        std::thread::spawn(move || send_app.run());
        self.client.run();
    }
}

/// App that is Send.
/// Used as a convenient workaround to send an App to a separate thread,
/// if we know that the App doesn't contain NonSend resources.
struct SendApp(App);

unsafe impl Send for SendApp {}

impl SendApp {
    pub fn run(mut self) {
        self.0.run();
    }
}
