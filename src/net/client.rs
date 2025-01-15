use crate::components::*;
use crate::gamemode::*;
use crate::ids::IdPooler;
use crate::load_map;
use crate::monsters::*;
use crate::net::input::*;
use crate::net::messages::*;
use crate::net::*;
use crate::physics::*;
use crate::pickups::*;
use crate::player::*;
use crate::sfx::AudioEvent;
use crate::snapshot::*;
use crate::spawn::*;
use crate::weapons;
use crate::weapons::*;
use bevy_egui::EguiContexts;
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetClient};
use renet_visualizer::RenetClientVisualizer;
use std::time::Duration;

use self::history::SnapshotInterpolation;

pub const CLIENT_SENDRATE_HZ: f64 = 128.0;
pub const CLIENT_SENDRATE_SECONDS: f64 = 1.0 / CLIENT_SENDRATE_HZ;

struct ProjectileMatch {
    entity: Entity,
    id: u16,
}
#[derive(Component)]
pub struct ClientInterpolate {
    pub start_position: Vec3,
    pub end_position: Vec3,
    pub timer: Timer,
}

impl Default for ClientInterpolate {
    fn default() -> Self {
        Self {
            start_position: Vec3::ZERO,
            end_position: Vec3::ZERO,
            timer: Timer::from_seconds(0.0, TimerMode::Once),
        }
    }
}

pub fn input_saver_system(
    local_player: Res<LocalPlayer>,
    mut saved_inputs: ResMut<SavedInputs>,
    player: Query<(&Transform, &MovementState, &Player), With<LocallyOwned>>,
) {
    saved_inputs.clean_old_inputs();

    // modify the latest input to include the final position and velocity processed by that input
    if let Some(latest_input) = saved_inputs.latest_input_mut() {
        for (transform, movement_state, player) in player.iter() {
            if local_player.equals(player) {
                latest_input.final_translation = transform.translation;
                latest_input.final_velocity = movement_state.velocity;
                latest_input.sent = false;
            }
        }
    }
}

pub fn send_message_system(
    time: Res<Time>,
    mut saved_inputs: ResMut<SavedInputs>,
    mut client: ResMut<RenetClient>,
) {
    saved_inputs.delta_time_sum += time.delta_seconds_f64();

    if saved_inputs.delta_time_sum >= CLIENT_SENDRATE_SECONDS * 1.5 {
        // if the delta time sum significantly exceeds the desired interval,
        // reset it to align closer to the desired rate.
        // this is to mitigate floating point errors
        saved_inputs.delta_time_sum = 0.0;
    } else {
        saved_inputs.delta_time_sum -= CLIENT_SENDRATE_SECONDS;

        let last_input: Option<SavedInput> = match saved_inputs.latest_input() {
            Some(input) => Some(input.clone()),
            None => None,
        };

        if let Some(last_input) = last_input {
            let mut last_input = last_input.input;
            for saved_input in saved_inputs.inputs_mut() {
                if saved_input.sent {
                    continue;
                }
                last_input.merge_important_props(&saved_input.input);
                saved_input.sent = true;
            }
            if let Ok(message) = bincode::serialize(&ClientMessage::PlayerInput(last_input.clone()))
            {
                client.send_message(DefaultChannel::Unreliable, message);
            }
        }
    }
}

