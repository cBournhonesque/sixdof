mod pathnodes;

use avian3d::collision::CollisionLayers;
use avian3d::prelude::Collider;
use bevy::prelude::*;
use bevy_trenchbroom::prelude::*;
use pathnodes::{draw_pathfinding_graph_system, generate_pathfinding_nodes_system, PathfindingGraph};

use crate::physics::GameLayer;

#[derive(SolidClass, Component, Reflect)]
#[no_register]
#[reflect(Component)]
#[geometry(GeometryProvider::new().smooth_by_default_angle().render().convex_collider())]
pub struct Worldspawn;

#[derive(Default)]
pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AmbientLight::NONE);
        app.insert_resource(PathfindingGraph::default());
        app.add_plugins(TrenchBroomPlugin(
            TrenchBroomConfig::new("sixdof")
                .register_class::<Worldspawn>()
        ));
        app.add_systems(Startup, load_map_system);
        app.add_systems(Update, add_map_colliders);
        app.add_systems(Update, generate_pathfinding_nodes_system);
        app.add_systems(Update, draw_pathfinding_graph_system);
    }
}

fn load_map_system(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    commands.spawn(SceneRoot(asset_server.load("maps/m4.map#Scene")));
}

fn add_map_colliders(
    mut commands: Commands,
    worldspawn_colliders: Query<Entity, (With<Worldspawn>, Changed<Collider>)>,
) {
    for e in worldspawn_colliders.iter() {
        commands.entity(e).insert(CollisionLayers {
            memberships: [GameLayer::Wall].into(),
            ..default()
        });
    }
}
