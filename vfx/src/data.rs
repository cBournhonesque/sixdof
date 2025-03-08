use serde::{Serialize, Deserialize};
use bevy::math::Vec3;
use bevy::color::Color;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VfxEmitterKind {
    Continuous {
        count_per_burst: u32,
        burst_count: u32,
        rate_millis: u32,
    },
}

/// Needed for `VfxScaleKind::OverLifetime`, defines the lifetime percent and scale at that percent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VfxPercentScaleRange {
    pub lifetime_percent: (i32, i32),
    pub scale: (Vec3, Vec3),
}

/// Needed for `VfxColorOverLifetime`, defines the lifetime percent and color at that percent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VfxPercentColorRange {
    pub lifetime_percent: (i32, i32),
    pub color: (Color, Color),
}

/// Defines the behavior of a particle emitter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VfxEmitterBehavior {
    /// The number of particles to spawn per burst.
    pub count_per_burst: u32,
    /// The number of bursts to emit.
    pub burst_count: u32,
    /// The rate at which the particles are bursted.
    pub burst_rate_millis: u32,
    /// The initial scale of the the particle. By default the size of a particle is 1 meter squared.
    pub initial_scale: (f32, f32),
    /// The initial velocity of the particle.
    pub initial_velocity: (Vec3, Vec3),
    /// The velocity decays over time. Like a drag force.
    pub velocity_decay: (f32, f32),
    /// The particle can scale by a velocity over time.
    pub scale_velocity: (f32, f32),
    /// The particle scale velocity decays over time.
    pub scale_velocity_decay: (f32, f32),
    /// The particle can scale over its lifetime.
    pub scale_over_lifetime: Vec<VfxPercentScaleRange>,
    /// The lifetime of the particle in milliseconds.
    pub lifetime_millis: (i32, i32),
    /// The color of the particle over its lifetime.
    pub color_over_lifetime: Vec<VfxPercentColorRange>,
    /// The amplitude of the wave of the particle.
    pub wave_amplitude: (f32, f32),
    /// The frequency of the wave of the particle.
    pub wave_frequency: (f32, f32),
    // Will randomly choose a texture from the list of textures.
    pub textures: Vec<String>,
    // Whether the particle should receive shadows or not.
    pub receive_shadows: bool,
    // The bounciness of the particle. Leave at 0 to disable bouncing.
    pub bounce_factor: f32,
}
