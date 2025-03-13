use avian3d::prelude::{AngularVelocity, LinearVelocity};
use bevy::{diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}, pbr::NotShadowCaster, prelude::*};
use bevy_config_stack::prelude::ConfigAssetLoaderPlugin;
use bevy_rich_text3d::{Text3d, Text3dPlugin, Text3dStyling, TextAtlas};

use lightyear::{client::prediction::diagnostics::PredictionMetrics, shared::replication::components::Controlled};
use serde::Deserialize;
use shared::weapons::{CurrentWeaponIndex, WeaponsData};

use shared::player::Player;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_hud);
        app.add_plugins(ConfigAssetLoaderPlugin::<HudConfig>::new("data/hud.ron"));
        app.add_plugins(Text3dPlugin{
            default_atlas_dimension: (1024, 1024),
            load_system_fonts: true,
            load_font_directories: vec!["../assets/fonts".to_owned()],
            ..default()
        });
        app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        app.add_systems(Update, (
            prediction_metrics_system,
            crosshair_system,
            camera_sway_system.run_if(resource_exists::<HudConfig>)
        ));
    }
}

#[derive(Asset, TypePath, Default, Deserialize, Debug, Resource)]
struct HudConfig {
    pub head_recenter_speed: f32,
    pub head_pitch_amount: f32,
    pub head_yaw_amount: f32,
    pub head_roll_amount: f32,
    pub head_x_amount: f32,
    pub head_y_amount: f32,
    pub head_z_amount: f32,
}

#[derive(Default, Component)]
struct Hud {
    pub health: i32,
    pub red_key: bool,
    pub blue_key: bool,
    pub yellow_key: bool,
}

#[derive(Component)]
struct PredictionMetricsText;

#[derive(Component, Debug)]
struct Crosshair;

#[derive(Component, Default, Clone)]
struct GForceData {
    prev_linear_velocity: Vec3,
    prev_angular_velocity: Vec3,
}

fn prediction_metrics_system(
    diagnostics: Res<DiagnosticsStore>,
    prediction_metrics: Option<Res<PredictionMetrics>>,
    mut text_query: Query<&mut Text, With<PredictionMetricsText>>,
) {
    if let Some(prediction_metrics) = prediction_metrics {
        if let Ok(mut text) = text_query.get_single_mut() {
            if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
                text.0 = format!("FPS: {}\nRollbacks: {}\nRollback Ticks: {}",
                    fps.smoothed().unwrap_or(0.0).round(),
                    prediction_metrics.rollbacks,
                    prediction_metrics.rollback_ticks
                );
            } else {
                text.0 = format!("Rollbacks: {}\nRollback Ticks: {}",
                    prediction_metrics.rollbacks,
                    prediction_metrics.rollback_ticks
                );
            }
        }
    }
}

fn spawn_hud(
    trigger: Trigger<OnAdd, Camera3d>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    weapons_data: Res<WeaponsData>,
) {

    // let crosshair = Crosshair {
    //     textures: weapons_data.weapons.iter().map(|weapon| asset_server.load(&weapon.1.crosshair.image)).collect()
    // };

    // let default_crosshair = crosshair.textures.first().unwrap().clone();

    commands
        .spawn((
            Hud {
                health: 100,
                red_key: false,
                blue_key: false,
                yellow_key: false,
            },
            Node {
                position_type: PositionType::Relative,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            }
        )).with_children(|parent| {
            // Prediction metrics text
            parent.spawn((
                PredictionMetricsText,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(10.0),
                    right: Val::Px(10.0),
                    ..default()
                },
                Text::new("Prediction metrics..."),
            ));

        //     // Crosshair
        //     parent.spawn((
        //         Crosshair {
        //             textures: vec![
        //                 default_crosshair.clone(),
        //                 asset_server.load("textures/hud/crosshairs/kenney_crosshair_pack/crosshair188.png"),
        //                 asset_server.load("textures/hud/crosshairs/kenney_crosshair_pack/crosshair030.png"),
        //                 asset_server.load("textures/hud/crosshairs/kenney_crosshair_pack/crosshair043.png"),
        //                 asset_server.load("textures/hud/crosshairs/kenney_crosshair_pack/crosshair018.png"),
        //             ],
        //         },
        //         ImageNode {
        //             color: Color::srgba(1.0, 1.0, 1.0, 0.15),
        //             image: default_crosshair,
        //             ..default()
        //         },
        //         Node {
        //             position_type: PositionType::Absolute,
        //             width: Val::Px(200.0),
        //             height: Val::Px(200.0),
        //             right: Val::Percent(50.0),
        //             bottom: Val::Percent(50.0),
        //             margin: UiRect {
        //                 right: Val::Px(-100.0),
        //                 bottom: Val::Px(-100.0),
        //                 ..default()
        //             },
        //             ..default()
        //         }
        //     ));
        });
}

