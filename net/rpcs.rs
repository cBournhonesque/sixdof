use crate::gamemode::*;
use crate::ids::IdPooler;
use crate::net::messages::*;
use crate::net::*;
use crate::snapshot::*;
use crate::weapons::*;

use bevy_renet::renet::ClientId;
use bevy_renet::renet::RenetServer;

const MTU: usize = 1000;

pub fn server_send_respawn_timer(client_id: &ClientId, server: &mut RenetServer, seconds: u8) {
    if let Ok(message) = bincode::serialize(&GameStateMessage::RespawnCounter(seconds)) {
        server.send_message(*client_id, NetChannel::GameState, message);
    }
}

pub fn server_send_map(
    client_id: &ClientId,
    server: &mut RenetServer,
    map: &str,
    gamemode: GameMode,
) {
    if let Ok(message) = bincode::serialize(&GameStateMessage::MapLoad(map.to_string(), gamemode)) {
        server.send_message(*client_id, NetChannel::GameState, message);
    }
}

pub fn server_send_map_to_all(server: &mut RenetServer, map: &str, gamemode: GameMode) {
    if let Ok(message) = bincode::serialize(&GameStateMessage::MapLoad(map.to_string(), gamemode)) {
        server.clients_id().iter().for_each(|client_id| {
            server.send_message(*client_id, NetChannel::GameState, message.clone());
        });
    }
}

pub fn server_send_snapshot(client_id: &ClientId, server: &mut RenetServer, snapshot: &Snapshot) {
    if let Ok(message) = bincode::serialize(&snapshot) {
        if message.len() < MTU {
            server_send_snapshot_unreliable(client_id, server, message);
        } else {
            server_send_snapshot_reliable(client_id, server, message);
        }
    }
}

fn server_send_snapshot_unreliable(
    client_id: &ClientId,
    server: &mut RenetServer,
    message: Vec<u8>,
) {
    server.send_message(*client_id, NetChannel::SnapshotUnreliable, message);
}

fn server_send_snapshot_reliable(client_id: &ClientId, server: &mut RenetServer, message: Vec<u8>) {
    server.send_message(*client_id, NetChannel::SnapshotReliable, message);
}

pub fn server_send_projectile(server: &mut RenetServer, event: &SpawnProjectileEvent) {
    if let Ok(message) = bincode::serialize(&WeaponsMessage::Projectile(event.clone())) {
        server.clients_id().iter().for_each(|client_id| {
            server.send_message(*client_id, NetChannel::Weapons, message.clone());
        });
    }
}

pub fn server_send_despawn_projectile(server: &mut RenetServer, event: &DespawnProjectileEvent) {
    if let Ok(message) = bincode::serialize(&WeaponsMessage::DespawnProjectile(event.clone())) {
        server.clients_id().iter().for_each(|client_id| {
            server.send_message(*client_id, NetChannel::Weapons, message.clone());
        });
    }
}

pub fn server_send_shotgun_fire(
    idpooler: &IdPooler,
    server: &mut RenetServer,
    event: &ShotgunFireEvent,
) {
    if let Ok(message) = bincode::serialize(&WeaponsMessage::ShotgunFire(event.clone())) {
        server.clients_id().iter().for_each(|client_id| {
            if let Some(player_id) = idpooler.get_player_id(client_id.raw()) {
                // we do not send to the owner
                if *player_id != event.owner_id {
                    server.send_message(*client_id, NetChannel::Weapons, message.clone());
                }
            }
        });
    }
}
