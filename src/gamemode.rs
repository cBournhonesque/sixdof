use crate::components::*;
use crate::has_authority;
use crate::ids::IdPooler;
use crate::in_playing_state;
use crate::is_dedicated_server;
use crate::is_loading_map;
use crate::monsters::*;
use crate::pickups::*;
use crate::player::*;
use crate::scripting::conversions::ScriptMonster;
use crate::scripting::conversions::ScriptPlayer;
use crate::scripting::ScriptContainer;
use crate::snapshot::history::SnapshotHistory;
use crate::spawn::*;
use crate::AppState;
use crate::PlayingSubState;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use bevy_renet::renet::ClientId;
use bevy_renet::renet::RenetServer;
use qevy::components::*;
use qevy::PostBuildMapEvent;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const RESPAWN_TIME: u8 = 2;
const STARTING_HEALTH: i16 = 200;

#[derive(Default)]
pub struct GameModePlugin {
    pub game_mode: GameMode,
    pub map: String,
}

impl Plugin for GameModePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                damage_system,
                server_player_dead_sender_system.run_if(resource_exists::<RenetServer>),
                respawn_system,
            )
                .chain()
                .run_if(has_authority),
        )
        .add_systems(PostUpdate, (post_update).chain().run_if(has_authority))
        .add_systems(
            Update,
            (
                scoreboard_system.run_if(in_playing_state.and_then(not(is_dedicated_server))),
                post_build_map_system.run_if(is_loading_map),
            )
                .chain(),
        )
        .init_resource::<GameModeController>()
        .add_event::<GameModeEvent>()
        .add_event::<GameModeCommands>();
    }
}

#[derive(Event)]
pub enum GameModeEvent {
    OnPlayerDead(Entity),
}

#[derive(Event)]
pub enum GameModeCommands {
    Start,
    PlayerConnected(ClientId),
    PlayerDisconnected(u8),
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum GameMode {
    #[default]
    SinglePlayer,
    Coop,
    Deathmatch,
    TeamDeathmatch,
}

#[derive(Resource, Debug)]
pub struct GameModeController {
    pub map: String,
    pub game_mode: GameMode,
}

impl GameModeController {
    pub fn new(game_mode: GameMode, map: String) -> Self {
        Self { map, game_mode }
    }
}

impl Default for GameModeController {
    fn default() -> Self {
        Self {
            map: "m4".to_string(),
            game_mode: GameMode::SinglePlayer,
        }
    }
}

impl GameModeController {
    pub fn get_game_mode_name(&self) -> &str {
        match self.game_mode {
            GameMode::SinglePlayer => "Single Player",
            GameMode::Coop => "Coop",
            GameMode::Deathmatch => "Deathmatch",
            GameMode::TeamDeathmatch => "Team Deathmatch",
        }
    }

