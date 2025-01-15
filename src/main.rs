use bevy::scene::ScenePlugin;
use bevy::window::{WindowMode, WindowResolution};
use bevy::{app::ScheduleRunnerPlugin, prelude::*, utils::Duration};
use bevy::{
    diagnostic::LogDiagnosticsPlugin, log::LogPlugin, window::PresentMode, winit::WinitSettings,
};
use bevy_common_assets::ron::RonAssetPlugin;
use bevy_contact_projective_decals::DecalPlugin;
use bevy_egui::EguiPlugin;
use bevy_flamethrower::prelude::*;
use bevy_fmod::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::ClientId;
use bevy_renet::{
    renet::transport::{
        ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport, ServerAuthentication,
        ServerConfig,
    },
    transport::{NetcodeClientPlugin, NetcodeServerPlugin},
};
use bevy_renet::{
    renet::{RenetClient, RenetServer},
    RenetClientPlugin, RenetServerPlugin,
};
use clap::Parser;
use gamemode::*;
use hud::Hud;
use ids::IdPooler;
use menu::MenuPlugin;
use player::LocalPlayer;
use scripting::ScriptPlugin;
use std::any::Any;
use std::fs;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::SystemTime,
};

mod components;
mod config;
mod console;
mod fx;
mod fx2;
mod gamemode;
mod gameplay;
mod hud;
mod ids;
mod menu;
mod monsters;
mod net;
mod physics;
mod pickups;
mod player;
mod scripting;
mod sfx;
mod snapshot;
mod spawn;
mod utils;
mod weapons;

pub type TClientId = u64;
pub type TPlayerNetId = u8;
pub type TBotNetId = u8;
pub type TProjectileNetId = u16;
pub type TPickupNetId = u16;

pub const TICKRATE: u64 = 64;
pub const UNNAMED_PLAYER: &'static str = "Unnamed Player";
pub const DEFAULT_PORT: u16 = 7777;
pub const PROTOCOL_ID: u64 = 0;

#[derive(Resource, Clone)]
pub struct ServerSettings {
    port: u16,
    game_mode: GameMode,
    map: String,
}

#[derive(Resource, Clone)]
pub struct ClientSettings {
    ip: IpAddr,
    port: u16,
    player_name: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States)]
pub enum PlayingSubState {
    Playing,
    Menu,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States)]
pub enum WorldState {
    SinglePlayer,
    Client,
    ListenServer,
    DedicatedServer,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States)]
pub enum AppState {
    None,
    SplashScreen,
    TitleScreen,
    LoadingMap,
    Playing(PlayingSubState),
    ConnectingToServer,
}

#[derive(Event)]
pub enum AppEvent {
    GotoTitleScreen,
    StartSinglePlayer,
    HostServer(ServerSettings),
    JoinServer(ClientSettings),
}

#[derive(Parser, PartialEq, Resource, Clone)]
pub enum Cli {
    SinglePlayer,
    DedicatedServer {
        #[arg(short, long, default_value_t = DEFAULT_PORT)]
        port: u16,

        #[arg(short, long, default_value_t = String::from("coop"))]
        game_mode: String,

        #[arg(short, long, default_value_t = String::from("m4"))]
        map: String,
    },
    Server {
        #[arg(short, long, default_value_t = DEFAULT_PORT)]
        port: u16,

        #[arg(long, default_value_t = String::from(UNNAMED_PLAYER))]
        player_name: String,

        #[arg(short, long, default_value_t = String::from("coop"))]
        game_mode: String,

        #[arg(short, long, default_value_t = String::from("m4"))]
        map: String,
    },
    Client {
        #[arg(short, long, default_value_t = Ipv4Addr::LOCALHOST.into())]
        ip: IpAddr,

        #[arg(short, long, default_value_t = DEFAULT_PORT)]
        port: u16,

        #[arg(long, default_value_t = String::from(UNNAMED_PLAYER))]
        player_name: String,
    },
}