pub fn weapons_receive_system(
    local_player: Res<LocalPlayer>,
    players: Query<&Player>,
    mut id_tracker: ResMut<IdPooler>,
    mut projectiles: Query<(Entity, &mut Projectile)>,
    mut predicted_projectiles: Query<(&mut Transform, &Children), With<PredictedProjectile>>,
    mut projectile_visuals: Query<(&mut Transform, &ProjectileFx), Without<PredictedProjectile>>,
    mut client: ResMut<RenetClient>,
    mut projectile_spawns: EventWriter<SpawnProjectileEvent>,
    mut projectile_despawns: EventWriter<DespawnProjectileEvent>,
    mut audio_events: EventWriter<AudioEvent>,
    weapon_configs: Res<Assets<WeaponConfig>>,
) {
    while let Some(message) = client.receive_message(crate::net::NetChannel::Weapons) {
        match bincode::deserialize::<WeaponsMessage>(&message) {
            Ok(WeaponsMessage::Projectile(spawn_projectile)) => {
                if spawn_projectile.owner_id != local_player.player_id {
                    projectile_spawns.send(spawn_projectile);
                } else {
                    if let Some(projectile_match) = find_matching_projectile(
                        spawn_projectile.input_id,
                        &spawn_projectile.spawn_translation,
                        &spawn_projectile.velocity,
                        spawn_projectile.weapon_key,
                        &mut projectiles,
                    ) {
                        if let Ok((mut transform, children)) =
                            predicted_projectiles.get_mut(projectile_match.entity)
                        {
                            if let Ok((_, mut projectile)) =
                                projectiles.get_mut(projectile_match.entity)
                            {
                                // offset the visuals back so they don't pop, we lerp them in another system
                                let cached_position = transform.translation;
                                for child in children.iter() {
                                    if let Ok((mut transform, _)) =
                                        projectile_visuals.get_mut(*child)
                                    {
                                        transform.translation =
                                            cached_position - spawn_projectile.spawn_translation;
                                    }
                                }

                                // synch its location
                                transform.translation = spawn_projectile.spawn_translation;
                                projectile.velocity = spawn_projectile.velocity;
                                projectile.nudge_delta_millis = spawn_projectile.nudge_delta_millis;
                            }
                        }
                    }
                }
            }
            Ok(WeaponsMessage::ShotgunFire(event)) => {
                for player in &mut players.iter() {
                    if player.id == event.owner_id {
                        if let Some(weapon_config) =
                            WeaponConfig::get(weapons::WEAPON_KEY_SHOTGUN, &weapon_configs)
                        {
                            Weapon::new(weapons::WEAPON_KEY_SHOTGUN, weapon_config.ammo).fire(
                                &FireWeapon {
                                    seed: event.seed,
                                    origin_transform: &event.origin_transform,
                                    owner_id: event.owner_id,
                                    predicted: false,
                                    proj_nudge_time_millis: 0,
                                    input_id: None,
                                },
                                &mut id_tracker,
                                &mut projectile_spawns,
                                &mut audio_events,
                                weapon_config,
                            );
                        }
                    }
                }
            }
            Ok(WeaponsMessage::DespawnProjectile(despawn_projectile)) => {
                find_matching_projectile(
                    despawn_projectile.input_id,
                    &despawn_projectile.spawn_translation,
                    &despawn_projectile.velocity,
                    despawn_projectile.weapon_key,
                    &mut projectiles,
                )
                .map(|projectile_match| {
                    let mut despawn_projectile = despawn_projectile.clone();
                    despawn_projectile.id = projectile_match.id;
                    projectile_despawns.send(despawn_projectile);
                });
            }
            Err(e) => {
                println!(
                    "Error deserializing server message on Weapons channel: {:?}",
                    e
                );
            }
        }
    }
}

fn find_matching_projectile(
    input_id: Option<u64>,
    spawn_translation: &Vec3,
    velocity: &Vec3,
    weapon_key: u8,
    projectiles: &mut Query<(Entity, &mut Projectile)>,
) -> Option<ProjectileMatch> {
    let mut best_match: Option<Entity> = None;
    let mut best_distance: f32 = f32::MAX;
    for (entity, projectile) in projectiles.iter_mut() {
        // we only care about projectiles related to the same input id
        if projectile.input_id != input_id {
            continue;
        }

        // we only care about projectiles related to the same kind
        if projectile.weapon_key != weapon_key {
            continue;
        }

        // we only care about projectiles moving in the same direction
        let direction = projectile.velocity.normalize();
        let candidate_direction = velocity.normalize();
        if (direction.dot(candidate_direction)) > 0.95 {
            // closest candidate is the best match
            let new_distance = projectile
                .spawn_translation
                .distance_squared(*spawn_translation);
            if best_match.is_none() {
                best_match = Some(entity);
                best_distance = new_distance;
            } else {
                if best_distance > new_distance {
                    best_match = Some(entity);
                }
            }
        }
    }

    if let Some(best_match) = best_match {
        if let Ok((entity, projectile)) = projectiles.get(best_match) {
            return Some(ProjectileMatch {
                entity,
                id: projectile.id,
            });
        }
    }

    None
}

