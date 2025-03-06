use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// The wetness of the reverb, this is the style of which the reverb is applied.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReverbMix {
    Wet,
    Dry,
}

/// To add ambience, we add a bit of reverb to the sound, this gives the illusion of larger spaces.
#[derive(Debug, Serialize, Deserialize)]
pub struct ReverbBehavior {
    pub damping: f32,
    pub feedback: f32,
    pub mix: ReverbMix,
}

/// To add ambience, we muffle sounds as they get further away from the listener to simulate atmosphere.
#[derive(Debug, Serialize, Deserialize)]
pub struct DistanceMuffleBehavior {
    /// Distance at which the sound starts to muffle in meters.
    pub min_distance: f32,

    /// Distance at which the sound is fully muffled in meters.
    pub max_distance: f32,

    /// Frequency at which the sound is fully muffled at the max distance.
    pub cutoff_hz: f32,
}

/// Configuration for a sound emitter:
/// 
/// High level configuration for a sound emitter, limited in options because 
/// we want a simple interface and keep sounds within a common theme.
/// Also, Kira doesn't have serde support, so we need to wrap the settings anyway.
#[derive(Asset, TypePath, Debug, Serialize, Deserialize)]
pub struct SoundEmitterBehavior {
    /// Path to the sound asset. Relative to assets/audio/
    pub asset_path: String,

    /// Volume of the sound at it's source in decibels.
    pub volume_db: f32,

    /// Distance at which the sound starts to fade out in meters.
    pub min_distance: f32,

    /// Distance at which the sound is at zero volume in meters.
    pub max_distance: f32,

    /// Reverb to add ambience to the sound.
    pub reverb: Option<ReverbBehavior>,

    /// Muffles the sound as it gets further away from the listener.
    pub distance_muffle: Option<DistanceMuffleBehavior>,

    /// Speed of sound, used for doppler effect.
    pub speed_of_sound: f32,

    /// The delay after the sound is finished playing before the sound is despawned.
    /// If you're using reverb, you may want to increase this value to allow the reverb to play out.
    pub despawn_delay: Option<f32>,

    /// Vary the eq of the sound between the min and max. 
    /// This helps keep the sound from sounding repetitive, for example weapons with high rates of fire.
    pub eq_variance: Option<EqVarianceBehavior>,
}

impl Default for SoundEmitterBehavior {
    fn default() -> Self {
        Self {
            asset_path: "".to_string(),
            volume_db: 1.0,
            min_distance: 0.0,
            max_distance: 100.0,
            reverb: Some(ReverbBehavior {
                damping: 0.5,
                feedback: 0.5,
                mix: ReverbMix::Wet,
            }),
            distance_muffle: Some(DistanceMuffleBehavior {
                min_distance: 0.0,
                max_distance: 100.0,
                cutoff_hz: 1000.0,
            }),
            speed_of_sound: 343.0,
            despawn_delay: Some(4.0),
            eq_variance: None,
        }
    }
}

impl SoundEmitterBehavior {
    pub fn compute_asset_path(&self) -> String {
        format!("audio/{}", self.asset_path)
    }
}

/// Configuration for the eq variance of a sound emitter.
#[derive(Debug, Serialize, Deserialize)]
pub struct EqVarianceBehavior {
    /// The minimum gain (db) for the low eq band.
    pub low_min_db: f32,

    /// The maximum gain (db) for the low eq band.
    pub low_max_db: f32,

    /// The minimum gain (db) for the mid eq band.
    pub mid_min_db: f32,

    /// The maximum gain (db) for the mid eq band.
    pub mid_max_db: f32,

    /// The minimum gain (db) for the high eq band.
    pub high_min_db: f32,

    /// The maximum gain (db) for the high eq band.
    pub high_max_db: f32,
}