fn main() {
    // enable backtracing
    std::env::set_var("RUST_BACKTRACE", "full");
    let mut app = App::new();

    let dedicated_server = if let Ok(Cli::DedicatedServer { .. }) = Cli::try_parse() {
        true
    } else {
        false
    };

    match Cli::try_parse() {
        Ok(Cli::SinglePlayer) => {
            app.insert_state(WorldState::SinglePlayer);
        }
        Ok(Cli::DedicatedServer {
            port,
            game_mode,
            map,
        }) => {
            app.insert_state(WorldState::DedicatedServer);
            app.insert_resource(ServerSettings {
                port,
                game_mode: parse_gamemode(&game_mode),
                map,
            });
        }
        Ok(Cli::Server {
            port,
            player_name,
            game_mode,
            map,
        }) => {
            app.insert_state(WorldState::ListenServer);
            app.insert_resource(ServerSettings {
                port,
                game_mode: parse_gamemode(&game_mode),
                map,
            });
        }
        Ok(Cli::Client {
            ip,
            port,
            player_name,
        }) => {
            app.insert_state(WorldState::Client);
            app.insert_resource(ClientSettings {
                ip,
                port,
                player_name,
            });
        }
        Err(e) => {
            println!(
                "No arguments provided!\nDefaulting to single-player\n: {}",
                e
            );
            app.insert_state(WorldState::SinglePlayer);
        }
    }

    setup_app(&mut app, dedicated_server);
    app.run();
}

