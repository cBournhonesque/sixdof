use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use core::time::Duration;
use bevy::window::{CursorOptions, PresentMode};
#[cfg(feature = "client")]
use lightyear::prelude::client::*;
#[cfg(feature = "server")]
use lightyear::prelude::server::*;
use lightyear::prelude::*;
#[cfg(feature = "server")]
use lightyear_examples_common::server::WebTransportCertificateSettings;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

pub const TICK_RATE: f64 = 64.0;
pub const REPLICATION_INTERVAL: Duration = Duration::from_millis(20);
pub const ASSETS_HOTRELOAD: bool = true;

pub const SERVER_ADDR: SocketAddr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 5001));
pub const LOCAL_SERVER_ADDR: SocketAddr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 5001));
pub const PROTOCOL_ID: u64 = 0;
pub const PRIVATE_KEY: [u8; 32] = [0; 32];
pub const LINK_CONDITIONER: Option<LinkConditionerConfig> = Some(LinkConditionerConfig {
    incoming_latency: Duration::from_millis(50),
    incoming_jitter: Duration::from_millis(5),
    incoming_loss: 0.05,
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

#[cfg(feature = "client")]
pub(crate) fn client(client_id: u64) -> impl Bundle {
    let client_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0);
    let auth = Authentication::Manual {
            server_addr: SERVER_ADDR,
            client_id,
            private_key: PRIVATE_KEY,
            protocol_id: PROTOCOL_ID,
    };
    let netcode_config = client::NetcodeConfig {
        // Make sure that the server times out clients when their connection is closed
        client_timeout_secs: 3,
        token_expire_secs: -1,
        ..default()
    };
    let certificate_digest = {
        #[cfg(target_family = "wasm")]
        {
            include_str!("../../certificates/digest.txt").to_string()
        }
        #[cfg(not(target_family = "wasm"))]
        {
            "".to_string()
        }
    };
    let conditioner = LINK_CONDITIONER.map(|c| RecvLinkConditioner::new(c));
    (
        Client::default(),
        Link::new(conditioner),
        LocalAddr(client_addr),
        PeerAddr(SERVER_ADDR),
        ReplicationReceiver::default(),
        PredictionManager::default(),
        InterpolationManager::default(),
        NetcodeClient::new(auth, netcode_config).unwrap(),
        WebTransportClientIo { certificate_digest },
        Name::from("Client"),
    )
}

#[cfg(feature = "server")]
pub(crate) fn server() -> impl Bundle {
    let certificate = &WebTransportCertificateSettings::FromFile {
        cert: "certificates/cert.pem".to_string(),
        key: "certificates/key.pem".to_string(),
    };
    (
        LocalAddr(LOCAL_SERVER_ADDR),
        NetcodeServer::new(server::NetcodeConfig {
            protocol_id: PROTOCOL_ID,
            private_key: PRIVATE_KEY,
            ..Default::default()
        }),
        WebTransportServerIo {
            certificate: certificate.into(),
        },
        Name::from("Server"),
    )
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