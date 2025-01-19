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
        app.add_observer(spawn_visuals);
        app.add_systems(Update, spawn_raycast_gizmos);
        app.add_systems(PostUpdate, show_raycast_gizmos);
    }
}


/// When an instant projectile is spawned, add Gizmo visuals
fn spawn_raycast_gizmos(
    mut commands: Commands,
    mut event_reader: EventReader<RayCastBullet>,
) {
    for event in event_reader.read() {
        info!(?event, "Shooting ray!");
        commands.spawn((
            DespawnAfter(Timer::new(Duration::from_millis(50), TimerMode::Once)),
            event.clone(),
        ));

    }
}

/// Display the gizmos for the raycast bullets
fn show_raycast_gizmos(
    mut gizmos: Gizmos,
    query: Query<&RayCastBullet>,
) {
    query.iter().for_each(|event| {
        info!(?event, "PrintRay");
        gizmos.ray(
            event.source,
            event.direction.as_vec3(),
            YELLOW,
        );
    });
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