    pub fn can_damage(
        &self,
        instigator: &Entity,
        victim: &Entity,
        instigator_team: &Team,
        victim_team: &Team,
    ) -> bool {
        // can always hurt self
        if instigator == victim {
            return true;
        }

        match self.game_mode {
            GameMode::SinglePlayer => true,
            GameMode::Coop => {
                if instigator_team == victim_team {
                    false
                } else {
                    true
                }
            }
            GameMode::Deathmatch => true,
            GameMode::TeamDeathmatch => {
                if instigator_team == victim_team {
                    false
                } else {
                    true
                }
            }
        }
    }
}

fn post_update(
    local_player: Res<LocalPlayer>,
    spawn_points: Query<&Transform, (With<PlayerSpawnPoint>, Without<Player>)>,
    controller: Res<GameModeController>,
    mut idpooler: ResMut<IdPooler>,
    mut game_mode_signal_reader: EventReader<GameModeCommands>,
    mut spawn_event_writer: EventWriter<SpawnEvent>,
) {
    if !local_player.has_authority() {
        return;
    }

    for signal in game_mode_signal_reader.read() {
        match signal {
            GameModeCommands::Start => {
                for player_id in idpooler.get_all_player_ids() {
                    create_player(
                        *player_id,
                        &match controller.game_mode {
                            GameMode::Deathmatch => Team::Deathmatch,
                            GameMode::TeamDeathmatch => Team::AntiVirus,
                            _ => Team::AntiVirus,
                        },
                        &mut spawn_event_writer,
                        &spawn_points,
                    );
                }
            }
            GameModeCommands::PlayerConnected(client_id) => {
                if let Ok(player_id) = idpooler.assign_and_reserve_player_id(client_id.raw()) {
                    let team = match controller.game_mode {
                        GameMode::Deathmatch => Team::Deathmatch,
                        GameMode::TeamDeathmatch => Team::AntiVirus,
                        _ => Team::AntiVirus,
                    };
                    create_player(player_id, &team, &mut spawn_event_writer, &spawn_points);
                }
            }
            GameModeCommands::PlayerDisconnected(player_id) => {
                spawn_event_writer.send(SpawnEvent::DespawnPlayer(*player_id));
            }
        }
    }
}

pub fn damage_system(
    local_player: Res<LocalPlayer>,
    game_mode: Res<GameModeController>,
    teams: Query<&Team>,
    mut commands: Commands,
    mut players: Query<(Entity, &Transform, &mut Player)>,
    monsters: Query<(&Transform, &Monster), Without<Player>>,
    mut healths: Query<&mut Health>,
    mut damage_events: EventReader<DamageEvent>,
    mut spawn_event_writer: EventWriter<SpawnEvent>,
    mut game_mode_event_writer: EventWriter<GameModeEvent>,
    mut script_container: NonSendMut<ScriptContainer>,
) {
    if !local_player.has_authority() {
        return;
    }

    for event in damage_events.read() {
        if let Ok(mut health) = healths.get_mut(event.victim) {
            let already_dead = health.dead();

            let instigator_team = teams.get(event.instigator);
            let victim_team = teams.get(event.victim);
            if let Ok(instigator_team) = instigator_team {
                if let Ok(victim_team) = victim_team {
                    // leave it to the game mode to decide if the instigator can damage the victim
                    if game_mode.can_damage(
                        &event.instigator,
                        &event.victim,
                        instigator_team,
                        victim_team,
                    ) {
                        health.decrement(event.amount);
                    }
                }
            }

            if health.dead() && !already_dead {
                let mut player_victim = None;
                let mut player_instigator = None;
                let mut monster_victim = None;
                let mut monster_instigator = None;

                if let Ok((_, transform, player)) = players.get(event.victim) {
                    if let Ok(health) = healths.get(event.victim) {
                        player_victim =
                            Some(ScriptPlayer::from_components(&transform, &health, &player));
                    }
                }

                if let Ok((_, transform, player)) = players.get(event.instigator) {
                    if let Ok(health) = healths.get(event.instigator) {
                        player_instigator =
                            Some(ScriptPlayer::from_components(&transform, &health, &player));
                    }
                }

                if let Ok((transform, monster)) = monsters.get(event.victim) {
                    if let Ok(health) = healths.get(event.victim) {
                        monster_victim = Some(ScriptMonster::from_components(
                            &transform, &health, &monster,
                        ));
                    }
                }

                if let Ok((transform, monster)) = monsters.get(event.instigator) {
                    if let Ok(health) = healths.get(event.instigator) {
                        monster_instigator = Some(ScriptMonster::from_components(
                            &transform, &health, &monster,
                        ));
                    }
                }

                if let Some((player_insigator, player_victim)) =
                    player_instigator.clone().zip(player_victim.clone())
                {
                    script_container.on_player_fragged_player(&player_insigator, &player_victim);
                }

                if let Some((monster_instigator, player_victim)) =
                    monster_instigator.clone().zip(player_victim)
                {
                    script_container.on_monster_fragged_player(&monster_instigator, &player_victim);
                }

                if let Some((player_instigator, monster_victim)) =
                    player_instigator.zip(monster_victim.clone())
                {
                    script_container.on_player_fragged_monster(&player_instigator, &monster_victim);
                }

                if let Some((monster_instigator, monster_victim)) =
                    monster_instigator.zip(monster_victim)
                {
                    script_container
                        .on_monster_fragged_monster(&monster_instigator, &monster_victim);
                }

                if let Ok((_, monster)) = monsters.get(event.victim) {
                    spawn_event_writer.send(SpawnEvent::DespawnMonster(monster.id));
                } else if let Ok((entity, _, mut player)) = players.get_mut(event.victim) {
                    if local_player.has_authority() {
                        set_player_respawn(&mut player);
                        crate::utils::turn_off(entity, &mut commands);
                        game_mode_event_writer.send(GameModeEvent::OnPlayerDead(entity));
                    }
                }
            }
        }
    }
}

fn set_player_respawn(player: &mut Player) {
    player
        .respawn_timer
        .set_duration(Duration::from_secs(RESPAWN_TIME as u64));
    player.respawn_timer.reset();
}

fn respawn_system(
    time: Res<Time>,
    mut commands: Commands,
    mut players: Query<(Entity, &mut Health, &mut Transform, &mut Player)>,
    spawn_points: Query<&Transform, (With<PlayerSpawnPoint>, Without<Player>)>,
) {
    for (entity, mut health, mut transform, mut player) in players.iter_mut() {
        if health.dead() {
            player.respawn_timer.tick(time.delta());

            if player.respawn_timer.finished() {
                if let Some(spawn_transform) = find_random_spawn_point(&spawn_points) {
                    health.set_current(STARTING_HEALTH);
                    *transform = spawn_transform;
                    crate::utils::turn_on(entity, &mut commands);
                }
            }
        }
    }
}

fn scoreboard_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    controller: Res<GameModeController>,
    players: Query<(&Team, &Player)>,
    mut contexts: EguiContexts,
) {
    if keyboard_input.pressed(KeyCode::Tab) {
        let game_mode_name = controller.get_game_mode_name();
        egui::Window::new(format!("Scores - {}", game_mode_name))
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .show(&contexts.ctx_mut(), |ui| match controller.game_mode {
                GameMode::Deathmatch | GameMode::SinglePlayer | GameMode::Coop => {
                    ui.vertical_centered(|ui| {
                        ui.heading("Players");
                        ui.separator();
                        ui.columns(3, |columns| {
                            columns[0].heading("Name");
                            columns[1].heading("Kills");
                            columns[2].heading("Deaths");
                        });
                        ui.separator();
                        for (_, player) in players.iter() {
                            ui.columns(3, |columns| {
                                columns[0].label(player.name.clone());
                                columns[1].label(player.frags.to_string());
                                columns[2].label(player.deaths.to_string());
                            });
                        }
                    });
                }
                GameMode::TeamDeathmatch => {
                    ui.vertical_centered(|ui| {
                        ui.heading("Red Team");
                        ui.separator();
                        ui.columns(4, |columns| {
                            columns[0].heading("Name");
                            columns[1].heading("Kills");
                            columns[2].heading("Deaths");
                        });
                        ui.separator();
                        for (team, player) in players.iter() {
                            if team == &Team::Virus {
                                ui.columns(4, |columns| {
                                    columns[0].label(player.name.clone());
                                    columns[1].label(player.frags.to_string());
                                    columns[2].label(player.deaths.to_string());
                                });
                            }
                        }
                    });
                    ui.vertical_centered(|ui| {
                        ui.heading("Blue Team");
                        ui.separator();
                        ui.columns(4, |columns| {
                            columns[0].heading("Name");
                            columns[1].heading("Kills");
                            columns[2].heading("Deaths");
                        });
                        ui.separator();
                        for (team, player) in players.iter() {
                            if team == &Team::AntiVirus {
                                ui.columns(4, |columns| {
                                    columns[0].label(player.name.clone());
                                    columns[1].label(player.frags.to_string());
                                    columns[2].label(player.deaths.to_string());
                                });
                            }
                        }
                    });
                }
            });
    }
}

