use avian3d::prelude::{Collider, LinearVelocity, PhysicsSet, Position, Rotation, SpatialQuery, SpatialQueryFilter};
use bevy::color::palettes::basic::{BLUE, YELLOW};
use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use bevy::utils::Duration;
use leafwing_input_manager::prelude::ActionState;
use lightyear::client::prediction::Predicted;
use lightyear::prelude::client::{Interpolated, VisualInterpolateStatus};
use lightyear::prelude::Replicating;
use shared::bot::Bot;
use shared::physics::GameLayer;
use shared::player::Player;
use shared::prelude::{DespawnAfter, PlayerInput, LinearProjectile};
use shared::weapons::Projectile;

pub(crate) struct ProjectilesPlugin;

impl Plugin for ProjectilesPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_projectile_visuals);
    }
}

#[derive(Component, Debug)]
struct VisualRay {
    source: Vec3,
    target: Vec3,
}

/// Display the gizmos for the raycast bullets
fn show_raycast_gizmos(
    mut gizmos: Gizmos,
    query: Query<&VisualRay>,
) {
    query.iter().for_each(|event| {
        gizmos.line(
            event.source,
            event.target,
            YELLOW,
        );
    });
}

/// When a projectile is spawn, add visuals to it
fn spawn_projectile_visuals(
    trigger: Trigger<OnAdd, Projectile>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.entity(trigger.entity()).insert(
        (
            Visibility::default(),
            Mesh3d(meshes.add(Mesh::from(Sphere {
                // TODO: must match the collider size
                //      @todo-brian-reply: nah, its common for games to have a visual size that 
                //      doesn't match the collider size, infact we should probably stick to 
                //      simple Point based collision (ray cast) for projectiles, unless 
                //      they are much larger than a typiical projectile, since there's going 
                //      to be a lot flying at once.
                radius: 0.05,
                ..default()
            }))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: BLUE.into(),
                ..Default::default()
            })),
            //VisualInterpolateStatus::<Transform>::default(),
            NotShadowCaster,
        )
    );
}
