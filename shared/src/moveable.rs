use bevy::prelude::*;
use avian3d::prelude::*;
use serde::{Deserialize, Serialize};

use crate::physics::GameLayer;

pub struct ShapecastMoveablePlugin;

impl Plugin for ShapecastMoveablePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (
            move_system,
        ));
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ShapecastMoveableShape {
    Sphere(f32),
}

#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Moveable {
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub collision_shape: ShapecastMoveableShape,
    pub collision_mask: LayerMask,
}

fn move_system(
    fixed_time: Res<Time<Fixed>>,
    spatial_query: SpatialQuery,
    mut simulations: Query<(Entity, &mut Moveable, &mut Transform)>,
) {
    for (entity, mut simulation, mut transform) in simulations.iter_mut() {
        const EPSILON: f32 = 0.001;
        
        let mut velocity = simulation.velocity;
        let mut remaining_motion = velocity * fixed_time.delta_secs();
    
        let collider = match &simulation.collision_shape {
            ShapecastMoveableShape::Sphere(radius) => {
                Collider::sphere(*radius)
            }
        };

        for _ in 0..4 {
    
            if let Some(hit) = spatial_query.cast_shape(
                &collider,
                transform.translation,
                Quat::default(),
                Dir3::new(remaining_motion.normalize_or_zero()).unwrap_or(Dir3::X),
                &ShapeCastConfig::from_max_distance(remaining_motion.length()),
                &SpatialQueryFilter {
                    mask: simulation.collision_mask,
                    ..default()
                }.with_excluded_entities([entity]),
            ) {
                // Move to just before the collision point
                transform.translation += remaining_motion.normalize_or_zero() * hit.distance;
    
                // Prevents sticking
                transform.translation += hit.normal1 * EPSILON;
    
                // Deflect velocity along the surface
                velocity -= hit.normal1 * velocity.dot(hit.normal1);
                remaining_motion -= hit.normal1 * remaining_motion.dot(hit.normal1);
            } else {
                // No collision, move the full distance
                transform.translation += remaining_motion;
                break;
            }
        }

        simulation.velocity = velocity;

        // Convert world space angular velocity to local space
        let local_angular_velocity = transform.rotation.inverse() * simulation.angular_velocity;
        
        // Create rotation delta in local space
        let rotation_delta = if local_angular_velocity.length_squared() > 0.0 {
            let angle = local_angular_velocity.length() * fixed_time.delta_secs();
            let axis = local_angular_velocity.normalize();
            Quat::from_axis_angle(axis, angle)
        } else {
            Quat::IDENTITY
        };

        // Apply rotation in local space
        transform.rotation = transform.rotation * rotation_delta;
    }
}

pub fn lerp(start: &Moveable, other: &Moveable, t: f32) -> Moveable {
    Moveable {
        velocity: start.velocity.lerp(other.velocity, t),
        angular_velocity: start.angular_velocity.lerp(other.angular_velocity, t),
        collision_shape: start.collision_shape.clone(),
        collision_mask: start.collision_mask.clone(),
    }
}

