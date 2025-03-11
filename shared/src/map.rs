use avian3d::collision::CollisionLayers;
use avian3d::prelude::{Friction, RigidBody};
use bevy::prelude::*;
use qevy::PostBuildMapEvent;
use serde::{Deserialize, Serialize};
use crate::physics::GameLayer;
use crate::states::AppState;

#[derive(Default)]
pub struct MapPlugin {
    pub(crate) headless: bool,
}

#[derive(Resource, Debug)]
pub struct GameModeController {
    pub map: String,
    pub game_mode: GameMode,
}

impl Default for GameModeController {
    fn default() -> Self {
        Self {
            map: "m4".to_string(),
            game_mode: GameMode::SinglePlayer,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum GameMode {
    #[default]
    SinglePlayer,
    Coop,
    Deathmatch,
    TeamDeathmatch,
}

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        // PLUGINS
        app.add_plugins(qevy::MapAssetLoaderPlugin {
            headless: self.headless,
            ..default()
        });

        // RESOURCES
        app.init_resource::<GameModeController>();

        // STATES
        app.insert_state(AppState::None);

        // SYSTEMS
        app.add_systems(
            // TODO: we start at LoadingMap to bypass the menu
            // OnEnter(AppState::LoadingMap),
            Startup,
            (clear_map_system, load_map_system).chain(),
        );
        app.add_systems(
            Update, add_map_colliders
        );
    }
}

fn clear_map_system(
    mut commands: Commands,
    map: Query<Entity, With<qevy::components::Map>>,
) {
    // despawn current map
    for e in map.iter() {
        if let Some(e) = commands.get_entity(e) {
            e.despawn_recursive();
        }
    }
}

fn load_map_system(
    asset_server: Res<AssetServer>,
    gamemode_controller: ResMut<GameModeController>,
    mut commands: Commands,
) {
    info!("Loading map: {}", gamemode_controller.map);
    info!("Setting gamemode: {:?}", gamemode_controller.game_mode);

    commands.spawn((
        qevy::components::Map {
            asset: asset_server.load(format!("{}.map", gamemode_controller.map)), // map must be under `assets` folder
            ..default()
        },
        Name::from("Map")
    ));
}

fn add_map_colliders(
    mut commands: Commands,
    mut events: EventReader<PostBuildMapEvent>,
    query: Query<&Children>
) {
    for event in events.read() {
        query.iter_descendants(event.map).for_each(|child| {
             if let Some(mut child_commands) = commands.get_entity(child) {
                child_commands.insert((
                    Name::from("MapCollider"),
                    CollisionLayers::new([GameLayer::Wall], [GameLayer::Player, GameLayer::Projectile]),
                    // map objects already have RigidBody::Static
                ));
            }
        });
    }
}