pub fn spawn_3d_hud(
    asset_server: &AssetServer,
    mut meshes: &mut Assets<Mesh>,
    mut materials: &mut Assets<StandardMaterial>,
    ship: &mut ChildBuilder,
) {
    ship.spawn((
        Crosshair,
        Mesh3d(meshes.add(Mesh::from(Rectangle::new(0.0375, 0.0375)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            base_color: Color::srgba(10.0, 5.0, 1.0, 1.0),
            emissive: LinearRgba::new(1.0, 0.5, 0.0, 1.0),
            base_color_texture: Some(asset_server.load("textures/hud/crosshairs/kenney_crosshair_pack/crosshair018.png")),
            ..Default::default()
        })),
        Transform::from_translation(Vec3::new(0.0, 0.0, -0.25)),
        NotShadowCaster,
    ));

    ship.spawn((
        Text3d::new("100"),
        Text3dStyling {
            font: "Roboto".into(),
            size: 64.0,
            ..default()
        },
        Mesh3d::default(),
        MeshMaterial3d(materials.add(
            StandardMaterial {
                base_color: Color::srgba(10.0, 5.0, 1.0, 1.0),
                emissive: LinearRgba::new(1.0, 0.5, 0.0, 1.0),
                base_color_texture: Some(TextAtlas::DEFAULT_IMAGE.clone()),
                alpha_mode: AlphaMode::Blend,
                ..Default::default()
            }
        )),
        Transform::from_translation(Vec3::new(-0.25, -0.15, -0.25))
            .with_scale(Vec3::new(0.001, 0.001, 0.001))
            .with_rotation(Quat::from_euler(EulerRot::XYZ, 0.0, 0.45, 0.0)),
        NotShadowCaster,
    ));
}

/// Fakes g-forces by swaying the camera
fn camera_sway_system(
    time: Res<Time>,
    config: Res<HudConfig>,
    mut player_ship_query: Query<(&LinearVelocity, &AngularVelocity, &Children, &Transform), With<Player>>,
    mut camera_query: Query<(&mut Transform, Option<&mut GForceData>), (With<Camera3d>, Without<Player>)>,
    mut commands: Commands,
) {
    if let Ok((ship_linear_velocity, ship_angular_velocity, children, ship_transform)) = player_ship_query.get_single() {
        for child in children.iter() {
            if let Ok((mut camera_transform, g_force_opt)) = camera_query.get_mut(*child) {
                let current_vel = ship_transform.rotation.inverse() * ship_linear_velocity.0;
                let current_ang = ship_transform.rotation.inverse() * ship_angular_velocity.0;
                
                if let Some(mut g_data) = g_force_opt {
                    let dt = time.delta_secs();
                    let accel = (current_vel - g_data.prev_linear_velocity) / dt;
                    
                    g_data.prev_linear_velocity = current_vel;
                    g_data.prev_angular_velocity = current_ang;
                    
                    // Calculate rotational g-forces
                    let pitch = -accel.z * config.head_pitch_amount;
                    let roll = accel.x * config.head_roll_amount + current_ang.y * config.head_roll_amount;
                    let yaw = -accel.x * config.head_yaw_amount;

                    let target_rotation = Quat::from_euler(
                        EulerRot::XYZ,
                        pitch,
                        yaw,
                        roll
                    );
                    
                    // Calculate positional g-forces (physical movement)
                    // Move camera backward when accelerating forward
                    let translation_z = -accel.z * config.head_z_amount;
                    // Move camera left when accelerating right
                    let translation_x = -accel.x * config.head_x_amount;
                    // Move camera down when accelerating up
                    let translation_y = -accel.y * config.head_y_amount;
                    
                    // Apply translation target based on acceleration
                    let target_translation = Vec3::new(
                        translation_x,
                        translation_y, 
                        translation_z
                    );
                    
                    // Smoothly interpolate rotation
                    camera_transform.rotation = camera_transform.rotation.slerp(
                        target_rotation, 
                        config.head_recenter_speed * dt
                    );
                    
                    // Smoothly interpolate position
                    camera_transform.translation = camera_transform.translation.lerp(
                        target_translation,
                        config.head_recenter_speed * dt
                    );
                } else {
                    commands.entity(*child).insert(GForceData {
                        prev_linear_velocity: current_vel,
                        prev_angular_velocity: current_ang,
                    });
                    
                    camera_transform.rotation = Quat::IDENTITY;
                    camera_transform.translation = Vec3::ZERO;
                }
            }
        }
    }
}

fn crosshair_system(
    mut commands: Commands,
    mut crosshair: Query<(&Crosshair, &mut ImageNode)>,
    mut current_weapon_idx: Query<&CurrentWeaponIndex, (With<Controlled>, Changed<CurrentWeaponIndex>)>,
) {
    // let Ok(current_weapon_idx) = current_weapon_idx.get_single() else { return };

    // let (crosshair, mut image) = crosshair.single_mut();
    // if let Some(current_weapon_handle) = crosshair.textures.get(current_weapon_idx.0 as usize) {
    //     if image.image != *current_weapon_handle{
    //         image.image = current_weapon_handle.clone();
    //     }
    // }
}
