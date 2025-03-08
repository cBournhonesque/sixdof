mod material;
mod data;

pub mod prelude {
    pub use super::*;
    pub use super::material::*;
    pub use super::data::*;
}

use std::time::Duration;

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline, NotShadowCaster, NotShadowReceiver}, 
    prelude::*, render::{mesh::MeshVertexBufferLayoutRef, render_resource::{AsBindGroup, AsBindGroupShaderType, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, CompareFunction, DepthBiasState, DepthStencilState, RenderPipelineDescriptor, Sampler, ShaderRef, ShaderStages, ShaderType, SpecializedMeshPipelineError, StencilState, TextureFormat, TextureSampleType, TextureViewDimension}, renderer::RenderDevice}, utils::HashMap};
use data::*;
use material::*;
use rand::{rngs::ThreadRng, Rng};
use serde::{Serialize, Deserialize};

pub struct VfxPlugin;

impl Plugin for VfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnVfxEmitterEvent>();
        app.init_resource::<VfxAssets>();
        app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, SoftParticleMaterialExtension>>::default());
        app.add_systems(Startup, setup_vfx);
        app.add_systems(Update, (
            spawn_emitter_system,
            move_particle_system, 
            scale_velocity_particle_system,
            scale_over_lifetime_particle_system,
            color_over_lifetime_system,
            billboard_system,
            update_material_time_system,
            particle_despawn_system,
            tick_continuous_emitter_system,
        ));
    }
}

fn setup_vfx(
    mut meshes: ResMut<Assets<Mesh>>,
    mut vfx_assets: ResMut<VfxAssets>,
) {
    vfx_assets.quad = Some(meshes.add(Rectangle::new(1.0, 1.0)));
}

#[derive(Event)]
pub struct SpawnVfxEmitterEvent {
    pub translation: Vec3,
    pub behavior: VfxEmitterBehavior,
}

#[derive(Resource, Default)]
pub struct VfxAssets {
    quad: Option<Handle<Mesh>>,
    textures: HashMap<String, Handle<Image>>,
}

#[derive(Component)]
pub struct VfxContinuousEmitter {
    pub count_per_burst: u32,
    pub count_left: u32,
    pub timer: Timer,
    pub behavior: VfxEmitterBehavior,
}

/// A component that makes the particle face the camera like a sprite. You'll probably want to use this.
#[derive(Component)]
pub struct VfxBillboard;

/// Moves over time by the given vector.
#[derive(Component, Debug)]
pub struct VfxVelocity(pub Vec3);

/// Scales over the lifetime of the particle.
#[derive(Component, Debug)]
pub struct VfxScaleOverLifetime {
    pub percent_scales: Vec<VfxPercentScale>,
}

/// Scales over time by the given vector.
#[derive(Component, Debug)]
pub struct VfxScaleVelocity(pub Vec3);

/// Applies an external force to the particle. Such as gravity and wind.
#[derive(Component, Debug)]
pub struct VfxExternalForce(pub Vec3);

/// Decays the velocity over time. Essentially a drag force.
#[derive(Component, Debug)]
pub struct VfxVelocityDecay(pub f32);

/// Decays the scale velocityover time.
#[derive(Component, Debug)]
pub struct VfxScaleVelocityDecay(pub f32);

/// A timer that controls the lifetime of the particle. When it's over, it's despawned.
#[derive(Component, Debug)]
pub struct VfxLifetime(pub Timer);

/// The bounce factor of the particle.
#[derive(Component, Debug)]
pub struct VfxBounce(pub f32);

/// Needed for `VfxScaleKind::OverLifetime`, defines the lifetime percent and scale at that percent.
#[derive(Debug, Serialize, Deserialize)]
pub struct VfxPercentScale {
    pub lifetime_percent: i32,
    pub scale: Vec3,
}

/// Needed for `VfxColorOverLifetime`, defines the lifetime percent and color at that percent.
#[derive(Debug, Serialize, Deserialize)]
pub struct VfxPercentColor {
    pub lifetime_percent: i32,
    pub color: Color,
}

