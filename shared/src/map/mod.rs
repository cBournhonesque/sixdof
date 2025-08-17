mod pathnodes;

use avian3d::prelude::CollisionLayers;
use bevy::ecs::component::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy_trenchbroom::prelude::*;
use pathnodes::{PathfindingGraph};

use crate::physics::GameLayer;

#[solid_class]
#[component(on_add = Self::on_add)]
pub struct Worldspawn;

impl Worldspawn {
    fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        world.commands().entity(ctx.entity).insert(CollisionLayers {
                memberships: [GameLayer::Wall].into(),
                ..default()
            });
    }
}

#[derive(Default)]
pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "render")]
        app.insert_resource(AmbientLight::NONE);

        app.insert_resource(PathfindingGraph::default());

        let config = TrenchBroomConfig::new("sixdof")
            .default_solid_spawn_hooks(|| SpawnHooks::new().smooth_by_default_angle().convex_collider());
        app.add_plugins(TrenchBroomPlugins(config).build());
        app.override_class::<Worldspawn>();
        app.add_systems(Startup, load_map_system);
        //app.add_systems(Update, generate_pathfinding_nodes_system);
        //app.add_systems(Update, draw_pathfinding_graph_system);
    }
}

fn load_map_system(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    commands.spawn(SceneRoot(asset_server.load("maps/m4.map#Scene")));
}
