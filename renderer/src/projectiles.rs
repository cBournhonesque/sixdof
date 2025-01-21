use bevy::color::palettes::basic::{BLUE, YELLOW};
use bevy::prelude::*;
use bevy::utils::Duration;
use leafwing_input_manager::prelude::ActionState;
use lightyear::client::prediction::Predicted;
use lightyear::prelude::client::VisualInterpolateStatus;
use lightyear::prelude::Replicating;
use shared::player::Player;
use shared::prelude::{DespawnAfter, PlayerInput, RayCastBullet};
use shared::projectiles::Projectile;

pub(crate) struct ProjectilesPlugin;

impl Plugin for ProjectilesPlugin {
    fn build(&self, app: &mut App) {
        // SYSTEMS
        app.add_observer(spawn_projectile_visuals);
        app.add_systems(Last, (spawn_raycast_gizmos, show_raycast_gizmos).chain());
    }
}

#[derive(Component, Debug)]
struct VisualRay {
    source: Vec3,
    target: Vec3,
}

// TODO: on the server, we should cast the ray directly from
/// When an instant projectile is spawned, add Gizmo visuals
fn spawn_raycast_gizmos(
    mut commands: Commands,
    // mut gizmos: Gizmos,
    mut event_reader: EventReader<RayCastBullet>,
    // NOTE: we cannot do the raycast directly forward, because then the raycast will be invisible
    // since the user is directly looking at that direction
    // instead we can offset the raycast
    camera: Query<(&Camera, &GlobalTransform)>,
) {
    for event in event_reader.read() {
        let target = event.source + event.direction.as_vec3() * 1000.0;
        // if the shot comes from the client, we need to angle the visuals otherwise the ray will be invisible
        // since the camera is looking straight into it
        let source = camera.get(event.shooter)
            .map_or(event.source, |(camera, transform)| {
                let res = camera.ndc_to_world(transform, Vec3::new(0.0, 0.5, 0.0)).unwrap_or_default();
                info!(?res, "converting from NDC to world");
                res
        });
        info!(?target, ?source, "Spawning ray visuals!");
        // gizmos.ray(
        //     event.source,
        //     event.direction.as_vec3() * 1000.0,
        //     YELLOW,
        // );
        let visual_raycast = VisualRay {
            source,
            target,
        };
        commands.spawn((
            DespawnAfter(Timer::new(Duration::from_millis(100), TimerMode::Once)),
            visual_raycast,
        ));
    }
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