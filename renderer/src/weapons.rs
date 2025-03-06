use audio::prelude::kira::effect::eq_filter::EqFilterKind;
use audio::prelude::kira::track::SpatialTrackDistances;
use audio::prelude::kira::{Decibels, Easing, Mapping, Mix, Value};
use audio::prelude::{EqFrequency, EqSettings, LowPassSettings, ReverbSettings, SfxFollowTarget, SfxSpatialEmitter};
use audio::SfxManager;
use avian3d::prelude::{Collider, LinearVelocity, PhysicsSet, Position, Rotation, SpatialQuery, SpatialQueryFilter};
use bevy::color::palettes::basic::{BLUE, YELLOW};
use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use bevy::utils::Duration;
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
use shared::prelude::{DespawnAfter, LinearProjectile, PlayerInput, ReverbMix, UniqueIdentity};
use shared::weapons::*;

pub(crate) struct WeaponsPlugin;

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_projectile_visuals);
        app.add_systems(Update, load_weapon_sounds_system.run_if(resource_exists::<WeaponsData>));
        app.add_systems(Update, weapon_fired_system.run_if(resource_exists::<WeaponsData>));
    }
}

/// When a projectile is spawn, add visuals to it
fn spawn_projectile_visuals(
    trigger: Trigger<OnAdd, Projectile>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.entity(trigger.entity()).insert(
        (
            Visibility::default(),
            Mesh3d(meshes.add(Mesh::from(Sphere {
                // TODO: must match the collider size
                //      @todo-brian-reply: nah, its common for games to have a visual size that 
                //      doesn't match the collider size, infact we should probably stick to 
                //      simple Point based collision (ray cast) for projectiles, unless 
                //      they are much larger than a typiical projectile, since there's going 
                //      to be a lot flying at once.
                radius: 0.05,
                ..default()
            }))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: BLUE.into(),
                ..Default::default()
            })),
            //VisualInterpolateStatus::<Transform>::default(),
            NotShadowCaster,
        )
    );
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

            // If it's our own weapon we dont use doppler
            let doppler_enabled = if controlled.get(event.shooter_entity).is_ok() {
                false
            } else {
                true
            };

            println!("doppler_enabled: {}", doppler_enabled);

            // Spawn the fire sound
            // @todo-brian: We probably want to tweak things based on if the shooter is the local player or not.
            commands.spawn((
                SfxSpatialEmitter {
                    // The unique id is the asset path of the fire sound
                    asset_unique_id: weapon.firing_sound.compute_asset_path(),
                    distances: SpatialTrackDistances {
                        min_distance: weapon.firing_sound.min_distance,
                        max_distance: weapon.firing_sound.max_distance,
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
                    doppler_enabled,
                    speed_of_sound: weapon.firing_sound.speed_of_sound as f64,
                    volume: Value::Fixed(Decibels(weapon.firing_sound.volume_db)),
                    loop_region: None,
                    despawn_entity_after_secs: weapon.firing_sound.despawn_delay,
                    follow: Some(SfxFollowTarget {
                        target: event.shooter_entity,
                        local_offset: Vec3::ZERO,
                    }),
                    ..default()
                },
                Transform::from_translation(event.fire_origin),
            ));
        }
    }
}
