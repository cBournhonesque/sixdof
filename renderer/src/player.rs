use bevy::color::palettes::basic::BLUE;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::prelude::*;
use lightyear::prelude::client::{Confirmed, Predicted, VisualInterpolateStatus};
use lightyear::shared::replication::components::Controlled;
use shared::player::Player;

/// Responsible for render-related systems for Players
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        // make sure that we run after Prediction/Interpolation components have been added
        app.add_systems(Update, spawn_visuals);
    }
}

// NOTE: we cannot use observers because we add Player before adding Confirmed/Predicted
//  or should we do Trigger<OnAdd, (Predicted, Interpolated)>?
/// Add meshes/visuals for spawned players
fn spawn_visuals(
    // we do not want to add visuals to confirmed entities on the client
    query: Query<(Entity, Has<Controlled>, Has<Predicted>), (Without<Confirmed>, Added<Player>)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    // mut atomized_materials: ResMut<Assets<AtomizedMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    query.iter().for_each(|(parent, is_controlled, is_predicted)| {
        // add visibility
        commands.entity(parent).insert(Visibility::default());

        // TODO: don't do this in host-server mode!
        // add visual interpolation on the predicted entity
        if is_predicted {
            commands.entity(parent).insert(VisualInterpolateStatus::<Transform>::default());
        }
        // add lights
        // TODO: why do we need it as a child? so we can specify a direction (via Transform) to the light?
        commands.entity(parent).with_children(|parent| {
            let headlamp_1_pos = Vec3::new(0.45, 0.0, 0.0);
            let headlamp_2_pos = Vec3::new(-0.45, 0.0, 0.0);
            for headlamp_index in 0..2 {
                let headlamp_pos = if headlamp_index == 0 {
                    headlamp_1_pos
                } else {
                    headlamp_2_pos
                };
                parent.spawn((
                    SpotLight {
                        color: Color::srgb(1.0, 0.95, 0.9),
                        outer_angle: 0.75,
                        inner_angle: 0.1,
                        shadows_enabled: true,
                        ..default()
                    },
                    Transform::from_translation(headlamp_pos)
                        .looking_at(Vec3::new(0.0, 0.0, -1.0), Vec3::Y),
                ));
            }
        });

        if !is_controlled {
            // add a mesh for other players
            commands.entity(parent).with_child((
                Mesh3d(meshes.add(Mesh::from(Sphere {
                    radius: 0.5,
                    ..default()
                }))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: BLUE.into(),
                    ..Default::default()
                })),
            ));
        } else {
            // spawn a camera for 1-st person view
            commands.entity(parent).insert((
                Camera3d::default(),
                Camera {
                    hdr: true,
                    ..default()
                },
                Projection::Perspective(PerspectiveProjection {
                    fov: 90.0_f32.to_radians(),
                    ..default()
                }),
            ));
        }
    });
}
