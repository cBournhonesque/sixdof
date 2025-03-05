mod asset;
mod settings;

pub mod prelude {
    pub use crate::asset::*;
    pub use crate::settings::*;
    pub use crate::*;
    pub use kira;
}

use asset::{SfxAsset, SfxAssetLoader};
use bevy::{prelude::*, utils::{HashMap, HashSet}};
use kira::{
    effect::{delay::DelayBuilder, eq_filter::EqFilterBuilder, filter::{FilterBuilder, FilterHandle}, reverb::ReverbBuilder, EffectBuilder}, listener::ListenerHandle, sound::{static_sound::{StaticSoundData, StaticSoundHandle}, EndPosition, PlaybackPosition, PlaybackState, Region}, track::{self, SendTrackBuilder, SendTrackHandle, SpatialTrackBuilder, SpatialTrackHandle, TrackBuilder, TrackHandle, TrackPlaybackState}, AudioManager, AudioManagerSettings, Capacities, Decibels, DefaultBackend, Easing, Mapping, Mix, Tween, Value
};
use prelude::{DelaySettings, EqSettings, LowPassSettings, ReverbSettings};
use std::{hash::Hash, sync::Arc};
use std::time::Duration;

pub struct SfxAudioPlugin {
    sub_track_capacity: usize,
    send_track_capacity: usize,
}

impl Default for SfxAudioPlugin {
    fn default() -> Self {
        Self {
			sub_track_capacity: 2048,
			send_track_capacity: 2048,
        }
    }
}

impl Plugin for SfxAudioPlugin {
    fn build(&self, app: &mut App) {
        let mut settings = AudioManagerSettings {
            capacities: Capacities {
                sub_track_capacity: self.sub_track_capacity,
                send_track_capacity: self.send_track_capacity,
                ..default()
            },
            ..default()
        };

        match AudioManager::<DefaultBackend>::new(settings) {
            Ok(audio_manager) => {
                app.insert_resource(SfxManager {
                    audio_manager,
                    loaded_sfx: HashMap::default(),
                });

                app.init_asset::<SfxAsset>();
                app.init_asset_loader::<SfxAssetLoader>();
                
                app.add_systems(PostUpdate, (
                    add_listener_system,
                    play_sfx_event_system,
                ).chain());
        
                app.add_systems(PreUpdate, (
                    update_listener_system,
                    update_sfx_event_system,
                    cleanup_finished_sfx_system,
                    despawn_finished_sfx_system,
                ).chain());
            },
            Err(e) => {
                error!("Failed to create audio manager! Sound will not play: {}", e);
            }
        }
    }
}

#[derive(Resource)]
pub struct SfxManager {
    audio_manager: AudioManager<DefaultBackend>,
    loaded_sfx: HashMap<String, Handle<SfxAsset>>,
}

impl SfxManager {
    pub fn load_sfx(&mut self, unique_id: String, path: String, asset_server: &AssetServer) {
        let handle = asset_server.load(&path);
        self.loaded_sfx.insert(unique_id, handle);
    }
}

#[derive(Component)]
pub struct SfxListener {
    listener_handle: Option<ListenerHandle>,
}

impl SfxListener {
    pub fn new() -> Self {
        Self {
            listener_handle: None,
        }
    }
}

#[derive(Component)]
struct SfxDespawnTimer(Timer);

#[derive(Component)]
struct SfxEmitterHandles {
    track: Option<SpatialTrackHandle>,
    sound_handle: Option<StaticSoundHandle>,
    send_handles: Vec<SendTrackHandle>,
    filter_handles: Vec<FilterHandle>,
}

/// A setting to follow an entity.
pub struct Follow {
    /// The entity to follow.
    pub target: Entity,
    /// The offset from the target entity. This will be rotated by the target entity's rotation.
    pub local_offset: Vec3,
}

