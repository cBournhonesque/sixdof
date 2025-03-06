mod asset;
mod data;

pub mod prelude {
    pub use crate::asset::*;
    pub use crate::data::*;
    pub use crate::*;
    pub use kira;
}

use asset::{SfxAsset, SfxAssetLoader};
use bevy::{prelude::*, utils::{HashMap, HashSet}};
use kira::{
    effect::{delay::DelayBuilder, eq_filter::EqFilterBuilder, filter::{FilterBuilder, FilterHandle}, reverb::ReverbBuilder, EffectBuilder}, listener::ListenerHandle, sound::{static_sound::{StaticSoundData, StaticSoundHandle}, EndPosition, PlaybackPosition, PlaybackState, Region}, track::{self, SendTrackBuilder, SendTrackHandle, SpatialTrackBuilder, SpatialTrackDistances, SpatialTrackHandle, TrackBuilder, TrackHandle, TrackPlaybackState}, AudioManager, AudioManagerSettings, Capacities, Decibels, DefaultBackend, Easing, Mapping, Mix, Tween, Value
};
use prelude::{DelaySettings, EqSettings, LowPassSettings, ReverbSettings};
use std::{hash::Hash, sync::Arc};
use std::time::Duration;

use crate::data::*;

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
        
                app.add_systems(PostUpdate, (
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
        if self.loaded_sfx.contains_key(&unique_id) {
            // already loaded, don't load again.
            return;
        }

        let handle = asset_server.load(&path);
        self.loaded_sfx.insert(unique_id, handle);
    }
}

#[derive(Component)]
struct SfxEmitterHandles {
    track: Option<TrackHandleKind>,
    sound_handle: Option<StaticSoundHandle>,
    send_handles: Vec<SendTrackHandle>,
    filter_handles: Vec<FilterHandle>,
}

#[derive(Component)]
struct SfxDespawnTimer(Timer);

