use std::time::{Duration, Instant};

use bevy::prelude::*;
use sfx::prelude::*;
use bevy_flycam::{FlyCam, NoCameraPlayerPlugin};
use kira::{effect::eq_filter::EqFilterKind, track::SpatialTrackDistances, Decibels, Easing, Mapping, Value};

const CANNON_LOCATION: Vec3 = Vec3::new(30.0, 0.0, 0.0);
const SMG_LOCATION: Vec3 = Vec3::ZERO;

fn main() {
    App::new()
        .insert_resource(Timers {
            shoot_timer: Timer::new(Duration::from_millis(200), TimerMode::Once),
            cannon_timer: Timer::new(Duration::from_millis(2000), TimerMode::Once),
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(NoCameraPlayerPlugin)
        .add_plugins(SfxAudioPlugin::default())
        .add_systems(Startup, setup_world_system)
        .add_systems(Update, spawn_sfx_system)
        .add_systems(Update, muzzle_flash_system)
        .run();
}

#[derive(Component)]
pub struct MuzzleFlash {
    pub lifetime: Timer,
}

#[derive(Resource)]
struct Timers {
    shoot_timer: Timer,
    cannon_timer: Timer,
}

fn setup_world_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut sfx_manager: ResMut<SfxManager>,
    asset_server: Res<AssetServer>,
) {
    // Camera
    commands.spawn((
        SfxListener::new(),
        Camera3d::default(),
        Transform::default(),
        FlyCam
    ));

    // Light
    commands.spawn((
        DirectionalLight {
            illuminance: 6000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::default()
            .with_translation(Vec3::new(0.0, 10.0, 10.0))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Cube
    commands.spawn((
        Mesh3d::from(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d::from(materials.add(Color::WHITE)),
        Transform::from_translation(SMG_LOCATION),
    ));

    // Cube 2
    commands.spawn((
        Mesh3d::from(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d::from(materials.add(Color::WHITE)),
        Transform::from_translation(CANNON_LOCATION),
    ));

    // Floor
    commands.spawn((
        Mesh3d::from(meshes.add(Cuboid::new(100.0, 1.0, 100.0))),
        MeshMaterial3d::from(materials.add(Color::srgba(0.5, 0.5, 0.5, 1.0))),
        Transform::default()
            .with_translation(Vec3::new(0.0, -1.0, 0.0)),
    ));

    // Load sfx
    sfx_manager.load_sfx(
        "smg".to_string(), 
        "audio/weapons/440559__charliewd100__futuristic-smg-sound-effect.wav".to_string(), 
        &asset_server
    );

    sfx_manager.load_sfx(
        "cannon".to_string(), 
        "audio/weapons/448002__kneeling__cannon.mp3".to_string(), 
        &asset_server
    );
}

fn spawn_sfx_system(
    time: Res<Time>,
    mut commands: Commands,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut sfx_manager: ResMut<SfxManager>,
    mut timers: ResMut<Timers>,
    mut cameras: Query<(Entity, &Transform), With<Camera3d>>,
) {
    let (camera_entity, camera_transform) = cameras.single();
    timers.shoot_timer.tick(time.delta());
    timers.cannon_timer.tick(time.delta());
    if mouse_input.pressed(MouseButton::Left) {
        if timers.shoot_timer.finished() {
            commands.spawn(SfxEmitter {
                asset_unique_id: "smg".to_string(),
                spatial: Some(SpatialTrackDistances {
                    min_distance: 0.0,
                    max_distance: 150.0,
                    ..default()
                }),
                reverb: Some(ReverbSettings::default()),
                low_pass: Some(LowPassSettings::default()),
                eq: None,
                delay: None,
                doppler_enabled: true,
                loop_region: None,
                despawn_entity_after_secs: Some(4.0),
                // follow: Some(Follow {
                //     target: camera_entity,
                //     local_offset: Vec3::ZERO,
                // }),
                ..default()
            });

            // spawn muzzle flash above the smg cube
            commands.spawn((
                MuzzleFlash {
                    lifetime: Timer::new(Duration::from_millis(100), TimerMode::Once),
                },
                Transform::from_translation(SMG_LOCATION + Vec3::new(0.0, 2.0, 0.0)),
                PointLight {
                    color: Color::srgba(1.0, 0.5, 0.0, 1.0),
                    ..default()
                }
            ));

            timers.shoot_timer.reset();
        }
    }

    if timers.cannon_timer.finished() {
        commands.spawn((
            SfxEmitter {
                asset_unique_id: "cannon".to_string(),
                spatial: Some(SpatialTrackDistances {
                    min_distance: 0.0,
                    max_distance: 200.0,
                    ..default()
                }),
                reverb: Some(ReverbSettings::default()),
                low_pass: Some(LowPassSettings::default()),
                eq: Some(EqSettings {
                    frequencies: vec![
                        // Bass
                        EqFrequency { 
                            kind: EqFilterKind::Bell, 
                            frequency: 100.0,
                            gain: Value::FromListenerDistance(Mapping {
                                input_range: (0.0, 200.0),
                                output_range: (Decibels(0.0), Decibels(20.0)),
                                easing: Easing::Linear,
                            }),
                            q: 1.0 
                        },
                        // Mids
                        EqFrequency { 
                            kind: EqFilterKind::Bell, 
                            frequency: 1000.0,
                            gain: Value::FromListenerDistance(Mapping {
                                input_range: (0.0, 200.0),
                                output_range: (Decibels(0.0), Decibels(-20.0)),
                                easing: Easing::Linear,
                            }),
                            q: 1.0 
                        },
                        // Highs
                        EqFrequency { 
                            kind: EqFilterKind::Bell, 
                            frequency: 10000.0,
                            gain: Value::FromListenerDistance(Mapping {
                                input_range: (0.0, 200.0),
                                output_range: (Decibels(0.0), Decibels(-20.0)),
                                easing: Easing::Linear,
                            }),
                            q: 1.0 
                        },
                    ],
                }),
                delay: None,
                doppler_enabled: true,
                loop_region: None,
                despawn_entity_after_secs: Some(4.0),
                // follow: Some(Follow {
                //     target: camera_entity,
                //     local_offset: Vec3::ZERO,
                // }),
                ..default()
            },
            Transform::from_translation(CANNON_LOCATION)
        ));

        // spawn muzzle flash above the cannon cube
        commands.spawn((
            MuzzleFlash {
                lifetime: Timer::new(Duration::from_millis(200), TimerMode::Once),
            },
            Transform::from_translation(CANNON_LOCATION + Vec3::new(0.0, 2.0, 0.0)),
            PointLight {
                color: Color::srgba(1.0, 0.5, 0.0, 1.0),
                ..default()
            }
        ));

        timers.cannon_timer.reset();
    }
}

fn muzzle_flash_system(
    time: Res<Time>,
    mut commands: Commands,
    mut muzzle_flashes: Query<(Entity, &mut MuzzleFlash)>,
) {
    for (entity, mut muzzle_flash) in muzzle_flashes.iter_mut() {
        muzzle_flash.lifetime.tick(time.delta());
        if muzzle_flash.lifetime.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