pub fn game_state_receive_system(
    local_player: Res<LocalPlayer>,
    mut players: Query<&mut Player>,
    mut client: ResMut<RenetClient>,
    mut gamemode_controller: ResMut<GameModeController>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    while let Some(message) = client.receive_message(crate::net::NetChannel::GameState) {
        match bincode::deserialize::<GameStateMessage>(&message) {
            Ok(GameStateMessage::MapLoad(map_name, gamemode)) => {
                info!(
                    "Received map load from server. Map: \"{}\" GameMode: \"{:?}\"",
                    map_name, gamemode
                );
                load_map(
                    &map_name,
                    &gamemode,
                    &mut gamemode_controller,
                    &mut next_app_state,
                );
            }
            Ok(GameStateMessage::RespawnCounter(respawn_time)) => {
                for mut player in players.iter_mut() {
                    if player.id == local_player.player_id {
                        player
                            .respawn_timer
                            .set_duration(Duration::from_secs(respawn_time as u64));
                        player.respawn_timer.reset();
                    }
                }
            }
            Err(e) => {
                println!(
                    "Error deserializing server message on GameState channel: {:?}",
                    e
                );
            }
        }
    }
}

pub fn snapshot_receive_system(
    time: Res<Time>,
    saved_inputs: Res<SavedInputs>,
    physics_context: Res<RapierContext>,
    mut commands: Commands,
    mut local_player: ResMut<LocalPlayer>,
    mut client: ResMut<RenetClient>,
    mut snapshot_interpolation: ResMut<SnapshotInterpolation>,
    mut players: Query<(
        Entity,
        &mut Player,
        &mut Team,
        &mut Health,
        &mut Transform,
        &mut MovementState,
        &mut Collider,
    )>,
    mut bots: Query<(Entity, &Monster, &Transform), Without<Player>>,
    mut pickups: Query<(Entity, &Pickup, &Transform), (Without<Player>, Without<Monster>)>,
    mut local_player_visuals: Query<
        (&GlobalTransform, &mut Transform),
        (
            With<LocalPlayerVisuals>,
            Without<Player>,
            Without<Monster>,
            Without<Pickup>,
        ),
    >,
    mut spawn_event_writer: EventWriter<SpawnEvent>,
) {
    let mut snapshot_to_process: Option<Snapshot> = None;

    while let Some(message) = client.receive_message(NetChannel::SnapshotReliable) {
        if let Ok(snapshot) = bincode::deserialize::<Snapshot>(&message) {
            if let Some(last_reconciled_snapshot_id) =
                snapshot_interpolation.latest_reconciled_snapshot_id
            {
                if snapshot.id > last_reconciled_snapshot_id {
                    snapshot_to_process = Some(snapshot);
                }
            } else {
                snapshot_to_process = Some(snapshot);
            }
        }
    }
    while let Some(message) = client.receive_message(NetChannel::SnapshotUnreliable) {
        if let Ok(snapshot) = bincode::deserialize::<Snapshot>(&message) {
            if let Some(last_reconciled_snapshot_id) =
                snapshot_interpolation.latest_reconciled_snapshot_id
            {
                if snapshot.id > last_reconciled_snapshot_id {
                    snapshot_to_process = Some(snapshot);
                }
            } else {
                snapshot_to_process = Some(snapshot);
            }
        }
    }

    if let Some(snapshot) = snapshot_to_process {
        local_player.player_id = snapshot.local_player_id.unwrap_or(local_player.player_id);
        snapshot_interpolation.latest_reconciled_snapshot_id = Some(snapshot.id);
        snapshot_interpolation.latest_reconciled_snapshot_time =
            Some(time.elapsed_seconds_wrapped_f64());

        // ---------------------------------------------------------------------------
        // Reconciles the snapshot with the current state of the game.
        // More updating happens in snapshot_interpolation_system() for interpolation.
        // ---------------------------------------------------------------------------

        // received an absolute snapshot, despawn entities that are not in the snapshot
        if snapshot.absolute {
            for (entity, player, _, _, _, _, _) in players.iter_mut() {
                if snapshot
                    .players
                    .iter()
                    .find(|player_snapshot| player_snapshot.id == player.id)
                    .is_none()
                {
                    if let Some(entity) = commands.get_entity(entity) {
                        entity.despawn_recursive();
                    }
                }
            }
            for (entity, bot, _) in bots.iter_mut() {
                if snapshot
                    .monsters
                    .iter()
                    .find(|bot_snapshot| bot_snapshot.id == bot.id)
                    .is_none()
                {
                    if let Some(entity) = commands.get_entity(entity) {
                        entity.despawn_recursive();
                    }
                }
            }
            for (entity, pickup, _) in pickups.iter_mut() {
                if snapshot
                    .pickups
                    .iter()
                    .find(|pickup_snapshot| pickup_snapshot.id == pickup.id)
                    .is_none()
                {
                    if let Some(entity) = commands.get_entity(entity) {
                        entity.despawn_recursive();
                    }
                }
            }
        }

        if snapshot.players.len() > 0 {
            let mut players = players.iter_mut().collect::<Vec<_>>();

            for player_snapshot in snapshot.players.iter() {
                let player = players
                    .iter_mut()
                    .find(|(_, player, _, _, _, _, _)| player.id == player_snapshot.id);

                if let Some((
                    entity,
                    player,
                    team,
                    health_component,
                    transform,
                    movement_state,
                    collider,
                )) = player
                {
                    //-----------------------------------------------------------------
                    // show/hide player
                    //-----------------------------------------------------------------

                    // show/hide the player and their collision if they're dead
                    if let Some(health) = player_snapshot.health {
                        if health > 0 {
                            crate::utils::turn_on(*entity, &mut commands);
                        } else {
                            crate::utils::turn_off(*entity, &mut commands);
                        }
                    }

                    player_snapshot.update_player(player, team, health_component);

                    //-----------------------------------------------------------------
                    // Reconcile local player
                    //-----------------------------------------------------------------
                    if player_snapshot.id == local_player.player_id {
                        if let Some(new_translation) = player_snapshot.translation {
                            // find saved input for this snapshot
                            let mut saved_input = None;
                            if let Some(last_processed_input_id) =
                                snapshot.local_last_processed_input_id
                            {
                                for input in saved_inputs.inputs() {
                                    if input.input.id == last_processed_input_id {
                                        saved_input = Some(input);
                                        break;
                                    }
                                }
                            }

                            if let Some(saved_input) = saved_input {
                                let error_delta = new_translation - saved_input.final_translation;
                                if error_delta.length() > 0.01 {
                                    player_snapshot.update_player_transform(
                                        false,
                                        transform,
                                        movement_state,
                                    );

                                    // collect all inputs after the last processed input
                                    if let Some(last_processed_input_id) =
                                        snapshot.local_last_processed_input_id
                                    {
                                        let inputs_to_apply = saved_inputs
                                            .inputs()
                                            .iter()
                                            .filter(|saved_input| {
                                                saved_input.input.id >= last_processed_input_id
                                            })
                                            .collect::<Vec<&SavedInput>>();

                                        // apply all inputs after the last processed input
                                        for saved_input in inputs_to_apply {
                                            crate::physics::move_entity(
                                                &entity,
                                                saved_input.input.move_direction,
                                                saved_input.input.look_rotation,
                                                transform,
                                                movement_state,
                                                collider,
                                                &physics_context,
                                                saved_input.delta_seconds,
                                            );
                                        }

                                        // resets the local visuals to keep the same global position
                                        // otherwise there would be a stutter, we lerp back to zero
                                        // in local_player_visuals_system()
                                        if let Ok((global_transform, mut visual_transform)) =
                                            local_player_visuals.get_single_mut()
                                        {
                                            visual_transform.translation = transform
                                                .compute_matrix()
                                                .inverse()
                                                .transform_point3(global_transform.translation());
                                        }
                                    }
                                }
                            } else {
                                // no inputs found at all
                                // this would happen under lag or
                                // receiving snapshot for the first time
                                player_snapshot.update_player_transform(
                                    false,
                                    transform,
                                    movement_state,
                                );
                            }
                        }
                    }
                }
                //-----------------------------------------------------------------
                // spawn new players
                //-----------------------------------------------------------------
                else {
                    if let Some(name) = &player_snapshot.name {
                        spawn_event_writer.send(SpawnEvent::Player(SpawnPlayer {
                            id: player_snapshot.id,
                            name: PlayerSnapshot::decode_name(&name),
                            team: player_snapshot.team.unwrap_or(Team::Spectator),
                            health: player_snapshot.health.unwrap_or(100),
                            position: player_snapshot.translation.unwrap_or_default(),
                        }));
                    }
                }
            }
        }

        //-----------------------------------------------------------------
        // spawn new bots
        //-----------------------------------------------------------------
        if snapshot.monsters.len() > 0 {
            let bots = bots.iter().collect::<Vec<_>>();
            for bot_snapshot in &snapshot.monsters {
                let bot = bots.iter().find(|(_, bot, _)| bot.id == bot_snapshot.id);
                if let None = bot {
                    if let Some(kind) = &bot_snapshot.kind {
                        spawn_event_writer.send(SpawnEvent::Monster(SpawnMonster {
                            id: bot_snapshot.id,
                            seed: bot_snapshot.id,
                            kind: kind.clone(),
                            translation: bot_snapshot
                                .translation
                                .unwrap_or_default()
                                .to_vec3(crate::snapshot::TRANSFORM_QUANTIZE_RANGE),
                        }));
                    }
                }
            }
        }

        //-----------------------------------------------------------------
        // spawn new pickups
        //-----------------------------------------------------------------
        if snapshot.pickups.len() > 0 {
            let pickups = pickups.iter().collect::<Vec<_>>();
            for pickup_snapshot in &snapshot.pickups {
                let pickup = pickups
                    .iter()
                    .find(|(_, pickup, _)| pickup.id == pickup_snapshot.id);
                if let None = pickup {
                    if let Some(kind) = &pickup_snapshot.kind {
                        spawn_event_writer.send(SpawnEvent::Pickup(SpawnPickup {
                            id: pickup_snapshot.id,
                            amount: 0,
                            kind: kind.clone(),
                            translation: pickup_snapshot
                                .translation
                                .unwrap_or_default()
                                .to_vec3(crate::snapshot::TRANSFORM_QUANTIZE_RANGE),
                        }));
                    }
                }
            }
        }

        // -----------------------------------------------------------------
        // despawn players
        // -----------------------------------------------------------------
        for player_id in &snapshot.player_deletions {
            spawn_event_writer.send(SpawnEvent::DespawnPlayer(*player_id));
        }

        // -----------------------------------------------------------------
        // despawn bots
        // -----------------------------------------------------------------
        for bot_id in &snapshot.monster_deletions {
            spawn_event_writer.send(SpawnEvent::DespawnMonster(*bot_id));
        }

        // -----------------------------------------------------------------
        // despawn pickups
        // -----------------------------------------------------------------
        for pickup_id in &snapshot.pickup_deletions {
            spawn_event_writer.send(SpawnEvent::DespawnPickup(*pickup_id));
        }

        snapshot_interpolation.lerp_from = Some(Snapshot {
            id: 0,
            absolute: false,
            players: {
                let mut out = Vec::new();
                for (entity, player, team, health, transform, movement_state, _) in players.iter() {
                    out.push(PlayerSnapshot::from_player(
                        entity,
                        player,
                        team,
                        health,
                        transform,
                        movement_state,
                    ));
                }
                out
            },
            monsters: {
                let mut out = Vec::new();
                for (entity, bot, transform) in bots.iter() {
                    out.push(MonsterSnapshot::from_monster(entity, bot, transform));
                }
                out
            },
            pickups: {
                let mut out = Vec::new();
                for (_, pickup, transform) in pickups.iter() {
                    out.push(PickupSnapshot::from_pickup(pickup, transform));
                }
                out
            },
            local_player_id: None,
            local_last_processed_input_id: None,
            player_deletions: Vec::new(),
            monster_deletions: Vec::new(),
            pickup_deletions: Vec::new(),
        });

        snapshot_interpolation.lerp_to = Some(snapshot);
    }
}

