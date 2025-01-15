use bevy::{audio, prelude::*, utils::HashMap};
use bevy_fmod::{components::audio_source::AudioSource, prelude::*};
use libfmod::{Attributes3d, EventDescription, PlaybackState, Vector};

pub const SFX_STANDARD_PLASMA: &str = "event:/Weapons/StandardPlasma";
pub const SFX_DUAL_LASERS: &str = "event:/Weapons/DualLasers";
pub const SFX_SHOTGUN: &str = "event:/Weapons/Shotgun";
pub const SFX_ROCKET_LAUNCHER: &str = "event:/Weapons/RocketLauncher";
pub const SFX_ROCKET_EXPLOSION: &str = "event:/Weapons/RocketExplosion";
pub const SFX_ROCKET_HISS: &str = "event:/Weapons/RocketHiss";
pub const SFX_HYPER_PLASMA: &str = "event:/Weapons/HyperPlasma";
pub const SFX_PLASMA_HISS: &str = "event:/Weapons/PlasmaHiss";

pub const SFX_BOT_GRUNT_PLASMA_SHOOT: &str = "event:/Bots/GruntPlasmaShoot";
pub const SFX_BOT_GRUNT_LASER_SHOOT: &str = "event:/Bots/GruntPlasmaShoot";
pub const SFX_BOT_GRUNT_FUSION_SHOOT: &str = "event:/Bots/GruntFusionShoot";
pub const SFX_BOT_GRUNT_DRONE_FLIGHT: &str = "event:/Bots/GruntDroneFlight";

pub struct SfxPlugin;
impl Plugin for SfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_sounds)
            .add_systems(Update, play_sounds)
            .add_event::<AudioEvent>();
    }
}

#[derive(Component)]
struct AudioSpawnedAndPlaying;

#[derive(Event)]
pub struct AudioEvent {
    pub event_name: String,
    pub translation: Vec3,
    pub first_person: bool,
    pub randomness: f32,
}

fn load_sounds(studio: ResMut<FmodStudio>) {
    let sound_paths = vec![
        SFX_STANDARD_PLASMA,
        SFX_DUAL_LASERS,
        SFX_SHOTGUN,
        SFX_ROCKET_LAUNCHER,
        SFX_HYPER_PLASMA,
    ];

    for path in sound_paths {
        match studio.0.get_event(path) {
            Ok(event_description) => {
                if let Err(e) = event_description.load_sample_data() {
                    println!("Failed to load sample data for event {}", e);
                }
            }
            Err(e) => {
                println!("Failed to get event description: {}", e);
            }
        }
    }
}

fn play_sounds(
    mut audio_events: EventReader<AudioEvent>,
    studio: ResMut<FmodStudio>,
    mut audio_sources: Query<(Entity, &AudioSource), Without<AudioSpawnedAndPlaying>>,
    mut commands: Commands,
) {
    for event in audio_events.read() {
        if let Ok(event_description) = studio.0.get_event(&event.event_name) {
            if let Ok(event_instance) = event_description.create_instance() {
                _ = event_instance.set_3d_attributes(Attributes3d {
                    position: Vector {
                        x: event.translation.x,
                        y: event.translation.y,
                        z: event.translation.z,
                    },
                    velocity: Vector {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    forward: Vector {
                        x: 0.0,
                        y: 0.0,
                        z: 1.0,
                    },
                    up: Vector {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                });

                let twodeeness = if event.first_person { 1.0 } else { 0.0 };

                if event_instance
                    .set_parameter_by_name("2Dness", twodeeness, true)
                    .is_err()
                {
                    println!(
                        "Failed to set 2Dness parameter for event {}",
                        event.event_name
                    );
                }

                if let Err(e) = event_instance.start() {
                    println!("Failed to start event instance: {}", e);
                }

                if let Err(e) = event_instance.release() {
                    println!("Failed to release event instance: {}", e);
                }
            }
        }
    }

    for (entity, audio_source) in audio_sources.iter_mut() {
        if let Err(e) = audio_source.event_instance.start() {
            println!("Failed to start event instance: {}", e);
        }

        if let Err(e) = audio_source.event_instance.release() {
            println!("Failed to release event instance: {}", e);
        }

        commands.entity(entity).insert(AudioSpawnedAndPlaying);
    }
}