fn create_player(
    id: u8,
    team: &Team,
    spawn_event_writer: &mut EventWriter<SpawnEvent>,
    spawn_points: &Query<&Transform, (With<PlayerSpawnPoint>, Without<Player>)>,
) {
    if spawn_points.iter().count() == 0 {
        println!("No spawn points found");
        return;
    }

    if let Some(spawn_point) = find_random_spawn_point(&spawn_points) {
        spawn_event_writer.send(SpawnEvent::Player(SpawnPlayer {
            id,
            health: STARTING_HEALTH,
            name: format!("Player {}", id),
            team: team.clone(),
            position: spawn_point.translation,
        }));
    }
}

fn find_random_spawn_point(
    spawn_points: &Query<&Transform, (With<PlayerSpawnPoint>, Without<Player>)>,
) -> Option<Transform> {
    if spawn_points.iter().count() == 0 {
        return None;
    }

    let spawn_point_index =
        rand::Rng::gen_range(&mut rand::thread_rng(), 0..spawn_points.iter().count());
    spawn_points.iter().nth(spawn_point_index).cloned()
}

// notifies the client that they died
// only ran on a server
fn server_player_dead_sender_system(
    local_player: Res<LocalPlayer>,
    idpooler: Res<IdPooler>,
    players: Query<&Player>,
    mut game_mode_event_reader: EventReader<GameModeEvent>,
    mut server: ResMut<RenetServer>,
) {
    if local_player.has_authority() {
        for event in game_mode_event_reader.read() {
            match event {
                GameModeEvent::OnPlayerDead(entity) => {
                    if let Ok(player) = players.get(*entity) {
                        if let Some(client_id) = idpooler.get_client_id_from_player_id(player.id) {
                            crate::net::rpcs::server_send_respawn_timer(
                                &ClientId::from_raw(*client_id),
                                &mut server,
                                RESPAWN_TIME,
                            );
                        }
                    }
                }
            }
        }
    }
}