pub fn compute_interpolation_alpha(
    time: Res<Time>,
    snapshot_interpolation: &Res<SnapshotInterpolation>,
) -> f32 {
    if let Some(latest_reconciled_snapshot_time) =
        snapshot_interpolation.latest_reconciled_snapshot_time
    {
        let tickrate = 1.0 / crate::TICKRATE as f64;

        let time_since_latest_reconciled_snapshot =
            time.elapsed_seconds_f64() - latest_reconciled_snapshot_time;
        let alpha = time_since_latest_reconciled_snapshot / tickrate;
        alpha.min(1.0).max(0.0) as f32
    } else {
        0.0
    }
}

pub fn player_interpolation_system(
    time: Res<Time>,
    local_player: Res<LocalPlayer>,
    snapshot_interpolation: Res<SnapshotInterpolation>,
    mut players: Query<(&mut Player, &mut Transform, &mut MovementState)>,
) {
    if let Some(lerp_to) = &snapshot_interpolation.lerp_to {
        if let Some(lerp_from) = &snapshot_interpolation.lerp_from {
            let interpolated_snapshot = Snapshot::interpolate_players(
                local_player.player_id,
                lerp_from,
                lerp_to,
                compute_interpolation_alpha(time, &snapshot_interpolation),
            );

            // -----------------------------------------------------------------
            // Updates the entities with the snapshot data.
            // The local player is updated in snapshot_receive_system().
            // -----------------------------------------------------------------
            let mut players = players.iter_mut().collect::<Vec<_>>();
            for player_snapshot in interpolated_snapshot.players.iter() {
                // local player is updated in snapshot_receive_system()
                if player_snapshot.id == local_player.player_id {
                    continue;
                }

                let player = players
                    .iter_mut()
                    .find(|(player, _, _)| player.id == player_snapshot.id);

                if let Some((_, transform, movement_state)) = player {
                    player_snapshot.update_player_transform(true, transform, movement_state);
                }
            }
        }
    }
}

