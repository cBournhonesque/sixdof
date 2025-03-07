use bevy::{core_pipeline::prepass::DepthPrepass, pbr::{ExtendedMaterial, NotShadowCaster, OpaqueRendererMethod}, prelude::*, render::{render_resource::StoreOp, renderer::RenderDevice, view::{ViewDepthTexture, ViewTarget}}};
use bevy_flycam::{FlyCam, NoCameraPlayerPlugin};
use vfx::SoftParticleMaterialExtension;

pub struct VfxPlugin;

impl Plugin for VfxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VfxAssets>();
        app.add_plugins(NoCameraPlayerPlugin);
        app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, SoftParticleMaterialExtension>>::default());
        app.add_systems(Startup, setup_vfx);
        app.add_systems(Update, billboard_system);
        app.add_systems(Update, update_material_time_system);
    }
}

#[derive(Resource, Default)]
struct VfxAssets {
    quad: Option<Handle<Mesh>>,
    particle_material: Option<Handle<ExtendedMaterial<StandardMaterial, SoftParticleMaterialExtension>>>,
}

#[derive(Component)]
struct Billboard;

fn billboard_system(
    mut billboards: Query<&mut Transform, With<Billboard>>,
    camera: Query<&Transform, (With<Camera>, Without<Billboard>)>,
) {
    let Ok(camera_transform) = camera.get_single() else { return };
    for mut transform in &mut billboards {
        transform.rotation = camera_transform.rotation;
    }
}

fn update_material_time_system(
    time: Res<Time>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, SoftParticleMaterialExtension>>>,
) {
    for (_, material) in materials.iter_mut() {
        material.extension.time = time.elapsed_secs();
    }
}

fn setup_vfx(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut vfx_assets: ResMut<VfxAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut soft_particle_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, SoftParticleMaterialExtension>>>,
) {
    vfx_assets.quad = Some(meshes.add(Rectangle::new(1.0, 1.0)));
        vfx_assets.particle_material = Some(soft_particle_materials.add(ExtendedMaterial {
            base: StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(asset_server.load("textures/smoke_07.png")),
            alpha_mode: AlphaMode::Blend,
            ..Default::default()
        },
        extension: SoftParticleMaterialExtension { 
            softness_factor: 6.0, 
            wave_amplitude: 0.1,
            wave_frequency: 1.0,
            time: 0.0,
        },
    }));
    
    commands.spawn((
        Camera3d::default(),
        Transform::default().with_translation(Vec3::new(0.0, 10.0, 10.0))
            .looking_at(Vec3::ZERO, Vec3::Y),
        FlyCam,
        DepthPrepass::default(),
    ));

    // spawn a floor
    commands.spawn((
        Mesh3d::from(meshes.add(Cuboid::new(10.0, 0.1, 10.0))),
        MeshMaterial3d::from(standard_materials.add(Color::srgba(0.1, 0.1, 0.1, 1.0))),
    ));

    // spawn a quad
    if let (Some(quad), Some(particle_material)) = (&vfx_assets.quad, &vfx_assets.particle_material) { 
        commands.spawn((
            Mesh3d::from(quad.clone()),
            MeshMaterial3d::from(particle_material.clone()),
            Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            Billboard,
            NotShadowCaster,
        ));
    }

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
        Mesh3d::from(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d::from(standard_materials.add(Color::srgba(0.5, 0.1, 0.1, 1.0))),
        Transform::from_translation(Vec3::new(0.5, 0.0, 0.65)),
    ));
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VfxPlugin)
        .run();
}
