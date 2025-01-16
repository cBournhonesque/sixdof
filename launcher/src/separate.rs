/// ListenServer where one thread runs the client and one thread runs the server
use bevy::asset::AssetPlugin;
use bevy::diagnostic::DiagnosticsPlugin;
use bevy::hierarchy::HierarchyPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use lightyear::prelude::client::{ClientTransport, ClientPlugins};
use lightyear::prelude::server::{ServerPlugins, ServerTransport};
use lightyear::transport::LOCAL_SOCKET;
use crate::settings;


pub struct Separate {
    client: App,
    server: App,
}
impl Separate {
    pub fn new(client_id: u64) -> Self {
        let mut client_config = settings::client_config(client_id);
        // we will communicate between the client and server apps via channels
        let (from_server_send, from_server_recv) = crossbeam_channel::unbounded();
        let (to_server_send, to_server_recv) = crossbeam_channel::unbounded();
        let transport_config = ClientTransport::LocalChannel {
            recv: from_server_recv,
            send: to_server_send,
        };
        client_config.net = settings::build_client_netcode_config(client_id, transport_config);

        let mut server_config = settings::server_config();
        server_config.net.push(settings::build_server_netcode_config(ServerTransport::Channels {
            // even if we communicate via channels, we need to provide a socket address for the client
            channels: vec![(LOCAL_SOCKET, to_server_recv, from_server_send)],
        }));

        // gui app
        let mut client_app = App::new();
        let mut server_app = App::new();
        client_app.add_plugins(
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
        server_app.add_plugins((
            MinimalPlugins,
            settings::log_plugin(),
            StatesPlugin,
            HierarchyPlugin,
            DiagnosticsPlugin,
        ));
        client_app.add_plugins(ClientPlugins { config: client_config });
        server_app.add_plugins(ServerPlugins { config: server_config });
        client_app.add_plugins(shared::SharedPlugin { headless: false });
        server_app.add_plugins(shared::SharedPlugin { headless: false });
        client_app.add_plugins(renderer::RendererPlugin);
        Self {
            client: client_app,
            server: server_app
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