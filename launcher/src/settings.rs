use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy::utils::Duration;
use bevy::window::{CursorOptions, PresentMode};
#[cfg(feature = "client")]
use lightyear::prelude::client::*;
#[cfg(feature = "server")]
use lightyear::prelude::server::{self, ServerConfig, ServerTransport};
use lightyear::prelude::*;
#[cfg(feature = "server")]
use lightyear_examples_common::settings::WebTransportCertificateSettings;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

pub const TICK_RATE: f64 = 64.0;
pub const REPLICATION_INTERVAL: Duration = Duration::from_millis(20);
pub const ASSETS_HOTRELOAD: bool = true;

pub const SERVER_ADDR: SocketAddr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 5001));
pub const PROTOCOL_ID: u64 = 0;
pub const PRIVATE_KEY: [u8; 32] = [0; 32];
pub const LINK_CONDITIONER: Option<LinkConditionerConfig> = Some(LinkConditionerConfig {
    incoming_latency: Duration::from_millis(100),
    incoming_jitter: Duration::from_millis(10),
    incoming_loss: 0.0,
});

pub(crate) fn get_assets_path() -> String {
    const ASSETS_PATH: &'static str = "../assets";

    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let asset_path = current_dir.join(ASSETS_PATH);
    asset_path
        .canonicalize()
        .expect("Failed to canonicalize asset path")
        .to_str()
        .expect("Failed to convert path to string")
        .to_string()
}

pub(crate) fn shared_config() -> SharedConfig {
    SharedConfig {
        client_replication_send_interval: REPLICATION_INTERVAL,
        server_replication_send_interval: REPLICATION_INTERVAL,
        tick: TickConfig {
            tick_duration: Duration::from_secs_f64(1.0 / TICK_RATE),
        },
    }
}

#[cfg(feature = "client")]
pub(crate) fn client_config(client_id: u64) -> ClientConfig {
    ClientConfig {
        shared: shared_config(),
        net: build_client_netcode_config(
            client_id,
            ClientTransport::WebTransportClient {
                // port of 0 means that the OS will find a random port
                client_addr: SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0),
                server_addr: SERVER_ADDR,
                #[cfg(target_family = "wasm")]
                certificate_digest: include_str!("../../certificates/digest.txt").to_string(),
            },
        ),
        ..default()
    }
}

#[cfg(feature = "server")]
pub(crate) fn server_config() -> ServerConfig {
    ServerConfig {
        shared: shared_config(),
        net: vec![build_server_netcode_config(
            ServerTransport::WebTransportServer {
                server_addr: SERVER_ADDR,
                certificate: (&WebTransportCertificateSettings::FromFile {
                    cert: "certificates/cert.pem".to_string(),
                    key: "certificates/key.pem".to_string(),
                })
                    .into(),
            },
        )],
        ..default()
    }
}

#[cfg(feature = "gui")]
pub(crate) fn window_plugin() -> WindowPlugin {
    WindowPlugin {
        primary_window: Some(Window {
            cursor_options: CursorOptions {
                visible: true,
                ..default()
            },
            title: format!("Lightyear Example: {}", env!("CARGO_PKG_NAME")),
            resolution: (1024., 768.).into(),
            present_mode: PresentMode::AutoVsync,
            // set to true if we want to capture tab etc in wasm
            prevent_default_event_handling: true,
            ..Default::default()
        }),
        ..default()
    }
}

pub(crate) fn log_plugin() -> LogPlugin {
    LogPlugin {
        level: Level::INFO,
        filter: "wgpu=error,bevy_render=info,bevy_ecs=warn,bevy_time=warn".to_string(),
        ..default()
    }
}

#[cfg(feature = "server")]
/// Build a netcode config for the server
pub(crate) fn build_server_netcode_config(transport: ServerTransport) -> server::NetConfig {
    server::NetConfig::Netcode {
        config: server::NetcodeConfig::default()
            .with_protocol_id(PROTOCOL_ID)
            .with_key(PRIVATE_KEY),
        io: server::IoConfig {
            transport,
            // TODO: add conditioner here?
            conditioner: None,
            ..default()
        },
    }
}

#[cfg(feature = "client")]
/// Build a netcode config for the client
pub(crate) fn build_client_netcode_config(
    client_id: u64,
    transport_config: ClientTransport,
) -> NetConfig {
    NetConfig::Netcode {
        auth: Authentication::Manual {
            server_addr: SERVER_ADDR,
            client_id,
            private_key: PRIVATE_KEY,
            protocol_id: PROTOCOL_ID,
        },
        config: NetcodeConfig {
            // Make sure that the server times out clients when their connection is closed
            client_timeout_secs: 3,
            ..default()
        },
        io: IoConfig {
            transport: transport_config,
            conditioner: LINK_CONDITIONER,
            compression: CompressionConfig::None,
        },
    }
}
