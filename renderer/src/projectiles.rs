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


// TODO: maybe this is not ne
/// When an instant projectile is spawned, add Gizmo visuals
fn spawn_raycast_gizmos(
    mut commands: Commands,
    mut gizmos: Gizmos,
    mut event_reader: EventReader<RayCastBullet>,
) {
    for event in event_reader.read() {
        info!(?event, "Shooting ray!");
        // gizmos.ray(
        //     event.source,
        //     event.direction.as_vec3() * 1000.0,
        //     YELLOW,
        // );
        // NOTE: we cannot do the raycast directly forward, because then the raycast will be invisible
        // since the user is directly looking at that direction
        // instead we can offset the raycast
        // let end = event.source + event.direction * 1000.0;
        // let source = event.source;
        let visual_raycast = RayCastBullet {
            source: event.source,
            direction: event.direction,
            ..default()
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
    query: Query<&RayCastBullet>,
) {
    query.iter().for_each(|event| {
        gizmos.ray(
            event.source,
            event.direction.as_vec3() * 1000.0,
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