/// Modifies the base color of the particle material over time.
#[derive(Component ,Debug, Serialize, Deserialize)]
pub struct VfxColorOverLifetime {
    pub percent_colors: Vec<VfxPercentColor>,
}

fn spawn_emitter_system(
    mut commands: Commands,
    mut camera_transform: Query<&Transform, (With<Camera>, Without<VfxContinuousEmitter>)>,
    mut asset_server: Res<AssetServer>,
    mut vfx_assets: ResMut<VfxAssets>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, SoftParticleMaterialExtension>>>,
    mut events: EventReader<SpawnVfxEmitterEvent>,
) {
    let Ok(camera_transform) = camera_transform.get_single() else { return };
    for event in events.read() {
        spawn_vfx_emitter(
            &mut commands, 
            &camera_transform, 
            &asset_server, 
            &event.behavior, 
            event.translation, 
            &mut vfx_assets, 
            &mut materials
        );
    }
}

fn move_particle_system(
    time: Res<Time>,
    mut velocities: Query<(Entity,&mut Transform, &mut VfxVelocity)>,
    mut external_forces: Query<&VfxExternalForce>,
    mut velocity_decays: Query<&VfxVelocityDecay>,
    mut bounces: Query<&VfxBounce>,
    spatial_query: SpatialQuery,
) {
    for (entity, mut transform, mut vfx_velocity) in velocities.iter_mut() {

        if let Ok(drag) = velocity_decays.get(entity) {
            vfx_velocity.0 *= 1.0 - drag.0 * time.delta_secs();
        }
        
        if let Ok(bounce_factor) = bounces.get(entity) {
            let hit = spatial_query.cast_ray(
                transform.translation, 
                Dir3::new(vfx_velocity.0).unwrap_or(Dir3::Y), 
                vfx_velocity.0.length() * time.delta_secs(),
                true, 
                &SpatialQueryFilter::default(),
            );

            if let Some(hit) = hit {
                transform.translation += hit.normal * hit.distance;
                vfx_velocity.0 = vfx_velocity.0.reflect(hit.normal) * bounce_factor.0;
            } else {
                transform.translation += vfx_velocity.0 * time.delta_secs();
            }
        } else {
            transform.translation += vfx_velocity.0 * time.delta_secs();
        }
    }
}

fn scale_velocity_particle_system(
    time: Res<Time>,
    mut scale_velocities: Query<(Entity, &mut Transform, &mut VfxScaleVelocity)>,
    mut scale_decays: Query<&VfxScaleVelocityDecay>,
) {
    for (entity, mut transform, mut vfx_scale_velocity) in scale_velocities.iter_mut() {
        if let Ok(scale_decay) = scale_decays.get(entity) {
            vfx_scale_velocity.0 *= 1.0 - scale_decay.0 * time.delta_secs();
        }

        transform.scale += vfx_scale_velocity.0 * time.delta_secs();
    }
}

fn scale_over_lifetime_particle_system(
    time: Res<Time>,
    mut particles: Query<(&mut Transform, &VfxScaleOverLifetime, &VfxLifetime)>,
) {
    for (mut transform, scale_over_lifetime, lifetime) in &mut particles {
        let lifetime_percent_f32 = lifetime.0.elapsed_secs() / lifetime.0.duration().as_secs_f32() * 100.0;
        let lifetime_percent_i32 = lifetime_percent_f32 as i32;
        
        let percent_scales = &scale_over_lifetime.percent_scales;
        
        if percent_scales.is_empty() {
            continue;
        }
        
        if percent_scales.len() == 1 {
            transform.scale = percent_scales[0].scale;
            continue;
        }
        
        let mut prev_scale = &percent_scales[0];
        let mut next_scale = &percent_scales[0];
        
        for i in 0..percent_scales.len() {
            if percent_scales[i].lifetime_percent <= lifetime_percent_i32 {
                prev_scale = &percent_scales[i];
            }
            
            if i < percent_scales.len() - 1 && 
               percent_scales[i].lifetime_percent <= lifetime_percent_i32 && 
               percent_scales[i+1].lifetime_percent > lifetime_percent_i32 {
                next_scale = &percent_scales[i+1];
                break;
            }
        }
        
        if prev_scale.lifetime_percent > lifetime_percent_i32 {
            transform.scale = percent_scales[0].scale;
            continue;
        }
        
        if prev_scale.lifetime_percent == percent_scales.last().unwrap().lifetime_percent {
            transform.scale = prev_scale.scale;
            continue;
        }
        
        let t = if next_scale.lifetime_percent != prev_scale.lifetime_percent {
            (lifetime_percent_f32 - prev_scale.lifetime_percent as f32) / 
            (next_scale.lifetime_percent as f32 - prev_scale.lifetime_percent as f32)
        } else {
            0.0
        };
        
        let t = t.clamp(0.0, 1.0);
        
        transform.scale = Vec3::new(
            prev_scale.scale.x * (1.0 - t) + next_scale.scale.x * t,
            prev_scale.scale.y * (1.0 - t) + next_scale.scale.y * t,
            prev_scale.scale.z * (1.0 - t) + next_scale.scale.z * t
        );
    }
}