enum TrackKind {
    Spatial(SpatialTrackBuilder),
    Flat(TrackBuilder),
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
    mut emitters: Query<(Entity, &mut SfxEmitter), Added<SfxEmitter>>,
) {
    for (listener_entity, listener) in listener_query.iter() {   
        if listener.listener_handle.is_none() {
            error!("Listener handle is None!");
            continue;
        }

        let listener_transform = match transforms.get(listener_entity) {
            Ok(transform) => transform,
            Err(e) => {
                error!("Error getting listener transform: {}", e);
                continue;
            }
        };
        
        for (emitter_entity, emitter) in emitters.iter() {
            let emitter_transform = match transforms.get(emitter_entity) {
                Ok(transform) => transform,
                Err(e) => {
                    error!("Error getting emitter transform: {}", e);
                    continue;
                }
            };

            let sfx_asset_handle = match sfx_manager.loaded_sfx.get(&emitter.asset_unique_id) {
                Some(handle) => handle.clone(),
                None => {
                    error!("Could not find sfx asset handle");
                    continue;
                }
            };

            let mut emitter_handles = SfxEmitterHandles {
                track: None,
                sound_handle: None,
                send_handles: Vec::new(),
                filter_handles: Vec::new(),
            };

            if let Some(spatial_distances) = &emitter.spatial {
                // Handle spatial audio
                let mut builder = SpatialTrackBuilder::default()
                    .distances(*spatial_distances)
                    .doppler_effect(emitter.doppler_enabled)
                    .speed_of_sound(emitter.speed_of_sound);

                // Add reverb if configured
                if let Some(reverb_settings) = &emitter.reverb {
                    if let Ok(reverb_send) = sfx_manager.audio_manager.add_send_track(
                        SendTrackBuilder::new().with_effect(ReverbBuilder::new()
                            .mix(reverb_settings.mix)
                            .damping(reverb_settings.damping)
                            .feedback(reverb_settings.feedback)
                        ),
                    ) {
                        builder = builder.with_send(reverb_send.id(), reverb_settings.volume.clone());
                        emitter_handles.send_handles.push(reverb_send);
                    }
                }

                // Add EQ if configured
                if let Some(eq_settings) = &emitter.eq {
                    for frequency in eq_settings.frequencies.iter() {
                        if let Ok(eq_send) = sfx_manager.audio_manager.add_send_track(
                            SendTrackBuilder::new().with_effect(EqFilterBuilder::new(
                                frequency.kind,
                                frequency.frequency,
                                frequency.gain.clone(),
                                frequency.q,
                            )),
                        ) {
                            builder = builder.with_send(eq_send.id(), Value::Fixed(Decibels(1.0)));
                            emitter_handles.send_handles.push(eq_send);
                        }
                    }
                }

                // Add delay if configured
                if let Some(delay_settings) = &emitter.delay {
                    if let Ok(delay_send) = sfx_manager.audio_manager.add_send_track(
                        SendTrackBuilder::new().with_effect(DelayBuilder::new()
                            .delay_time(delay_settings.delay_time)
                            .feedback(delay_settings.feedback)
                        ),
                    ) {
                        builder = builder.with_send(delay_send.id(), Value::Fixed(Decibels(1.0)));
                        emitter_handles.send_handles.push(delay_send);
                    }
                }

                // Add low pass filter if configured
                if let Some(low_pass_settings) = &emitter.low_pass {
                    builder = builder.with_effect(FilterBuilder::new().cutoff(low_pass_settings.cutoff_hz.clone()));
                }

                // Calculate initial position
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

                // Create spatial track
                if let Ok(spatial_track) = sfx_manager.audio_manager.add_spatial_sub_track(
                    listener.listener_handle.as_ref().unwrap().id(),
                    mint::Vector3::from([initial_position.x, initial_position.y, initial_position.z]),
                    builder,
                ) {
                    emitter_handles.track = Some(TrackHandleKind::Spatial(spatial_track));
                }

            } else {
                // Handle flat audio
                let mut builder = TrackBuilder::default();

                // Add reverb if configured
                if let Some(reverb_settings) = &emitter.reverb {
                    if let Ok(reverb_send) = sfx_manager.audio_manager.add_send_track(
                        SendTrackBuilder::new().with_effect(ReverbBuilder::new()
                            .mix(reverb_settings.mix)
                            .damping(reverb_settings.damping)
                            .feedback(reverb_settings.feedback)
                        ),
                    ) {
                        builder = builder.with_send(reverb_send.id(), reverb_settings.volume.clone());
                        emitter_handles.send_handles.push(reverb_send);
                    }
                }

                // Add EQ if configured
                if let Some(eq_settings) = &emitter.eq {
                    for frequency in eq_settings.frequencies.iter() {
                        if let Ok(eq_send) = sfx_manager.audio_manager.add_send_track(
                            SendTrackBuilder::new().with_effect(EqFilterBuilder::new(
                                frequency.kind,
                                frequency.frequency,
                                frequency.gain.clone(),
                                frequency.q,
                            )),
                        ) {
                            builder = builder.with_send(eq_send.id(), Value::Fixed(Decibels(1.0)));
                            emitter_handles.send_handles.push(eq_send);
                        }
                    }
                }

                // Add delay if configured
                if let Some(delay_settings) = &emitter.delay {
                    if let Ok(delay_send) = sfx_manager.audio_manager.add_send_track(
                        SendTrackBuilder::new().with_effect(DelayBuilder::new()
                            .delay_time(delay_settings.delay_time)
                            .feedback(delay_settings.feedback)
                        ),
                    ) {
                        builder = builder.with_send(delay_send.id(), Value::Fixed(Decibels(1.0)));
                        emitter_handles.send_handles.push(delay_send);
                    }
                }

                // Add low pass filter if configured
                if let Some(low_pass_settings) = &emitter.low_pass {
                    builder = builder.with_effect(FilterBuilder::new().cutoff(low_pass_settings.cutoff_hz.clone()));
                }

                // Create flat track
                if let Ok(flat_track) = sfx_manager.audio_manager.add_sub_track(builder) {
                    emitter_handles.track = Some(TrackHandleKind::Flat(flat_track));
                }
            }

            // Play sound on the created track
            if let Some(track) = &mut emitter_handles.track {
                if let Some(sfx_asset) = assets.get(&sfx_asset_handle) {
                    let mut sound_data = sfx_asset.sound_data.clone();
                    if let Some(loop_region) = emitter.loop_region.clone() {
                        sound_data = sound_data.loop_region(loop_region);
                    }

                    let play_result = match track {
                        TrackHandleKind::Spatial(spatial_track) => spatial_track.play(sound_data),
                        TrackHandleKind::Flat(flat_track) => flat_track.play(sound_data),
                    };

                    match play_result {
                        Ok(handle) => {
                            emitter_handles.sound_handle = Some(handle);
                            commands.entity(emitter_entity).insert(emitter_handles);
                            info!("Successfully started playing sfx");
                        }
                        Err(e) => error!("Error playing sfx: {}", e),
                    }
                }
            }
        }
    }
}

/// Updates the position of the emitter if it is a spatial track.
fn update_sfx_event_system(
    time: Res<Time>,
    transforms: Query<&Transform>,
    mut query: Query<(Entity, &SfxEmitter, &mut SfxEmitterHandles)>,
) {
    for (emitter_entity, emitter, mut emitter_handles) in query.iter_mut() {
        if let Some(track_handle) = &mut emitter_handles.track {
            match track_handle {
                TrackHandleKind::Spatial(spatial_track) => {
                    let position = if let Some(follow) = &emitter.follow {
                        if let Ok(target_transform) = transforms.get(follow.target) {
                            let rotated_offset = target_transform.rotation * follow.local_offset;
                            target_transform.translation + rotated_offset
                        } else {
                            continue;
                        }
                    } else if let Ok(emitter_transform) = transforms.get(emitter_entity) {
                        emitter_transform.translation
                    } else {
                        continue;
                    };

                    spatial_track.set_position(
                        mint::Vector3::from([position.x, position.y, position.z]),
                        Tween::default()
                    );
                    spatial_track.set_game_loop_delta_time(time.delta_secs_f64());
                }
                TrackHandleKind::Flat(_) => {}
            }
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
    query: Query<(Entity, &SfxEmitter, &SfxEmitterHandles)>,
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
                    commands.entity(entity).remove::<SfxEmitter>();
                    info!("Removed emitter component from entity {}", entity);
                }
            }
        }
    }
}

fn despawn_finished_sfx_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut SfxDespawnTimer, &mut SfxEmitter)>,
) {
    for (entity, mut timer, mut emitter) in query.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            commands.entity(entity).despawn_recursive();
            info!("Removed emitter component from entity {} after {} seconds", entity, timer.0.elapsed_secs());
        }
    }
}

