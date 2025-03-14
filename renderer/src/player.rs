use avian3d::prelude::{AngularVelocity, LinearVelocity};
use bevy::core_pipeline::bloom::Bloom;
use bevy::pbr::NotShadowCaster;
use sfx::prelude::SfxListener;
use bevy::color::palettes::basic::BLUE;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use leafwing_input_manager::prelude::ActionState;
use lightyear::prelude::client::{Confirmed, Predicted, PredictionSet, VisualInterpolateStatus};
use lightyear::shared::replication::components::Controlled;
use shared::player::Player;
use shared::prelude::PlayerInput;
use shared::weapons::WeaponsData;
use crate::hud::spawn_3d_hud;
use crate::VisibleFilter;

/// Responsible for render-related systems for Players
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        // cannot use an observer right now because we are not on lightyear-main where all components are synced at the same time
        // app.add_observer(spawn_visuals);

        app.add_systems(PreUpdate, spawn_visuals.after(PredictionSet::Sync).run_if(resource_exists::<WeaponsData>));
        app.add_systems(Update, (
            toggle_mouse_pointer_system,
        ));
    }
}

fn toggle_mouse_pointer_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let mut window = windows.single_mut();
    if keyboard_input.just_pressed(KeyCode::Tab) {
        toggle_mouse_pointer(&mut window);
    }
}

fn toggle_mouse_pointer(
    window: &mut Window,
) {
    if window.cursor_options.visible {
        mouse_pointer_off(window);
    } else {
        mouse_pointer_on(window);
    }
}

fn mouse_pointer_off(
    window: &mut Window,
) {
    window.cursor_options = CursorOptions {
        visible: false,
        grab_mode: CursorGrabMode::Confined, // DO NOT USE LOCKED! For some reason it causes jittering. Confined is fine.
        ..default()
    };
}

fn mouse_pointer_on(
    window: &mut Window,
) {
    window.cursor_options = CursorOptions::default();
}

/// Add meshes/visuals for spawned players
fn spawn_visuals(
    // we do not want to add visuals to confirmed entities on the client
    query: Query<(Entity, Has<Controlled>, Has<Predicted>), (VisibleFilter, Added<Player>)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    // mut atomized_materials: ResMut<Assets<AtomizedMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
    weapons_data: Res<WeaponsData>,
) {
    for (parent, is_controlled, is_predicted) in query.iter() {
        commands.entity(parent).insert(Visibility::default());

        // TODO: don't do this in host-server mode!
        // add visual interpolation on the predicted entity
        if is_predicted {
            commands.entity(parent).insert(VisualInterpolateStatus::<Transform>::default());
        }

        // Add headlights
        commands.entity(parent).with_children(|parent| {
            let headlamp_1_pos = Vec3::new(0.25, 0.0, -0.25);
            let headlamp_2_pos = Vec3::new(-0.25, 0.0, -0.25);
            for headlamp_index in 0..2 {
                let headlamp_pos = if headlamp_index == 0 {
                    headlamp_1_pos
                } else {
                    headlamp_2_pos
                };
                parent.spawn((
                    SpotLight {
                        intensity: 2_000_000.0,
                        color: Color::srgb(1.0, 0.95, 0.9),
                        outer_angle: 0.75,
                        inner_angle: 0.1,
                        range: 100.0,
                        shadows_enabled: true,
                        ..default()
                    },
                    Transform::from_translation(headlamp_pos)
                        .looking_at(Vec3::new(0.0, 0.0, -1.25), Vec3::Y),
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
            let mut window = windows.single_mut();
            mouse_pointer_off(&mut window);

            // spawn a camera for 1-st person view
            commands.entity(parent).with_children(|parent| {
                // spawn it as a child so we can sway the camera seperately from the ship
                parent.spawn((
                    Camera3d::default(),
                    Camera {
                        hdr: true,
                        ..default()
                    },
                    Bloom::NATURAL,
                    Projection::Perspective(PerspectiveProjection {
                        fov: 90.0_f32.to_radians(),
                        ..default()
                    }),
                    SfxListener::new(),
                ));

                // 3d hud is attached to the ship, seperate from the camera so we can see the camera sway
                spawn_3d_hud(&asset_server, &mut meshes, &mut materials, parent, &weapons_data);
            });
        }
    }
}
