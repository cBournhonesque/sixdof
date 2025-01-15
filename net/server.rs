use crate::components::*;
use crate::gamemode::*;
use crate::ids::IdPooler;
use crate::net::input::*;
use crate::net::messages::*;
use crate::net::LocallyOwned;
use crate::physics::*;
use crate::player::*;
use crate::snapshot::{history::*, *};
use crate::utils;
use crate::weapons;
use crate::weapons::*;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetServer, ServerEvent};

#[derive(Event, Clone)]
pub struct NetPlayerFiredEvent {
    pub player_id: u8,
    pub input_id: u64,
    pub projectile_nudge_time_millis: u8,
    pub input: PlayerInput,
}

pub fn server_send_snapshot_system(
    idpooler: Res<IdPooler>,
    players: Query<&Player>,
    mut server: ResMut<RenetServer>,
    snapshot_history: Res<SnapshotHistory>,
) {
    let mut latest_processed_input_ids: HashMap<u8, u64> = HashMap::new();
    for player in players.iter() {
        if let Some(latest_processed_input) = &player.latest_processed_input {
            latest_processed_input_ids.insert(player.id, latest_processed_input.id);
        }
    }

    if let Some(snapshot) = snapshot_history.server_snapshots.values().last() {
        let mut snapshot = snapshot.clone();
        // send the snapshot to all clients
        for client_id in server.clients_id() {
            let last_processed_input_id = idpooler
                .get_player_id(client_id.raw())
                .and_then(|player_id| latest_processed_input_ids.get(player_id));

            if let Some(last_processed_input_id) = last_processed_input_id {
                let mut send_absolute = true;

                snapshot.local_player_id = idpooler.get_player_id(client_id.raw()).cloned();
                snapshot.local_last_processed_input_id = Some(*last_processed_input_id);

                if let Some(last_acked_snapshot_id) =
                    snapshot_history.server_last_acked_snapshots.get(&client_id)
                {
                    if let Some(last_acked_snapshot) = snapshot_history
                        .server_snapshots
                        .get(last_acked_snapshot_id)
                    {
                        crate::net::rpcs::server_send_snapshot(
                            &client_id,
                            &mut server,
                            &Snapshot::diff_snapshots(
                                &snapshot,
                                last_acked_snapshot,
                                &snapshot_history.bubble_up_deletions(&client_id),
                            ),
                        );
                        send_absolute = false;
                    }
                }

                if send_absolute {
                    crate::net::rpcs::server_send_snapshot(&client_id, &mut server, &snapshot);
                }
            }
        }
    }
}