fn color_over_lifetime_system(
    time: Res<Time>,
    mut commands: Commands,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, SoftParticleMaterialExtension>>>,
    mut particles: Query<(
        Entity, 
        &mut VfxLifetime, 
        &VfxColorOverLifetime, 
        &MeshMaterial3d<ExtendedMaterial<StandardMaterial, SoftParticleMaterialExtension>>,
    )>,
) {
    for (entity, mut lifetime, color_over_lifetime, material_handle) in &mut particles {
        lifetime.0.tick(time.delta());
    
        let lifetime_percent = lifetime.0.elapsed_secs() / lifetime.0.duration().as_secs_f32();
        let lifetime_percent_i32 = (lifetime_percent * 100.0) as i32;
        
        let handle = material_handle.0.clone();
        
        // Find the appropriate color based on the percentage
        if let Some(material) = materials.get_mut(&handle) {
            let percent_colors = &color_over_lifetime.percent_colors;
            let mut color_index = 0;
            
            for (i, percent_color) in percent_colors.iter().enumerate() {
                if percent_color.lifetime_percent <= lifetime_percent_i32 {
                    color_index = i;
                } else {
                    break;
                }
            }
            
            let current_color;
            
            if color_index >= percent_colors.len() - 1 {
                current_color = percent_colors[color_index].color;
            } else {
                let next_index = color_index + 1;
                let prev_percent = percent_colors[color_index].lifetime_percent;
                let next_percent = percent_colors[next_index].lifetime_percent;
                
                let t = if next_percent != prev_percent {
                    (lifetime_percent_i32 - prev_percent) as f32 / (next_percent - prev_percent) as f32
                } else {
                    0.0
                };
                
                let color1 = percent_colors[color_index].color.to_srgba();
                let color2 = percent_colors[next_index].color.to_srgba();
                
                // Leeeeeeeeerp lerp lerp
                let r = color1.red * (1.0 - t) + color2.red * t;
                let g = color1.green * (1.0 - t) + color2.green * t;
                let b = color1.blue * (1.0 - t) + color2.blue * t;
                let a = color1.alpha * (1.0 - t) + color2.alpha * t;
                
                current_color = Color::srgba(r, g, b, a);
            }
            
            material.base.base_color = current_color;
        }
    }
}