pub fn bot_interpolation_system(
    time: Res<Time>,
    snapshot_interpolation: Res<SnapshotInterpolation>,
    mut bots: Query<(&mut Monster, &mut Transform), (Without<Player>, Without<MonsterVisuals>)>,
) {
    if let Some(lerp_to) = &snapshot_interpolation.lerp_to {
        if let Some(lerp_from) = &snapshot_interpolation.lerp_from {
            let alpha = compute_interpolation_alpha(time, &snapshot_interpolation);
            for (bot, mut transform) in bots.iter_mut() {
                let from_snap = lerp_from
                    .monsters
                    .iter()
                    .find(|bot_snapshot| bot_snapshot.id == bot.id);

                let to_snap = lerp_to
                    .monsters
                    .iter()
                    .find(|bot_snapshot| bot_snapshot.id == bot.id);

                if let (Some(from_snap), Some(to_snap)) = (from_snap, to_snap) {
                    let from = from_snap
                        .translation
                        .unwrap_or_default()
                        .to_vec3(crate::snapshot::TRANSFORM_QUANTIZE_RANGE);
                    let to = to_snap
                        .translation
                        .unwrap_or_default()
                        .to_vec3(crate::snapshot::TRANSFORM_QUANTIZE_RANGE);
                    transform.translation = from.lerp(to, alpha);

                    let rot_from = from_snap.rotation.unwrap_or_default().to_quat();
                    let rot_to = to_snap.rotation.unwrap_or_default().to_quat();
                    transform.rotation = rot_from.slerp(rot_to, alpha);
                }
            }
        }
    }
}