fn setup_app(mut app: &mut App, dedicated_server: bool) {
    app.insert_resource(ids::IdPooler::new(true));
    app.insert_resource(player::LocalPlayer {
        client_id: ClientId::from_raw(0),
        ..default()
    });

    //---------------------------------------------------------------------
    // Standard App Plugins
    // A standard app is every world state except for dedicated server
    //---------------------------------------------------------------------
    if !dedicated_server {
        app.add_plugins((
            DefaultPlugins
                .set(LogPlugin {
                    filter: get_log_filter().to_string(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Quantsum".to_string(),
                        mode: WindowMode::BorderlessFullscreen,
                        resizable: true,
                        present_mode: PresentMode::AutoVsync,
                        resolution: WindowResolution::new(1920.0, 1080.0)
                            .with_scale_factor_override(1.0),
                        ..default()
                    }),
                    ..default()
                }),
            EguiPlugin,
            hud::HudPlugin,
            console::ConsolePlugin,
            ParticlePlugin {
                shader_list: vec![
                    ParticleShader::Textured("shaders/particles/instanced_smoke.wgsl"),
                    ParticleShader::Textured("shaders/particles/instanced_explosion.wgsl"),
                    ParticleShader::NonTextured("shaders/particles/instanced_spark.wgsl"),
                ],
            },
            RenetClientPlugin,
            RenetServerPlugin,
            NetcodeClientPlugin,
            NetcodeServerPlugin,
            MenuPlugin,
            qevy::MapAssetLoaderPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ));

        app.add_plugins((
            fx::FxPlugin,
            fx2::FxPlugin,
            sfx::SfxPlugin,
            DecalPlugin,
            FmodPlugin {
                audio_banks_paths: &[
                    "./assets/audio/quantsum/Build/Desktop/Master.bank",
                    "./assets/audio/quantsum/Build/Desktop/Master.strings.bank",
                    "./assets/audio/quantsum/Build/Desktop/Weapons.bank",
                    "./assets/audio/quantsum/Build/Desktop/Bots.bank",
                ],
            },
        ));

        app.insert_resource(WinitSettings {
            focused_mode: bevy::winit::UpdateMode::Continuous,
            unfocused_mode: bevy::winit::UpdateMode::Continuous,
        });

        app.add_systems(FixedPreUpdate, app_event_system);

    //----------------------------------------------------------------------
    // Dedicated Server Plugins
    // A dedicated server is a headless server that does not render anything
    //----------------------------------------------------------------------
    } else {
        app.insert_resource(ids::IdPooler::new(false));
        app.insert_resource(player::LocalPlayer {
            client_id: ClientId::from_raw(0),
            ..default()
        });
        app.add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 200.0,
            ))),
            LogPlugin {
                filter: get_log_filter().to_string(),
                ..default()
            },
            AssetPlugin::default(),
            ScenePlugin::default(),
            RenetServerPlugin,
            NetcodeServerPlugin,
            qevy::MapAssetLoaderPlugin { headless: true },
        ));

        // events & resources that need to exist but otherwise ignored
        app.init_resource::<Assets<Mesh>>();
        app.add_event::<fx::Explode>();
        app.add_event::<fx::DecalEvent>();
        app.add_event::<fx2::ParticleEvent>();
        app.add_event::<sfx::AudioEvent>();
    }

    //-----------------------------------------------------------------
    // Shared Plugins
    //-----------------------------------------------------------------
    app.add_plugins((
        net::NetPlugin,
        GameModePlugin::default(),
        RapierPhysicsPlugin::<NoUserData>::default(),
        RonAssetPlugin::<weapons::WeaponConfig>::new(&["weapon.ron"]),
        ScriptPlugin,
    ));
    //-----------------------------------------------------------------

    app.insert_state(AppState::None);
    app.add_systems(Startup, boot_system);
    app.add_systems(Startup, weapons::load_weapon_assets_system);

    // systems shared by both dedicated server and standard app kinds
    let shared_update_systems = (
        monsters::Monster::systems(),
        spawn::spawn_health_system,
        spawn::spawn_event_system,
        physics::movement_system,
        weapons::weapons_system,
        weapons::Projectile::update_systems(),
    )
        .chain()
        .run_if(in_playing_state)
        .run_if(resource_exists::<LocalPlayer>);

    if dedicated_server {
        app.add_systems(Update, shared_update_systems);
    } else {
        app.add_systems(OnEnter(AppState::TitleScreen), clear_map_system);
        app.add_systems(
            Update,
            (
                player::player_input_system,
                player::player_input_reader_system,
                shared_update_systems,
                spawn::spawn_visuals_system,
                player::local_player_visuals_system,
                player::player_camera_system,
                monsters::visuals_system,
                weapons::ProjectileFx::systems(),
            )
                .chain()
                .run_if(in_playing_state)
                .run_if(resource_exists::<LocalPlayer>),
        );
    }

    app.add_systems(
        OnEnter(AppState::LoadingMap),
        (clear_map_system, load_map_system).chain(),
    );
    app.add_systems(OnExit(AppState::LoadingMap), finish_load_map_system);

    app.add_systems(
        FixedUpdate,
        (
            qevy::gameplay_systems::rapier_trigger_system,
            monsters::bot_fire_system,
            pickups::pickup_system,
            gameplay::door_system,
            snapshot::snapshot_system,
            weapons::Projectile::fixed_update_systems(),
        )
            .chain()
            .run_if(in_playing_state)
            .run_if(resource_exists::<LocalPlayer>),
    );

    setup_shared_resources(&mut app, dedicated_server);
    setup_shared_events(&mut app);
}

fn setup_shared_resources(app: &mut App, dedicated_server: bool) {
    app.insert_resource(AmbientLight {
        color: Color::rgb(1.0, 1.0, 1.0),
        brightness: 0.04,
        ..default()
    })
    .insert_resource(Msaa::Sample2)
    .insert_resource(Time::<Fixed>::from_hz(TICKRATE as f64))
    .insert_resource(config::Config::default())
    .insert_resource(net::input::SavedInputs::default())
    .insert_resource(snapshot::history::SnapshotHistory::default())
    .insert_resource(snapshot::history::SnapshotInterpolation::default())
    .insert_resource(weapons::WeaponAssetHandles::default())
    .insert_resource(LocalPlayer {
        client_id: ClientId::from_raw(0),
        ..default()
    })
    .insert_resource(IdPooler::new(!dedicated_server));
}

fn setup_shared_events(app: &mut App) {
    app.add_event::<components::DoorKeyPickupEvent>()
        .add_event::<components::DamageEvent>()
        .add_event::<weapons::ShotgunFireEvent>()
        .add_event::<net::input::NetPlayerInputEvent>()
        .add_event::<net::server::NetPlayerFiredEvent>()
        .add_event::<spawn::SpawnEvent>()
        .add_event::<spawn::SpawnVisualsEvent>()
        .add_event::<weapons::SpawnProjectileEvent>()
        .add_event::<weapons::DespawnProjectileEvent>()
        .add_event::<weapons::SpawnProjectileVisualsEvent>()
        .add_event::<AppEvent>();
}