pub fn receive_message_system(
    idpooler: Res<IdPooler>,
    mut server: ResMut<RenetServer>,
    mut input_event_writer: EventWriter<NetPlayerInputEvent>,
    mut snapshot_history: ResMut<SnapshotHistory>,
) {
    // Receive message from all clients
    for client_id in server.clients_id() {
        let player_id = idpooler.get_player_id(client_id.raw());

        if let Some(player_id) = player_id {
            while let Some(message) = server.receive_message(client_id, DefaultChannel::Unreliable)
            {
                let decoded = bincode::deserialize::<ClientMessage>(&message);
                match decoded {
                    Ok(ClientMessage::PlayerInput(player_input)) => {
                        let player_input_event = NetPlayerInputEvent {
                            input: player_input.clone(),
                            player_id: *player_id,
                        };

                        if let Some(snapshot_id) = player_input.snapshot_id {
                            // ack the snapshot
                            snapshot_history
                                .server_last_acked_snapshots
                                .insert(client_id, snapshot_id);
                        }

                        input_event_writer.send(player_input_event);
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn handle_events_system(
    game_mode_controller: Res<GameModeController>,
    mut idpooler: ResMut<IdPooler>,
    mut snapshot_history: ResMut<SnapshotHistory>,
    mut server_events: EventReader<ServerEvent>,
    mut game_mode_signals: EventWriter<GameModeCommands>,
    mut server: ResMut<RenetServer>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                crate::net::rpcs::server_send_map(
                    client_id,
                    &mut server,
                    &game_mode_controller.map,
                    game_mode_controller.game_mode.clone(),
                );
                game_mode_signals.send(GameModeCommands::PlayerConnected(*client_id));
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                if let Some(player_id) = idpooler.get_player_id(client_id.raw()) {
                    game_mode_signals.send(GameModeCommands::PlayerDisconnected(*player_id));
                }
                snapshot_history
                    .server_last_acked_snapshots
                    .remove(&client_id);
                idpooler.release_player_id(client_id.raw());

                println!("Client disconnected: {:?} {:?}", client_id, reason);
            }
        }
    }
}

pub fn player_input_reader_system(
    time: Res<Time>,
    physics_context: Res<RapierContext>,
    mut players: Query<
        (
            Entity,
            &Health,
            &mut Player,
            &mut Transform,
            &mut MovementState,
            &Collider,
        ),
        Without<LocallyOwned>,
    >,
    mut weapon_fired_writer: EventWriter<NetPlayerFiredEvent>,
    mut input_event_reader: EventReader<NetPlayerInputEvent>,
) {
    let mut inputs: HashMap<u8, Vec<&NetPlayerInputEvent>> = HashMap::new();
    for input in input_event_reader.read() {
        let player_inputs = inputs.entry(input.player_id).or_insert(Vec::new());
        player_inputs.push(input);
    }

    for (entity, health, mut player, mut transform, mut movement_state, collider) in
        players.iter_mut()
    {
        if health.dead() {
            continue;
        }

        let inputs = inputs.get(&player.id);
        if inputs.is_none() {
            if let Some(last_processed_input) = &player.latest_processed_input {
                // apply last input if no new inputs
                crate::physics::move_entity(
                    &entity,
                    last_processed_input.move_direction,
                    last_processed_input.look_rotation,
                    &mut transform,
                    &mut movement_state,
                    &collider,
                    &physics_context,
                    time.delta_seconds(),
                );
            }
        } else if let Some(inputs) = inputs {
            // apply inputs
            let delta_seconds = time.delta_seconds() / inputs.len() as f32;
            for input in inputs {
                crate::physics::move_entity(
                    &entity,
                    input.input.move_direction,
                    input.input.look_rotation,
                    &mut transform,
                    &mut movement_state,
                    &collider,
                    &physics_context,
                    delta_seconds,
                );

                if input.input.weapon_key.is_some() {
                    weapon_fired_writer.send(NetPlayerFiredEvent {
                        player_id: player.id,
                        input_id: input.input.id,
                        projectile_nudge_time_millis: if let Some(snapshot_id) =
                            input.input.snapshot_id
                        {
                            // nudge forward by their half ping-time (remember, id is a timestamp)
                            ((utils::timestamp_millis_since_epoch() - snapshot_id) as f32 * 0.5)
                                as u8
                        } else {
                            0
                        },
                        input: input.input.clone(),
                    });
                }

                player.latest_processed_input = Some(input.input.clone());
            }
        }
    }
}

pub fn projectile_replicator_system(
    idpooler: Res<IdPooler>,
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut projectile_despawns: EventReader<DespawnProjectileEvent>,
    mut shotgun_events: EventReader<ShotgunFireEvent>,
    mut projectiles: Query<(Entity, &Projectile, &Transform), Without<HasBeenReplicatedOnce>>,
) {
    for event in shotgun_events.read() {
        crate::net::rpcs::server_send_shotgun_fire(&idpooler, &mut server, &event);
    }

    for (entity, projectile, transform) in projectiles.iter_mut() {
        // shotguns are too spammy so we replicate a single shotgun *event* instead
        if projectile.weapon_key == weapons::WEAPON_KEY_SHOTGUN {
            continue;
        }

        let spawn_projectile = SpawnProjectileEvent {
            id: projectile.id,
            weapon_key: projectile.weapon_key.clone(),
            velocity: projectile.velocity,
            owner_id: projectile.owner_id,
            input_id: projectile.input_id,
            current_translation: transform.translation,
            spawn_translation: projectile.spawn_translation,

            // these dont synch across the net, so these can be whatever
            nudge_delta_millis: 0,
            predicted: false,
        };

        crate::net::rpcs::server_send_projectile(&mut server, &spawn_projectile);

        commands.entity(entity).insert(HasBeenReplicatedOnce);
    }

    for event in projectile_despawns.read() {
        if event.hit_damageable && event.weapon_key != weapons::WEAPON_KEY_SHOTGUN {
            crate::net::rpcs::server_send_despawn_projectile(&mut server, &event);
        }
    }
}