pub fn post_build_map_system(
    local_player: Res<LocalPlayer>,
    controller: Res<GameModeController>,
    mut idpooler: ResMut<IdPooler>,
    mut commands: Commands,
    mut event_reader: EventReader<PostBuildMapEvent>,
    mut spawn_event_writer: EventWriter<SpawnEvent>,
    mut map_entities: Query<(Entity, &qevy::components::MapEntityProperties)>,
    mut game_mode_signals: EventWriter<GameModeCommands>,
    mut next_state: ResMut<NextState<AppState>>,
    mut script_container: NonSendMut<ScriptContainer>,
) {
    for _ in event_reader.read() {
        // to set these up, see the .fgd file in the TrenchBroom
        // game folder for Qevy Example also see the readme
        for (entity, props) in map_entities.iter_mut() {
            match props.classname.as_str() {
                "player_spawn_point" => {
                    if local_player.has_authority() {
                        // create a spawn point entity so we can respawn here later
                        commands.spawn((
                            GameplayEntity,
                            PlayerSpawnPoint,
                            TransformBundle {
                                local: props.transform,
                                ..default()
                            },
                        ));
                    }
                }
                "laser_enemy_spawn_point"
                | "plasma_enemy_spawn_point"
                | "fusion_enemy_spawn_point" => {
                    if local_player.has_authority() {
                        let enemy_type = props.classname.as_str();
                        match enemy_type {
                            "plasma_enemy_spawn_point" => {
                                if let Ok(monster_id) = idpooler.assign_and_reserve_monster_id() {
                                    spawn_event_writer.send(SpawnEvent::Monster(SpawnMonster {
                                        id: monster_id,
                                        seed: monster_id,
                                        kind: MonsterKind::GruntPlasma,
                                        translation: props.transform.translation,
                                    }));
                                }
                            }
                            "laser_enemy_spawn_point" => {
                                if let Ok(bot_id) = idpooler.assign_and_reserve_monster_id() {
                                    spawn_event_writer.send(SpawnEvent::Monster(SpawnMonster {
                                        id: bot_id,
                                        seed: bot_id,
                                        kind: MonsterKind::GruntLasers,
                                        translation: props.transform.translation,
                                    }));
                                }
                            }
                            "fusion_enemy_spawn_point" => {
                                if let Ok(bot_id) = idpooler.assign_and_reserve_monster_id() {
                                    spawn_event_writer.send(SpawnEvent::Monster(SpawnMonster {
                                        id: bot_id,
                                        seed: bot_id,
                                        kind: MonsterKind::GruntFusion,
                                        translation: props.transform.translation,
                                    }));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                "light" => {
                    commands.entity(entity).insert(PointLightBundle {
                        transform: props.transform,
                        point_light: PointLight {
                            color: props.get_property_as_color("color", Color::WHITE),
                            radius: props.get_property_as_f32("radius", 0.0),
                            range: props.get_property_as_f32("range", 10.0),
                            intensity: props.get_property_as_f32("intensity", 800.0),
                            shadows_enabled: props.get_property_as_bool("shadows_enabled", false),
                            ..default()
                        },
                        ..default()
                    });
                }
                "directional_light" => {
                    commands.entity(entity).insert(DirectionalLightBundle {
                        transform: props.transform,
                        directional_light: DirectionalLight {
                            color: props.get_property_as_color("color", Color::WHITE),
                            illuminance: props.get_property_as_f32("illuminance", 10000.0),
                            shadows_enabled: props.get_property_as_bool("shadows_enabled", false),
                            ..default()
                        },
                        ..default()
                    });
                }
                "navigation_node" => {
                    if local_player.has_authority() {
                        commands.entity(entity).insert((
                            NavigationNode,
                            TransformBundle {
                                local: props.transform,
                                ..default()
                            },
                        ));
                    }
                }
                "mover" => {
                    let mover_type = props.get_property_as_string("mover_type", &"linear".into());
                    let mut mover_entity = commands.entity(entity);
                    mover_entity.insert((
                        Mover {
                            speed: props.get_property_as_f32("speed", 1.0),
                            destination_translation: props
                                .get_property_as_vec3("translation", Vec3::ZERO),
                            start_translation: Vec3::ZERO,
                        },
                        TransformBundle {
                            local: Transform::from_xyz(0.0, 0.0, 0.0),
                            ..default()
                        },
                    ));

                    if mover_type == "door" {
                        let open_once = props.get_property_as_bool("open_once", false);
                        let open_time = props.get_property_as_i32("open_time", 1000);
                        mover_entity.insert(Door {
                            open_time: std::time::Duration::from_millis(open_time as u64),
                            triggered_time: None,
                            key: props.get_property_as_string("key", &"".into()).into(),
                            open_once: open_once,
                        });
                    }
                }
                "key_red" => {
                    if local_player.has_authority() {
                        if let Ok(pickup_id) = idpooler.assign_and_reserve_pickup_id() {
                            spawn_event_writer.send(SpawnEvent::Pickup(SpawnPickup {
                                id: pickup_id,
                                amount: 0,
                                kind: PickupKind::RedKey,
                                translation: props.transform.translation,
                            }));
                        }
                    }
                }
                "key_blue" => {
                    if local_player.has_authority() {
                        if let Ok(pickup_id) = idpooler.assign_and_reserve_pickup_id() {
                            spawn_event_writer.send(SpawnEvent::Pickup(SpawnPickup {
                                id: pickup_id,
                                amount: 0,
                                kind: PickupKind::BlueKey,
                                translation: props.transform.translation,
                            }));
                        }
                    }
                }
                "key_yellow" => {
                    if local_player.has_authority() {
                        if let Ok(pickup_id) = idpooler.assign_and_reserve_pickup_id() {
                            spawn_event_writer.send(SpawnEvent::Pickup(SpawnPickup {
                                id: pickup_id,
                                amount: 0,
                                kind: PickupKind::YellowKey,
                                translation: props.transform.translation,
                            }));
                        }
                    }
                }
                "key_orange" => {
                    if local_player.has_authority() {
                        if let Ok(pickup_id) = idpooler.assign_and_reserve_pickup_id() {
                            spawn_event_writer.send(SpawnEvent::Pickup(SpawnPickup {
                                id: pickup_id,
                                amount: 0,
                                kind: PickupKind::OrangeKey,
                                translation: props.transform.translation,
                            }));
                        }
                    }
                }
                _ => {}
            }
        }

        next_state.set(AppState::Playing(PlayingSubState::Playing));

        if local_player.has_authority() {
            game_mode_signals.send(GameModeCommands::Start);
        }

        // TODO: trigger this via system
        script_container
            .on_enter_playing_state(controller.map.clone(), local_player.has_authority());
    }
}

pub fn parse_gamemode(input: &str) -> GameMode {
    match input.trim().to_lowercase().as_str() {
        "singleplayer" | "sp" => GameMode::SinglePlayer,
        "cooperative" | "coop" => GameMode::Coop,
        "deathmatch" | "dm" => GameMode::Deathmatch,
        "teamdeathmatch" | "tdm" => GameMode::TeamDeathmatch,
        _ => GameMode::SinglePlayer,
    }
}