fn boot_system(
    mut commands: Commands,
    mut config: ResMut<config::Config>,
    world_state: Res<State<WorldState>>,
    server_settings: Option<Res<ServerSettings>>,
    client_settings: Option<Res<ClientSettings>>,
    mut gamemode_controller: ResMut<GameModeController>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    match &world_state.get() {
        WorldState::SinglePlayer => {
            next_app_state.set(AppState::TitleScreen);
        }
        WorldState::Client => {
            if let Some(client_settings) = client_settings {
                start_client(&client_settings, &mut commands, &mut next_app_state);
            }
        }
        WorldState::ListenServer => {
            if let Some(server_settings) = server_settings {
                start_server(
                    &mut gamemode_controller,
                    &mut commands,
                    &mut next_app_state,
                    &server_settings,
                );
            }
        }
        WorldState::DedicatedServer => {
            if let Some(server_settings) = server_settings {
                start_server(
                    &mut gamemode_controller,
                    &mut commands,
                    &mut next_app_state,
                    &server_settings,
                );
            }
        }
    }

    // TODO: move this to a plugin
    let mut config_object = config::Config::default();

    match fs::read_to_string("config.toml") {
        Ok(config) => {
            // load the config file
            if let Ok(config) = toml::from_str::<config::Config>(&config.as_ref()) {
                config_object = config;
            } else if let Err(e) = toml::from_str::<config::Config>(&config.as_ref()) {
                println!("Failed to parse config file: {}", e);
            }
        }
        Err(_) => {
            // create the config file
            if let Ok(config) = toml::to_string(&config_object) {
                match fs::write("config.toml", config) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Failed to write config file: {}", e);
                    }
                }
            }
        }
    };

    // update the config
    *config = config_object;
}

fn app_event_system(
    mut commands: Commands,
    mut app_events: EventReader<AppEvent>,
    mut gamemode_controller: ResMut<GameModeController>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for event in app_events.read() {
        commands.remove_resource::<RenetClient>();
        commands.remove_resource::<NetcodeClientTransport>();
        commands.remove_resource::<RenetServer>();
        commands.remove_resource::<NetcodeServerTransport>();
        commands.remove_resource::<ClientSettings>();
        commands.remove_resource::<ServerSettings>();

        match event {
            AppEvent::GotoTitleScreen => {
                next_state.set(AppState::TitleScreen);
            }
            AppEvent::StartSinglePlayer => {
                load_map(
                    "m4",
                    &GameMode::SinglePlayer,
                    &mut gamemode_controller,
                    &mut next_state,
                );
            }
            AppEvent::HostServer(server_settings) => {
                start_server(
                    &mut gamemode_controller,
                    &mut commands,
                    &mut next_state,
                    server_settings,
                );
            }
            AppEvent::JoinServer(client) => {
                start_client(client, &mut commands, &mut next_state);
            }
        }

        // only run the loop once per frame
        return;
    }
}

fn start_client(
    client_settings: &ClientSettings,
    commands: &mut Commands,
    next_state: &mut NextState<AppState>,
) {
    let client_id = utils::timestamp_millis_since_epoch();

    let authentication = ClientAuthentication::Unsecure {
        server_addr: SocketAddr::new(client_settings.ip, client_settings.port),
        client_id: client_id,
        user_data: None,
        protocol_id: PROTOCOL_ID,
    };

    let socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)).unwrap();

    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    if let Ok(transport) = NetcodeClientTransport::new(current_time, authentication, socket) {
        commands.insert_resource(client_settings.clone());
        commands.insert_resource(ids::IdPooler::new(false));
        commands.insert_resource(player::LocalPlayer {
            client_id: ClientId::from_raw(client_id),
            ..default()
        });
        commands.insert_resource(RenetClient::new(crate::net::connection_config()));
        commands.insert_resource(transport);

        next_state.set(AppState::ConnectingToServer);
    }
}