#[derive(Component)]
#[require(Transform)]
pub struct SfxSpatialEmitter {
    /// The unique id of the asset to play. Must be loaded using the `load_sfx` method in the `SfxManager` first
    pub asset_unique_id: String,
    /// The reverb settings for the sfx.
    pub reverb: Option<ReverbSettings>,
    /// The low pass filter settings for the sfx.
    pub low_pass: Option<LowPassSettings>,
    /// The eq settings for the sfx.
    pub eq: Option<EqSettings>,
    /// The volume of the sfx.
    pub volume: Value<Decibels>,
    /// The delay settings for the sfx.
    pub delay: Option<DelaySettings>,
    /// The region of the sound to loop.
    pub loop_region: Option<Region>,
    /// You can easily follow an entity to move the sfx with it.
    pub follow: Option<Follow>,
    /// After the sound has stopped, we will despawn recursively the entity 
    /// containing this component after this many seconds. If you're using
    /// reverb, you may want to increase this value to allow the reverb to play out.
    pub despawn_entity_after_secs: Option<f32>,
}

impl Default for SfxSpatialEmitter {
    fn default() -> Self {
        Self {
            asset_unique_id: "default".to_string(),
            reverb: None,
            low_pass: None,
            eq: None,
            delay: None,
            volume: Value::Fixed(Decibels(1.0)),
            loop_region: None,
            despawn_entity_after_secs: Some(4.0),
            follow: None,
        }
    }
}

enum TrackHandleKind {
    Spatial(SpatialTrackHandle),
    Flat(TrackHandle),
}

