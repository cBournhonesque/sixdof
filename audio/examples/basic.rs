use std::time::Duration;

use bevy::prelude::*;
use audio::prelude::*;
use bevy_flycam::{FlyCam, NoCameraPlayerPlugin};
use kira::{effect::eq_filter::EqFilterKind, Decibels, Easing, Mapping, Value};

fn main() {
    App::new()  
        .add_plugins(DefaultPlugins)
        .add_plugins(NoCameraPlayerPlugin)
        .add_plugins(SfxAudioPlugin::default())
        .add_systems(Startup, setup_world_system)
        .add_systems(Update, spawn_sfx_system)
        .run();
}

#[derive(Component)]
struct ShootTimer {
    timer: Timer,
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
        ShootTimer {
            timer: Timer::new(Duration::from_millis(200), TimerMode::Once),
        },
        FlyCam
    ));

    // Light
    commands.spawn((
        DirectionalLight {
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
        Transform::from_translation(Vec3::ZERO),
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
}

fn spawn_sfx_system(
    time: Res<Time>,
    mut commands: Commands,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut sfx_manager: ResMut<SfxManager>,
    mut shoot_timer: Query<&mut ShootTimer>,
    mut cameras: Query<(Entity, &Transform), With<Camera3d>>,
) {
    let (camera_entity, camera_transform) = cameras.single();
    if let Ok(mut shoot_timer) = shoot_timer.get_single_mut() {
        shoot_timer.timer.tick(time.delta());
        if mouse_input.pressed(MouseButton::Left) {
            if shoot_timer.timer.finished() {
                commands.spawn(SfxSpatialEmitter {
                    asset_unique_id: "smg".to_string(),
                    reverb: Some(ReverbSettings::default()),
                    low_pass: Some(LowPassSettings::default()),
                    eq: None,
                    delay: None,
                    volume: Value::FromListenerDistance(Mapping {
                        input_range: (3.0, 100.0),
                        output_range: (Decibels(1.0), Decibels(0.0)),
                        easing: Easing::Linear,
                    }),
                    loop_region: None,
                    despawn_entity_after_secs: Some(4.0),
                    // follow: Some(Follow {
                    //     target: camera_entity,
                    //     local_offset: Vec3::ZERO,
                    // }),
                    ..default()
                });
                shoot_timer.timer.reset();
            }
        }
    }
}
