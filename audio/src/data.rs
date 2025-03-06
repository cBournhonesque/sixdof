use std::time::Duration;

use bevy::prelude::*;
use kira::effect::eq_filter::EqFilterKind;
use kira::effect::filter::FilterHandle;
use kira::sound::static_sound::StaticSoundHandle;
use kira::track::SendTrackHandle;
use kira::{Decibels, Easing, Mapping, Value};
use kira::{listener::ListenerHandle, sound::Region, track::{SpatialTrackDistances, SpatialTrackHandle, TrackHandle}};

#[derive(Component)]
pub struct SfxListener {
    pub(crate) listener_handle: Option<ListenerHandle>,
}

impl SfxListener {
    pub fn new() -> Self {
        Self {
            listener_handle: None,
        }
    }
}

/// A setting to follow an entity.
pub struct SfxFollowTarget {
    /// The entity to follow.
    pub target: Entity,
    /// The offset from the target entity. This will be rotated by the target entity's rotation.
    pub local_offset: Vec3,
}

impl Default for SfxFollowTarget {
    fn default() -> Self {
        Self {
            target: Entity::PLACEHOLDER,
            local_offset: Vec3::ZERO,
        }
    }
}

#[derive(Component)]
#[require(Transform)]
pub struct SfxSpatialEmitter {
    /// The unique id of the asset to play. Must be loaded using the `load_sfx` method in the `SfxManager` first
    pub asset_unique_id: String,
    /// Sets the distances from a listener at which the emitter is loudest and quietest.
    pub distances: SpatialTrackDistances,
    /// The reverb settings for the sfx.
    pub reverb: Option<ReverbSettings>,
    /// The low pass filter settings for the sfx.
    pub low_pass: Option<LowPassSettings>,
    /// The eq settings for the sfx.
    pub eq: Option<EqSettings>,
    /// Whether the doppler effect is enabled for the sfx.
    pub doppler_enabled: bool,
    /// The speed of sound, used for the doppler effect.
    pub speed_of_sound: f64,
    /// The volume of the sfx.
    pub volume: Value<Decibels>,
    /// The delay settings for the sfx.
    pub delay: Option<DelaySettings>,
    /// The region of the sound to loop.
    pub loop_region: Option<Region>,
    /// You can easily follow an entity to move the sfx with it.
    pub follow: Option<SfxFollowTarget>,
    /// After the sound has stopped, we will despawn recursively the entity 
    /// containing this component after this many seconds. If you're using
    /// reverb, you may want to increase this value to allow the reverb to play out.
    pub despawn_entity_after_secs: Option<f32>,
}

impl Default for SfxSpatialEmitter {
    fn default() -> Self {
        Self {
            asset_unique_id: "default".to_string(),
            distances: SpatialTrackDistances::default(),
            reverb: None,
            low_pass: None,
            eq: None,
            doppler_enabled: true,
            speed_of_sound: 343.0,
            volume: Value::Fixed(Decibels(1.0)),
            delay: None,
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

#[derive(Debug, Clone)]
pub struct ReverbSettings {
    pub damping: f64,
    pub feedback: f64,
    pub mix: kira::Mix,
    pub volume: Value<Decibels>,
}

impl Default for ReverbSettings {
    fn default() -> Self {
        Self {
            damping: 0.5,
            feedback: 0.5,
            mix: kira::Mix::WET,
            volume: Value::FromListenerDistance(Mapping {
                input_range: (0.0, 100.0),
                output_range: (Decibels(6.0), Decibels(40.0)),
                easing: Easing::Linear,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LowPassSettings {
    pub cutoff_hz: Value<f64>,
}

impl Default for LowPassSettings {
    fn default() -> Self {
        Self {
            cutoff_hz: Value::FromListenerDistance(Mapping {
                input_range: (1.0, 50.0),
                output_range: (20000.0, 500.0),
                easing: Easing::Linear,
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct EqFrequency {
    pub kind: EqFilterKind,
    pub frequency: f64,
    pub gain: Value<Decibels>,
    pub q: f64,
}

#[derive(Debug, Clone)]
pub struct EqSettings {
    pub frequencies: Vec<EqFrequency>,
}

impl Default for EqSettings {
    fn default() -> Self {
        Self { 
            frequencies: vec![
                EqFrequency { kind: EqFilterKind::Bell, frequency: 100.0, gain: Value::Fixed(Decibels(0.0)), q: 1.0 },
                EqFrequency { kind: EqFilterKind::Bell, frequency: 1000.0, gain: Value::Fixed(Decibels(0.0)), q: 1.0 },
                EqFrequency { kind: EqFilterKind::Bell, frequency: 10000.0, gain: Value::Fixed(Decibels(0.0)), q: 1.0 },
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct DelaySettings {
    pub delay_time: Duration,
    pub feedback: Value<Decibels>,
}

impl Default for DelaySettings {
    fn default() -> Self {
        Self { delay_time: Duration::from_secs(1), feedback: Value::Fixed(Decibels(0.0)) }
    }
}
