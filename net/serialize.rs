use bevy::prelude::*;

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct QuantizedVec3U16 {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

impl QuantizedVec3U16 {
    pub fn from_vec3(vec: &Vec3, range: u32) -> Self {
        QuantizedVec3U16 {
            x: quant_f32_to_u16(vec.x, range),
            y: quant_f32_to_u16(vec.y, range),
            z: quant_f32_to_u16(vec.z, range),
        }
    }

    pub fn to_vec3(&self, range: u32) -> Vec3 {
        Vec3 {
            x: dequant_u16_to_f32(self.x, range),
            y: dequant_u16_to_f32(self.y, range),
            z: dequant_u16_to_f32(self.z, range),
        }
    }

    pub fn lerp(&mut self, other: &QuantizedVec3U16, t: f32) -> QuantizedVec3U16 {
        QuantizedVec3U16 {
            x: (self.x as f32 + (other.x as f32 - self.x as f32) * t) as u16,
            y: (self.y as f32 + (other.y as f32 - self.y as f32) * t) as u16,
            z: (self.z as f32 + (other.z as f32 - self.z as f32) * t) as u16,
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct QuantizedRotation {
    pub pitch: u8,
    pub yaw: u8,
    pub roll: u8,
}

impl QuantizedRotation {
    fn scale_angle_to_byte(angle: f32) -> u8 {
        // Assuming the angle is in the range [-π, π]
        let normalized = (angle + std::f32::consts::PI) / (2.0 * std::f32::consts::PI);
        (normalized * 255.0) as u8
    }

    fn scale_byte_to_angle(byte: u8) -> f32 {
        let normalized = byte as f32 / 255.0;
        normalized * (2.0 * std::f32::consts::PI) - std::f32::consts::PI
    }

    pub fn from_quat(quat: &Quat) -> Self {
        let (pitch, yaw, roll) = quat.to_euler(EulerRot::YXZ);
        QuantizedRotation {
            pitch: QuantizedRotation::scale_angle_to_byte(pitch),
            yaw: QuantizedRotation::scale_angle_to_byte(yaw),
            roll: QuantizedRotation::scale_angle_to_byte(roll),
        }
    }

    pub fn to_quat(&self) -> Quat {
        let pitch = QuantizedRotation::scale_byte_to_angle(self.pitch);
        let yaw = QuantizedRotation::scale_byte_to_angle(self.yaw);
        let roll = QuantizedRotation::scale_byte_to_angle(self.roll);
        Quat::from_euler(EulerRot::YXZ, pitch, yaw, roll)
    }

    pub fn slerp(&mut self, other: &QuantizedRotation, t: f32) -> QuantizedRotation {
        let q1 = self.to_quat();
        let q2 = other.to_quat();
        let slerped = q1.slerp(q2, t);
        QuantizedRotation::from_quat(&slerped)
    }
}

pub fn quant_f32_to_u16(value: f32, range: u32) -> u16 {
    let min = -(range as f32);
    let max = range as f32;
    let clamped_value = value.clamp(min, max);
    let normalized = (clamped_value - min) / (max - min);
    (normalized * u16::max_value() as f32).round() as u16
}

pub fn dequant_u16_to_f32(value: u16, range: u32) -> f32 {
    let min = -(range as f32);
    let max = range as f32;
    let normalized = value as f32 / u16::max_value() as f32;
    normalized * (max - min) + min
}
