use sfx::prelude::kira::effect::eq_filter::EqFilterKind;
use sfx::prelude::kira::track::SpatialTrackDistances;
use sfx::prelude::kira::{Decibels, Easing, Mapping, Mix, Value};
use sfx::prelude::{EqFrequency, EqSettings, LowPassSettings, ReverbSettings, SfxFollowTarget, SfxEmitter};
use sfx::SfxManager;
use avian3d::prelude::{Collider, LinearVelocity, PhysicsSet, Position, Rotation, SpatialQuery, SpatialQueryFilter};
use bevy::color::palettes::basic::{BLUE, YELLOW};
use bevy::pbr::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::utils::{Duration, HashMap};
use bevy_config_stack::prelude::ConfigAssetLoadedEvent;
use leafwing_input_manager::prelude::ActionState;
use lightyear::client::prediction::Predicted;
use lightyear::prelude::client::{Interpolated, VisualInterpolateStatus};
use lightyear::prelude::{ClientId, Replicating};
use lightyear::shared::replication::components::Controlled;
use rand::Rng;
use shared::bot::Bot;
use shared::physics::GameLayer;
use shared::player::{self, Player};
use shared::prelude::{DespawnAfter, PlayerInput, ProjectileVisuals, ReverbMix, UniqueIdentity};
use shared::weapons::*;
use vfx::VfxBillboard;

pub(crate) struct WeaponsPlugin;

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProjectileVisualsCache>();
        app.add_observer(spawn_projectile_visuals_observer);
        app.add_systems(Startup, setup_projectile_visuals_cache_system);
        app.add_systems(Update, load_weapon_sounds_system.run_if(resource_exists::<WeaponsData>));
        app.add_systems(Update, weapon_fired_system.run_if(resource_exists::<WeaponsData>));
    }
}

#[derive(Resource, Default)]
struct ProjectileVisualsCache {
    quad: Option<Handle<Mesh>>,
    textures: HashMap<String, Handle<Image>>,
    meshes: HashMap<String, Handle<Mesh>>,
}

fn setup_projectile_visuals_cache_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<ProjectileVisualsCache>,
) {
    let quad = meshes.add(Mesh::from(Rectangle::default()));
    cache.quad = Some(quad);
}

/// When a projectile is spawn, add visuals to it
fn spawn_projectile_visuals_observer(
    trigger: Trigger<OnAdd, Projectile>,
    weapons_data: Option<Res<WeaponsData>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<ProjectileVisualsCache>,
    mut projectile: Query<(&mut Transform, &WeaponFiredEvent), With<Projectile>>,
    mut asset_server: ResMut<AssetServer>,
) {
    let entity = trigger.entity();

    let weapons_data = if let Some(weapons_data) = weapons_data { 
        weapons_data 
    } else {
        error!("No weapons data found, skipping projectile visuals");
        return; 
    };

    let (mut projectile_transform, projectile) = if let Ok(projectile) = projectile.get_mut(entity) {
        projectile
    } else {
        error!("No projectile found for entity: {}", entity);
        return;
    };

    let weapon_id = projectile.weapon_index;
    let weapon = if let Some(data) = weapons_data.weapons.get(&weapon_id) {
        data
    } else {
        error!("No data found for weapon id: {}", weapon_id);
        return;
    };

    match &weapon.projectile_visuals {
        ProjectileVisuals::Sprite { texture_asset_path, base_color, emissive_color, light_color, scale } => {

            let quad = if let Some(quad) = cache.quad.clone() {
                quad
            } else {
                error!("No quad found in cache, skipping projectile visuals");
                return;
            };

            let mut needs_insert = false;
            if !cache.textures.contains_key(texture_asset_path) {
                needs_insert = true;
            }

            if needs_insert {
                cache.textures.insert(texture_asset_path.clone(), asset_server.load(format!("textures/{}", texture_asset_path)));
            }

            if let Some(texture) = cache.textures.get(texture_asset_path) {
                projectile_transform.scale = Vec3::splat(*scale);
                commands.entity(entity).insert((
                    InheritedVisibility::default(),
                    Visibility::default(),
                    // TODO: this is only necessary for Predicted projectiles, not interpolated ones!
                    VisualInterpolateStatus::<Transform>::default(),
                    Mesh3d(quad.clone()),
                    // @todo-brian: We should probably have a separate material for projectiles, so we're not using PBR.
                    // But at least right now they get affected by fog and stuff, so maybe it's fine.
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: *base_color,
                        base_color_texture: Some(texture.clone()),
                        emissive: if let Some(emissive_color) = emissive_color {
                            (*emissive_color).into()
                        } else {
                            LinearRgba::BLACK
                        },
                        alpha_mode: AlphaMode::Blend,
                        ..Default::default()
                    })),
                    NotShadowReceiver,
                    NotShadowCaster,
                    VfxBillboard,
                ));

                if let Some(light_color) = light_color {
                    commands.entity(entity).insert(PointLight {
                        color: *light_color,
                        intensity: 10000.0,
                        // range: 100.0,
                        // radius: 0.1,
                        shadows_enabled: false,
                        ..default()
                    });
                }
            } else {
                error!("Failed to load texture for projectile visuals: {}", texture_asset_path);
            }
        }
        ProjectileVisuals::Mesh { mesh_asset_path, base_color_texture_path, scale } => {
            unimplemented!();
        }
    }
}