fn play_sfx_event_system(
    mut sfx_manager: ResMut<SfxManager>,
    mut commands: Commands,
    mut listener_query: Query<(Entity, &SfxListener)>,
    transforms: Query<&Transform>,
    mut assets: Res<Assets<SfxAsset>>,
    mut emitters: Query<(Entity, &mut SfxSpatialEmitter), Added<SfxSpatialEmitter>>,
) {
    for (listener_entity, listener) in listener_query.iter() {   
        if listener.listener_handle.is_none() {
            error!("Listener handle is None!");
            continue;
        }

        let listener_transform = transforms.get(listener_entity);
        if let Err(e) = listener_transform {
            error!("Error getting listener transform for entity {}: {}", listener_entity, e);
            continue;
        }

        let listener_transform = listener_transform.unwrap(); // SAFETY: We just checked if it's none
        
        for (emitter_entity, mut emitter) in emitters.iter_mut() {
            let emitter_transform = transforms.get(emitter_entity);

            if let Err(e) = emitter_transform {
                error!("Error getting emitter transform for entity {}: {}", emitter_entity, e);
                continue;
            }

            let emitter_transform = emitter_transform.unwrap(); // SAFETY: We just checked if it's none

            let sfx_asset_handle = if let Some(handle) = sfx_manager.loaded_sfx.get(&emitter.asset_unique_id) {
                handle.clone()
            } else {
                error!("Could not find sfx asset handle for: {}", emitter.asset_unique_id);
                continue;
            };

            let mut track_builder = SpatialTrackBuilder::default().doppler_effect(true).speed_of_sound(100.0);

            let mut emitter_handles = SfxEmitterHandles {
                track: None,
                sound_handle: None,
                send_handles: Vec::new(),
                filter_handles: Vec::new(),
            };

            if let Some(reverb_settings) = &emitter.reverb {
                if let Ok(reverb_send) = sfx_manager.audio_manager.add_send_track(
                    SendTrackBuilder::new().with_effect(ReverbBuilder::new()
                        .mix(reverb_settings.mix)
                        .damping(reverb_settings.damping)
                        .feedback(reverb_settings.feedback)
                    ),
                ) {
                    let volume = reverb_settings.volume.clone();
                    let reverb_id = reverb_send.id();
                    track_builder = track_builder.with_send(
                        reverb_id,
                        volume,
                    );
                    emitter_handles.send_handles.push(reverb_send);
                }
            }

            if let Some(eq_settings) = &emitter.eq {
                for frequency in eq_settings.frequencies.iter() {
                    if let Ok(eq_send) = sfx_manager.audio_manager.add_send_track(
                        SendTrackBuilder::new().with_effect(EqFilterBuilder::new(
                            frequency.kind,
                            frequency.frequency,
                            frequency.gain,
                            frequency.q,
                        ),
                    )) {
                        let eq_id = eq_send.id();
                        track_builder = track_builder.with_send(
                            eq_id,
                            Value::Fixed(Decibels(1.0)),
                        );
                        emitter_handles.send_handles.push(eq_send);
                    }
                }
            }

            if let Some(delay_settings) = &emitter.delay {
                if let Ok(delay_send) = sfx_manager.audio_manager.add_send_track(
                    SendTrackBuilder::new().with_effect(DelayBuilder::new()
                        .delay_time(delay_settings.delay_time)
                        .feedback(delay_settings.feedback)
                    ),
                ) {
                    let delay_id = delay_send.id();
                    track_builder = track_builder.with_send(
                        delay_id,
                        Value::Fixed(Decibels(1.0)),
                    );
                    emitter_handles.send_handles.push(delay_send);
                }
            }

            if let Some(low_pass_settings) = &emitter.low_pass {
                let cutoff_hz = low_pass_settings.cutoff_hz.clone();
                let mut filter_builder = FilterBuilder::new().cutoff(cutoff_hz);
                emitter_handles.filter_handles.push(track_builder.add_effect(filter_builder));
            }

            let initial_position = if let Some(follow) = &emitter.follow {
                if let Ok(target_transform) = transforms.get(follow.target) {
                    let rotated_offset = target_transform.rotation * follow.local_offset;
                    target_transform.translation + rotated_offset
                } else {
                    emitter_transform.translation
                }
            } else {
                emitter_transform.translation
            };

            let track_handle = sfx_manager.audio_manager.add_spatial_sub_track(
                listener.listener_handle.as_ref().unwrap().id(), // SAFETY: We we just checked if it's none at the beginning of the main loop.
                mint::Vector3 { 
                    x: initial_position.x,
                    y: initial_position.y,
                    z: initial_position.z
                },
                track_builder,
            );

            if let Ok(track_handle) = track_handle {
                info!("Created spatial track");
                emitter_handles.track = Some(track_handle);
                let loop_region = emitter.loop_region.clone();
                if let Some(spatial_track) = &mut emitter_handles.track {
                    if let Some(sfx_asset) = assets.get(&sfx_asset_handle) {
                        let mut sound_data = sfx_asset.sound_data.clone();
                        sound_data = sound_data.loop_region(loop_region);
                        if let Ok(handle) = spatial_track.play(sound_data) {
                            emitter_handles.sound_handle = Some(handle);
                            commands.entity(emitter_entity).insert(emitter_handles);
                            info!("Successfully started playing sfx");
                        } else {
                            error!("Error playing sfx");
                        }
                    } else {
                        error!("Could not find sfx asset data");
                    }
                }
            } else {
                error!("Failed to create spatial track");
            }
        }
    }
}

/// Updates the position of the emitter if it is a spatial track.
fn update_sfx_event_system(
    time: Res<Time>,
    mut sfx_manager: ResMut<SfxManager>,
    mut transforms: Query<&Transform>,
    mut query: Query<(Entity, &mut SfxSpatialEmitter, &mut SfxEmitterHandles)>,
) {
    for (emitter_entity, mut emitter, mut emitter_handles) in query.iter_mut() {
        if let Some(spatial_track_handle) = &mut emitter_handles.track {
            // Are we following a target?
            if let Some(follow) = &emitter.follow {
                if let Ok(target_transform) = transforms.get(follow.target) {
                    let rotated_offset = target_transform.rotation * follow.local_offset;
                    let position = target_transform.translation + rotated_offset;
                    spatial_track_handle.set_position(
                        mint::Vector3 { 
                            x: position.x, 
                            y: position.y, 
                            z: position.z 
                        },
                        Tween::default()
                    );
                }
            }
            // Otherwise, we are just using the emitter's transform.
            else {
                if let Ok(emitter_transform) = transforms.get(emitter_entity) {
                    spatial_track_handle.set_position(
                        mint::Vector3 { 
                            x: emitter_transform.translation.x, 
                            y: emitter_transform.translation.y, 
                            z: emitter_transform.translation.z 
                        },
                        Tween::default()
                    );
                }
            }
            spatial_track_handle.set_game_loop_delta_time(time.delta_secs_f64());
        }
    }
}

