use crate::{in_playing_state, player::Player, AppState, PlayingSubState};
use bevy::prelude::*;
use bevy_renet::renet::{ChannelConfig, ConnectionConfig, RenetClient, RenetServer, SendType};
use renet_visualizer::{RenetClientVisualizer, RenetVisualizerStyle};
use std::time::Duration;

pub mod client;
pub mod input;
pub mod messages;
pub mod rpcs;
pub mod serialize;
pub mod server;

pub const AUTHORITY_ID: u64 = 0;

pub struct NetPlugin;
impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                client::snapshot_receive_system,
                client::weapons_receive_system,
                client::player_interpolation_system,
                client::pickup_interpolation_system,
            )
                .chain()
                .run_if(in_playing_state)
                .run_if(resource_exists::<RenetClient>),
        )
        .add_systems(
            PreUpdate,
            (
                server::handle_events_system,
                server::receive_message_system.run_if(in_playing_state),
                server::player_input_reader_system.run_if(in_playing_state),
            )
                .chain()
                .run_if(resource_exists::<RenetServer>),
        )
        .add_systems(
            Update,
            (client::bot_interpolation_system, client::visualizer_system)
                .chain()
                .run_if(in_playing_state)
                .run_if(resource_exists::<RenetClient>),
        )
        .add_systems(
            PostUpdate,
            (client::input_saver_system, client::send_message_system)
                .chain()
                .run_if(in_playing_state)
                .run_if(resource_exists::<RenetClient>),
        )
        .add_systems(
            PostUpdate,
            (server::projectile_replicator_system)
                .chain()
                .run_if(resource_exists::<RenetServer>),
        )
        .add_systems(
            FixedPostUpdate,
            (server::server_send_snapshot_system)
                .chain()
                .run_if(resource_exists::<RenetServer>)
                .run_if(in_playing_state),
        )
        .add_systems(
            FixedUpdate,
            (client::game_state_receive_system)
                .chain()
                .run_if(resource_exists::<RenetClient>),
        )
        .insert_resource(RenetClientVisualizer::<200>::new(
            RenetVisualizerStyle::default(),
        ));
    }
}

#[derive(Component)]
pub struct LocallyOwned;

pub fn connection_config() -> ConnectionConfig {
    ConnectionConfig {
        available_bytes_per_tick: 1024 * 1024,
        client_channels_config: NetChannel::channels_config(),
        server_channels_config: NetChannel::channels_config(),
    }
}

pub enum NetChannel {
    SnapshotUnreliable,
    SnapshotReliable,
    GameState,
    Weapons,
}

impl From<NetChannel> for u8 {
    fn from(channel_id: NetChannel) -> Self {
        match channel_id {
            NetChannel::SnapshotUnreliable => 0,
            NetChannel::SnapshotReliable => 1,
            NetChannel::GameState => 2,
            NetChannel::Weapons => 3,
        }
    }
}

impl NetChannel {
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ChannelConfig {
                channel_id: Self::SnapshotUnreliable.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::Unreliable,
            },
            ChannelConfig {
                channel_id: Self::SnapshotReliable.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::ReliableUnordered {
                    resend_time: Duration::from_millis(5),
                },
            },
            ChannelConfig {
                channel_id: Self::GameState.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(50),
                },
            },
            ChannelConfig {
                channel_id: Self::Weapons.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::Unreliable,
            },
        ]
    }
}