/// Whenever weapon data is loaded/reloaded we load/reload the weapon sounds
fn load_weapon_sounds_system(
    mut commands: Commands,
    mut sfx_manager: ResMut<SfxManager>,
    asset_server: Res<AssetServer>,
    weapons_data: Res<WeaponsData>,
    mut events: EventReader<ConfigAssetLoadedEvent<WeaponsData>>,
) {
    for _ in events.read() {
        for (weapon_idx, weapon) in weapons_data.weapons.iter() {
            // Load fire sounds:
            // We treat the sound path as the unique id.
            let path = weapon.firing_sound.compute_asset_path();
            sfx_manager.load_sfx(path.clone(), path, &asset_server);
        }
    }
}

/// When a weapon is fired, spawn the fire sound, muzzle flash and any vfx.
fn weapon_fired_system(
    weapons_data: Res<WeaponsData>,
    mut commands: Commands,
    mut events: EventReader<WeaponFiredEvent>,
    controlled: Query<(), With<Controlled>>,
) {
    for event in events.read() {
        if let Some(weapon) = weapons_data.weapons.get(&event.weapon_index) {

            // If it's our own weapon we want to make sure we use a 2D (but still stereo) sound.
            let is_controlled = if controlled.get(event.shooter_entity).is_ok() {
                true
            } else {
                false
            };

            // Spawn the fire sound
            // @todo-brian: We probably want to tweak things based on if the shooter is the local player or not.
            commands.spawn((
                SfxEmitter {
                    // The unique id is the asset path of the fire sound
                    asset_unique_id: weapon.firing_sound.compute_asset_path(),
                    spatial: if is_controlled {
                        None
                    } else {
                        Some(SpatialTrackDistances {
                            min_distance: weapon.firing_sound.min_distance,
                            max_distance: weapon.firing_sound.max_distance,
                        })
                    },
                    reverb: {
                        if let Some(reverb) = &weapon.firing_sound.reverb {
                            Some(ReverbSettings {
                                damping: reverb.damping as f64,
                                feedback: reverb.feedback as f64,
                                mix: {
                                    if reverb.mix == ReverbMix::Wet {
                                        Mix::WET
                                    } else {
                                        Mix::DRY
                                    }
                                },
                                volume: Value::Fixed(Decibels(1.0)),
                            })
                        } else {
                            None
                        }
                    },
                    low_pass: {
                        // Do not distance muffle for controlled weapons
                        if is_controlled {
                            None
                        } else {
                            if let Some(distance_muffle) = &weapon.firing_sound.distance_muffle {
                                Some(LowPassSettings {
                                    cutoff_hz: Value::FromListenerDistance(Mapping {
                                        input_range: (distance_muffle.min_distance as f64, distance_muffle.max_distance as f64),
                                        output_range: (20000.0, distance_muffle.cutoff_hz as f64),
                                        easing: Easing::Linear,
                                    })
                                })
                            } else {
                                None
                            }
                        }
                    },
                    eq: {
                        if let Some(eq_variance) = &weapon.firing_sound.eq_variance {

                            let mut rng = rand::rng();

                            let low_gain = if eq_variance.low_min_db >= eq_variance.low_max_db {
                                eq_variance.low_min_db
                            } else {
                                rng.random_range(eq_variance.low_min_db..eq_variance.low_max_db)
                            };

                            let mid_gain = if eq_variance.mid_min_db >= eq_variance.mid_max_db {
                                eq_variance.mid_min_db
                            } else {
                                rng.random_range(eq_variance.mid_min_db..eq_variance.mid_max_db)
                            };

                            let high_gain = if eq_variance.high_min_db >= eq_variance.high_max_db {
                                eq_variance.high_min_db
                            } else {
                                rng.random_range(eq_variance.high_min_db..eq_variance.high_max_db)
                            };

                            Some(EqSettings {
                                frequencies: vec![
                                    EqFrequency { kind: EqFilterKind::Bell, frequency: 200.0, gain: Value::Fixed(Decibels(low_gain)), q: 1.0 },
                                    EqFrequency { kind: EqFilterKind::Bell, frequency: 2000.0, gain: Value::Fixed(Decibels(mid_gain)), q: 1.0 },
                                    EqFrequency { kind: EqFilterKind::Bell, frequency: 20000.0, gain: Value::Fixed(Decibels(high_gain)), q: 1.0 },
                                ],
                            })
                        } else {
                            None
                        }
                    },
                    delay: None,
                    // No doppler for controlled weapons
                    doppler_enabled: is_controlled,
                    speed_of_sound: weapon.firing_sound.speed_of_sound as f64,
                    volume: Value::Fixed(Decibels(weapon.firing_sound.volume_db)),
                    loop_region: None,
                    despawn_entity_after_secs: weapon.firing_sound.despawn_delay,
                    // No follow for controlled weapons
                    follow: if is_controlled {
                        None
                    } else {
                        Some(SfxFollowTarget {
                            target: event.shooter_entity,
                            local_offset: Vec3::ZERO,
                        })
                    },
                    ..default()
                },
                Transform::from_translation(event.fire_origin),
            ));
        }
    }
}