/// Adds a listener to the audio manager.
fn add_listener_system(
    mut sfx_manager: ResMut<SfxManager>,
    mut listener_query: Query<(Entity, &Transform, &mut SfxListener), Added<SfxListener>>,
) {
    for (listener, listener_transform, mut sfx_listener) in listener_query.iter_mut() {
        info!("Adding listener at position: {:?}", listener_transform.translation);
        if let Ok(kira_listener) = sfx_manager.audio_manager.add_listener(
            mint::Vector3 { 
                x: listener_transform.translation.x, 
                y: listener_transform.translation.y, 
                z: listener_transform.translation.z 
            }, 
            mint::Quaternion { 
                v: mint::Vector3 { 
                    x: listener_transform.rotation.x, 
                    y: listener_transform.rotation.y, 
                    z: listener_transform.rotation.z 
                }, 
                s: listener_transform.rotation.w 
            }
        ) {
            info!("Successfully added Kira listener");
            sfx_listener.listener_handle = Some(kira_listener);
        } else {
            error!("Failed to add Kira listener");
        }
    }
}

/// Updates the listener's position and orientation.
fn update_listener_system(
    time: Res<Time>,
    mut sfx_manager: ResMut<SfxManager>,
    mut listener_query: Query<(Entity, &Transform, &mut SfxListener), Changed<Transform>>,
) {
    for (listener, listener_transform, mut sfx_listener) in listener_query.iter_mut() {
        if let Some(mut kira_listener) = sfx_listener.listener_handle.as_mut() {
            kira_listener.set_position(
                mint::Vector3 { 
                    x: listener_transform.translation.x, 
                    y: listener_transform.translation.y, 
                    z: listener_transform.translation.z 
                },
                Tween::default()
            );
            kira_listener.set_orientation(
                mint::Quaternion { 
                    v: mint::Vector3 { 
                        x: listener_transform.rotation.x, 
                        y: listener_transform.rotation.y, 
                        z: listener_transform.rotation.z 
                    },
                    s: listener_transform.rotation.w
                },
                Tween::default()
            );
            kira_listener.set_game_loop_delta_time(time.delta_secs_f64());
        }
    }
}

/// Cleans up finished sfx.
fn cleanup_finished_sfx_system(
    mut commands: Commands,
    despawn_timer: Query<&mut SfxDespawnTimer>,
    query: Query<(Entity, &SfxSpatialEmitter, &SfxEmitterHandles)>,
    time: Res<Time>,
) {
    for (entity, emitter, emitter_handles) in query.iter() {
        if let Some(sound_handle) = &emitter_handles.sound_handle {
            if matches!(sound_handle.state(), PlaybackState::Stopped) {

                // Don't despawn if the reverb frequency is 1.0, 
                // because that means the reverb is going forever.
                if let Some(reverb_settings) = &emitter.reverb {
                    if reverb_settings.feedback >= 1.0 {
                        continue;
                    }
                }

                if let Some(despawn_delay) = emitter.despawn_entity_after_secs {
                    if despawn_timer.get(entity).is_err() {
                        commands.entity(entity).insert(SfxDespawnTimer(Timer::new(
                            Duration::from_secs_f32(despawn_delay),
                            TimerMode::Once,
                        )));
                    }
                } else {
                    commands.entity(entity).remove::<SfxSpatialEmitter>();
                    info!("Removed emitter component from entity {}", entity);
                }
            }
        }
    }
}

fn despawn_finished_sfx_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut SfxDespawnTimer, &mut SfxSpatialEmitter)>,
) {
    for (entity, mut timer, mut emitter) in query.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            commands.entity(entity).despawn_recursive();
            info!("Removed emitter component from entity {} after {} seconds", entity, timer.0.elapsed_secs());
        }
    }
}