fn particle_despawn_system(
    mut commands: Commands,
    mut particles: Query<(Entity, &mut VfxLifetime)>,
) {
    for (entity, mut lifetime) in &mut particles {
        if lifetime.0.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn billboard_system(
    mut billboards: Query<&mut Transform, With<VfxBillboard>>,
    camera: Query<&Transform, (With<Camera>, Without<VfxBillboard>)>,
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

fn tick_continuous_emitter_system(
    time: Res<Time>,
    camera_transform: Query<&Transform, (With<Camera>, Without<VfxContinuousEmitter>)>,
    mut commands: Commands,
    mut asset_server: ResMut<AssetServer>,
    mut vfx_assets: ResMut<VfxAssets>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, SoftParticleMaterialExtension>>>,
    mut continuous_emitters: Query<(Entity, &Transform, &mut VfxContinuousEmitter), Without<Camera>>,
) {
    let Ok(camera_transform) = camera_transform.get_single() else { return };
    for (entity, transform, mut continuous_emitter) in &mut continuous_emitters {
        continuous_emitter.timer.tick(time.delta());
        if continuous_emitter.timer.finished() {
            if continuous_emitter.count_left > 0 {
                let mut rng = rand::rng();
                burst_particles(
                    &mut commands, 
                    &camera_transform,
                    &mut asset_server, 
                    &continuous_emitter.behavior, 
                    continuous_emitter.count_per_burst, 
                    transform.translation, 
                    &mut vfx_assets,
                    &mut materials,
                    &mut rng,
                );
                continuous_emitter.count_left = continuous_emitter.count_left.saturating_sub(1);
            }
        }

        if continuous_emitter.count_left <= 0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}

pub fn spawn_vfx_emitter(
    commands: &mut Commands,
    camera_transform: &Transform,
    asset_server: &AssetServer,
    behavior: &VfxEmitterBehavior,
    translation: Vec3,
    mut vfx_assets: &mut VfxAssets,
    materials: &mut ResMut<Assets<ExtendedMaterial<StandardMaterial, SoftParticleMaterialExtension>>>,
) {
    let mut rng = rand::rng();
    burst_particles(
        commands, 
        camera_transform,
        asset_server, 
        behavior, 
        behavior.count_per_burst, 
        translation, 
        &mut vfx_assets,
        materials,
        &mut rng,
    );
    commands.spawn((
        VfxContinuousEmitter {
            behavior: behavior.clone(),
            count_per_burst: behavior.count_per_burst,
            count_left: behavior.burst_count - 1,
            timer: Timer::new(Duration::from_millis(behavior.burst_rate_millis as u64), TimerMode::Repeating),
        },
        Transform::from_translation(translation),
    ));
}

/// Returns the number of particles spawned.
fn burst_particles(
    commands: &mut Commands,
    camera_transform: &Transform,
    asset_server: &AssetServer,
    emitter_behavior: &VfxEmitterBehavior,
    count_per_burst: u32,
    translation: Vec3,
    vfx_assets: &mut VfxAssets,
    materials: &mut ResMut<Assets<ExtendedMaterial<StandardMaterial, SoftParticleMaterialExtension>>>,
    mut rng: &mut ThreadRng,
) {
    let Some(quad) = &vfx_assets.quad else { return };

    for _ in 0..count_per_burst {

        let texture_name = if emitter_behavior.textures.len() > 0 {
            let texture_index = rng.random_range(0..emitter_behavior.textures.len());
            Some(&emitter_behavior.textures[texture_index])
        } else {
            None
        };

        let base_color_texture = {
            if let Some(texture_name) = texture_name {
                if let Some(texture) = vfx_assets.textures.get(texture_name) {
                    Some(texture.clone())
                } else {
                    Some(asset_server.load(texture_name))
                }
            } else {
                None
            }
        };

        let wave_amplitude = random_f32(emitter_behavior.wave_amplitude.0, emitter_behavior.wave_amplitude.1, &mut rng);
        let wave_frequency = random_f32(emitter_behavior.wave_frequency.0, emitter_behavior.wave_frequency.1, &mut rng);
        let mut initial_scale = random_f32(emitter_behavior.initial_scale.0, emitter_behavior.initial_scale.1, &mut rng);
        let id = commands.spawn((
            Mesh3d::from(quad.clone()),
            MeshMaterial3d::from(materials.add(
                ExtendedMaterial {
                    base: StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture,
                    alpha_mode: AlphaMode::Blend,
                    ..Default::default()
                },
                extension: SoftParticleMaterialExtension { 
                        softness_factor: 12.0, 
                        wave_amplitude,
                        wave_frequency,
                        time: 0.0,
                    },
                })
            ),
            Transform::from_translation(translation).with_scale(Vec3::new(initial_scale, initial_scale, initial_scale)).with_rotation(camera_transform.rotation),
            NotShadowCaster,
            VfxBillboard,
        )).id();

        if emitter_behavior.receive_shadows {
            commands.entity(id).insert(NotShadowReceiver);
        }

        let mut initial_velocity = random_vec3(emitter_behavior.initial_velocity.0, emitter_behavior.initial_velocity.1, &mut rng);
        if initial_velocity.length() > 0.0 {
            commands.entity(id).insert(VfxVelocity(initial_velocity));
            let mut velocity_decay = random_f32(emitter_behavior.velocity_decay.0, emitter_behavior.velocity_decay.1, &mut rng);
            if velocity_decay > 0.0 {
                commands.entity(id).insert(VfxVelocityDecay(velocity_decay));
            }
        }

        let mut scale_velocity = random_f32(emitter_behavior.scale_velocity.0, emitter_behavior.scale_velocity.1, &mut rng);
        if scale_velocity > 0.0 {
            commands.entity(id).insert(VfxScaleVelocity(Vec3::new(scale_velocity, scale_velocity, scale_velocity)));
            let mut scale_velocity_decay = random_f32(emitter_behavior.scale_velocity_decay.0, emitter_behavior.scale_velocity_decay.1, &mut rng);
            if scale_velocity_decay > 0.0 {
                commands.entity(id).insert(VfxScaleVelocityDecay(scale_velocity_decay));
            }
        }

        let lifetime_millis = random_i32(emitter_behavior.lifetime_millis.0, emitter_behavior.lifetime_millis.1, &mut rng);
        if lifetime_millis > 0 {
            commands.entity(id).insert(VfxLifetime(Timer::new(Duration::from_millis(lifetime_millis as u64), TimerMode::Once)));
            if emitter_behavior.scale_over_lifetime.len() > 0 {
                let mut percent_scales = Vec::new();
                for percent_scale in emitter_behavior.scale_over_lifetime.iter() {
                    percent_scales.push(VfxPercentScale {
                        lifetime_percent: random_i32(percent_scale.lifetime_percent.0, percent_scale.lifetime_percent.1, &mut rng),
                        scale: random_vec3(percent_scale.scale.0, percent_scale.scale.1, &mut rng),
                    });
                }
                commands.entity(id).insert(VfxScaleOverLifetime {
                    percent_scales,
                });
            }

            if emitter_behavior.color_over_lifetime.len() > 0 {
                let mut percent_colors = Vec::new();
                for percent_color in emitter_behavior.color_over_lifetime.iter() {
                    percent_colors.push(VfxPercentColor {
                        lifetime_percent: random_i32(percent_color.lifetime_percent.0, percent_color.lifetime_percent.1, &mut rng),
                        color: random_color(percent_color.color.0, percent_color.color.1, &mut rng),
                    });
                }
                commands.entity(id).insert(VfxColorOverLifetime {
                    percent_colors,
                });
            }
        }

        if emitter_behavior.bounce_factor > 0.0 {
            commands.entity(id).insert(VfxBounce(emitter_behavior.bounce_factor));
        }
    }
}

fn random_i32(min: i32, max: i32, rng: &mut ThreadRng) -> i32 {
    if min >= max {
        return min;
    }
    rng.random_range(min..max)
}

fn random_f32(min: f32, max: f32, rng: &mut ThreadRng) -> f32 {
    if min >= max {
        return min;
    }
    rng.random_range(min..max)
}

fn random_vec3(min: Vec3, max: Vec3, rng: &mut ThreadRng) -> Vec3 {
    if min == max {
        return min;
    }
    
    Vec3::new(
        if min.x >= max.x { min.x } else { rng.random_range(min.x..max.x) },
        if min.y >= max.y { min.y } else { rng.random_range(min.y..max.y) },
        if min.z >= max.z { min.z } else { rng.random_range(min.z..max.z) },
    )
}

fn random_color(min: Color, max: Color, rng: &mut ThreadRng) -> Color {
    if min == max {
        return min;
    }
    let min = min.to_srgba();
    let max = max.to_srgba();
    Color::srgba(
        if min.red >= max.red { min.red } else { rng.random_range(min.red..max.red) },
        if min.green >= max.green { min.green } else { rng.random_range(min.green..max.green) },
        if min.blue >= max.blue { min.blue } else { rng.random_range(min.blue..max.blue) },
        if min.alpha >= max.alpha { min.alpha } else { rng.random_range(min.alpha..max.alpha) },
    )
}