pub fn pickup_interpolation_system(
    time: Res<Time>,
    snapshot_interpolation: Res<SnapshotInterpolation>,
    mut pickups: Query<
        (&mut Pickup, &mut Transform),
        (Without<Player>, Without<Monster>, Without<MonsterVisuals>),
    >,
) {
    if let Some(lerp_to) = &snapshot_interpolation.lerp_to {
        if let Some(lerp_from) = &snapshot_interpolation.lerp_from {
            let interpolated_snapshot = Snapshot::interpolate_pickups(
                lerp_from,
                lerp_to,
                compute_interpolation_alpha(time, &snapshot_interpolation),
            );

            let mut pickups = pickups.iter_mut().collect::<Vec<_>>();
            for pickup_snapshot in &interpolated_snapshot.pickups {
                {
                    let pickup = pickups
                        .iter_mut()
                        .find(|(pickup, _)| pickup.id == pickup_snapshot.id);
                    if let Some((pickup, transform)) = pickup {
                        pickup_snapshot.update_pickup(pickup, transform);
                    }
                }
            }
        }
    }
}

pub fn visualizer_system(
    mut egui_contexts: EguiContexts,
    mut visualizer: ResMut<RenetClientVisualizer<200>>,
    client: Res<RenetClient>,
    mut show_visualizer: Local<bool>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    visualizer.add_network_info(client.network_info());
    if keyboard_input.just_pressed(KeyCode::F1) {
        *show_visualizer = !*show_visualizer;
    }
    if *show_visualizer {
        visualizer.show_window(egui_contexts.ctx_mut());
    }
}
