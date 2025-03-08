use std::time::Duration;

use bevy::{core_pipeline::prepass::DepthPrepass, pbr::{ExtendedMaterial, NotShadowCaster, OpaqueRendererMethod}, prelude::*, render::{render_resource::StoreOp, renderer::RenderDevice, view::{ViewDepthTexture, ViewTarget}}};
use bevy_flycam::{FlyCam, NoCameraPlayerPlugin};
use avian3d::prelude::*;
use vfx::prelude::*;

fn main() {
    App::new()
        .insert_resource(Ticker(Timer::new(Duration::from_secs(2), TimerMode::Repeating)))
        .init_resource::<VfxAssets>()
        .add_plugins(DefaultPlugins)
        .add_plugins(VfxPlugin)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(NoCameraPlayerPlugin)
        .add_systems(Startup, setup_system)
        .add_systems(Update, spawn_vfx_emitter_system)
        .run();
}

#[derive(Resource)]
struct Ticker(pub Timer);

fn setup_system(
    asset_server: Res<AssetServer>,
    mut vfx_assets: ResMut<VfxAssets>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut spawn_vfx_emitter_events: EventWriter<SpawnVfxEmitterEvent>,
) {
    let camera_transform = Transform::default().with_translation(Vec3::new(0.0, 10.0, 10.0)).looking_at(Vec3::ZERO, Vec3::Y);
    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        camera_transform.clone(),
        FlyCam,
        DepthPrepass::default(),
    ));

    // spawn a floor
    commands.spawn((
        Collider::cuboid(10.0, 0.1, 10.0),
        Mesh3d::from(meshes.add(Cuboid::new(10.0, 0.1, 10.0))),
        MeshMaterial3d::from(standard_materials.add(Color::srgba(0.1, 0.1, 0.1, 1.0))),
    ));

    // spawn a light
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 10.0, 10.0))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // spawn a pillar
    commands.spawn((
        Collider::cuboid(1.0, 10.0, 1.0),
        Mesh3d::from(meshes.add(Cuboid::new(1.0, 10.0, 1.0))),
        MeshMaterial3d::from(standard_materials.add(Color::srgba(0.5, 0.1, 0.1, 1.0))),
        Transform::from_translation(Vec3::new(0.5, 5.0, 1.65)),
    ));

}

fn spawn_vfx_emitter_system(
    time: Res<Time>,
    mut ticker: ResMut<Ticker>,
    mut spawn_emitter_events: EventWriter<SpawnVfxEmitterEvent>,
) {
    ticker.0.tick(time.delta());
    if ticker.0.just_finished() {

        // Spawn Smoke
        spawn_emitter_events.send(
            SpawnVfxEmitterEvent {
                translation: Vec3::ZERO.with_y(1.0), 
                behavior: VfxEmitterBehavior {
                    count_per_burst: 16,
                    burst_count: 1,
                    burst_rate_millis: 0,
                    initial_scale: (1.0, 1.0),
                    initial_velocity: (Vec3::NEG_ONE, Vec3::ONE),
                    velocity_decay: (0.5, 1.5),
                    scale_velocity: (0.0, 3.0),
                    scale_velocity_decay: (0.1, 0.5),
                    scale_over_lifetime: vec![],
                    lifetime_millis: (1000, 2000),
                    color_over_lifetime: vec![
                        VfxPercentColorRange {
                            lifetime_percent: (0, 0),
                            color: (
                                Color::BLACK.with_alpha(0.75),
                                Color::WHITE.with_alpha(0.75),
                            ),
                        },
                        VfxPercentColorRange {
                            lifetime_percent: (100, 100),
                            color: (
                                Color::BLACK.with_alpha(0.0),
                                Color::BLACK.with_alpha(0.0),
                            ),
                        },
                    ],
                    wave_amplitude: (0.2, 0.5),
                    wave_frequency: (0.4, 0.5),
                    textures: vec![
                        "textures/smoke_01.png".to_string(),
                        "textures/smoke_02.png".to_string(),
                        "textures/smoke_03.png".to_string(),
                        "textures/smoke_04.png".to_string(),
                        "textures/smoke_05.png".to_string(),
                        "textures/smoke_06.png".to_string(),
                        "textures/smoke_07.png".to_string(),
                        "textures/smoke_08.png".to_string(),
                        "textures/smoke_09.png".to_string(),
                        "textures/smoke_10.png".to_string(),
                    ],
                    receive_shadows: true,
                    bounce_factor: 0.0,
                }, 
            }
        );

        // Spawn sparks
        spawn_emitter_events.send(
            SpawnVfxEmitterEvent {
                translation: Vec3::ZERO.with_y(1.0), 
                behavior: VfxEmitterBehavior {
                    count_per_burst: 28,
                    burst_count: 1,
                    burst_rate_millis: 0,
                    initial_scale: (0.05, 0.01),
                    initial_velocity: (Vec3::new(-6.0, -6.0, -6.0), Vec3::new(6.0, 6.0, 6.0)),
                    velocity_decay: (0.5, 1.5),
                    scale_velocity: (0.0, -2.0),
                    scale_velocity_decay: (0.1, 0.5),
                    scale_over_lifetime: vec![],
                    lifetime_millis: (1000, 2000),
                    color_over_lifetime: vec![
                        VfxPercentColorRange {
                            lifetime_percent: (0, 0),
                            color: (
                                Color::srgba(20.0, 10.0, 0.1, 1.0),
                                Color::srgba(30.0, 15.0, 0.1, 1.0),
                            ),
                        },
                        VfxPercentColorRange {
                            lifetime_percent: (100, 100),
                            color: (
                                Color::BLACK.with_alpha(0.0),
                                Color::BLACK.with_alpha(0.0),
                            ),
                        },
                    ],
                    wave_amplitude: (0.0, 0.0),
                    wave_frequency: (0.0, 0.0),
                    textures: vec![
                        "textures/circle_05.png".to_string(),
                    ],
                    receive_shadows: false,
                    bounce_factor: 0.5,
                },
            }
        );
    }
}

