use bevy::color::palettes::basic::{BLUE, YELLOW};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use lightyear::client::prediction::Predicted;
use lightyear::prelude::client::VisualInterpolateStatus;
use lightyear::prelude::Replicating;
use shared::player::Player;
use shared::prelude::{PlayerInput, RayCastBullet};
use shared::projectiles::Projectile;

pub(crate) struct ProjectilesPlugin;

impl Plugin for ProjectilesPlugin {
    fn build(&self, app: &mut App) {

        // SYSTEMS
        app.add_observer(spawn_visuals);
        app.add_systems(PostUpdate, show_raycast_gizmos);
    }
}


/// When an instant projectile is spawned, add Gizmo visuals
fn show_raycast_gizmos(
    mut event_reader: EventReader<RayCastBullet>,
    mut gizmos: Gizmos,
) {
    for event in event_reader.read() {
        info!(?event, "Shooting ray!");
        // gizmos.line(
        //     event.source,
        //     event.source + event.direction.as_vec3() * 1000.0,
        //     YELLOW,
        // );
        // gizmos.line(
        //     event.source,
        //     event.source - event.direction.as_vec3() * 1000.0,
        //     YELLOW,
        // );
        gizmos.ray(
            event.source,
            event.direction.as_vec3(),
            YELLOW,
        );
    }
}

/// When a projectile is spawn, add visuals to it
fn spawn_visuals(
    trigger: Trigger<OnAdd, Projectile>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.entity(trigger.entity()).with_child(
        (
            Mesh3d(meshes.add(Mesh::from(Sphere {
                // TODO: must match the collider size
                radius: 0.05,
                ..default()
            }))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: BLUE.into(),
                ..Default::default()
            })),
            VisualInterpolateStatus::<Transform>::default(),
        )
    );
}