fn start_server(
    gamemode_controller: &mut ResMut<GameModeController>,
    commands: &mut Commands,
    next_app_state: &mut ResMut<NextState<AppState>>,
    server_settings: &ServerSettings,
) {
    let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), server_settings.port);
    if let Ok(socket) = UdpSocket::bind(server_addr) {
        let server_config = ServerConfig {
            current_time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(),
            max_clients: 64,
            protocol_id: PROTOCOL_ID,
            public_addresses: vec![server_addr],
            authentication: ServerAuthentication::Unsecure,
        };

        if let Ok(transport) = NetcodeServerTransport::new(server_config, socket) {
            commands.insert_resource(server_settings.clone());
            commands.insert_resource(RenetServer::new(crate::net::connection_config()));
            commands.insert_resource(transport);

            load_map(
                &server_settings.map,
                &server_settings.game_mode,
                gamemode_controller,
                next_app_state,
            );
        }
    }
}

fn clear_map_system(
    mut commands: Commands,
    mut snapshot_history: ResMut<snapshot::history::SnapshotHistory>,
    map: Query<Entity, With<qevy::components::Map>>,
    gameplay_entities: Query<Entity, With<crate::components::GameplayEntity>>,
    hud_entities: Query<Entity, With<Hud>>,
) {
    snapshot_history.clear();

    // despawn the current hud
    for e in hud_entities.iter() {
        if let Some(e) = commands.get_entity(e) {
            e.despawn_recursive();
        }
    }

    // despawn current map
    for e in map.iter() {
        if let Some(e) = commands.get_entity(e) {
            e.despawn_recursive();
        }
    }

    // despawn all gameplay entities
    for e in gameplay_entities.iter() {
        if let Some(e) = commands.get_entity(e) {
            e.despawn_recursive();
        }
    }
}

pub fn load_map(
    map: &str,
    gamemode: &GameMode,
    gamemode_controller: &mut ResMut<GameModeController>,
    next_app_state: &mut ResMut<NextState<AppState>>,
) {
    gamemode_controller.map = map.to_string();
    gamemode_controller.game_mode = gamemode.clone();
    next_app_state.set(AppState::LoadingMap);
}

fn load_map_system(
    asset_server: Res<AssetServer>,
    gamemode_controller: ResMut<GameModeController>,
    server: Option<ResMut<RenetServer>>,
    mut commands: Commands,
) {
    info!("Loading map: {}", gamemode_controller.map);
    info!("Setting gamemode: {:?}", gamemode_controller.game_mode);

    commands.spawn(qevy::components::MapBundle {
        map: qevy::components::Map {
            asset: asset_server.load(format!("{}.map", gamemode_controller.map)), // map must be under `assets` folder
            ..default()
        },
        ..default()
    });

    if let Some(mut server) = server {
        crate::net::rpcs::server_send_map_to_all(
            &mut server,
            &gamemode_controller.map,
            gamemode_controller.game_mode.clone(),
        );
    }
}

fn finish_load_map_system() {}

fn get_log_filter() -> &'static str {
    "shambler=error"
}

pub fn has_authority(local_player: Option<Res<LocalPlayer>>) -> bool {
    if let Some(local_player) = local_player {
        return local_player.has_authority();
    }
    false
}

pub fn in_playing_state(app_state: Res<State<AppState>>) -> bool {
    app_state.get() == &AppState::Playing(PlayingSubState::Playing)
        || app_state.get() == &AppState::Playing(PlayingSubState::Menu)
}

pub fn is_dedicated_server(world_state: Res<State<WorldState>>) -> bool {
    world_state.get() == &WorldState::DedicatedServer
}

pub fn is_paused(app_state: Res<State<AppState>>) -> bool {
    app_state.get() == &AppState::Playing(PlayingSubState::Menu)
}

pub fn is_loading_map(app_state: Res<State<AppState>>) -> bool {
    app_state.get() == &AppState::LoadingMap
}
