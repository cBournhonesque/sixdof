use bevy::prelude::*;
use avian3d::prelude::*;
use lightyear::prelude::client::{Predicted, Rollback};
use serde::{Deserialize, Serialize};

use crate::prelude::UniqueIdentity;

pub struct ShapecastMoveablePlugin;

impl Plugin for ShapecastMoveablePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (
            move_system,
        ));
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MoveableShape {
    Point,
    Sphere(f32),
}

/// A moveable is an object that can move around the world kinematically.
/// Internally it uses shapecasting (or raycasting for Point based moveables) to detect collisions.
/// It handles the collision response by sliding along the surface of the objects it hits.
#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Moveable {
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub collision_shape: MoveableShape,
    pub collision_mask: LayerMask,
}

/// Data about a hit that occurred during movement.
pub enum MoveableHitData {
    ShapeCast(ShapeHitData),
    RayCast(RayHitData),
}

/// Non replicated data about a moveable, contains hooks used to modify the moveable's behavior during movement.
#[derive(Component)]
pub struct MoveableExtras {
    /// If set, the moveable will not collide with the specified entities.
    pub ignore_entities: Option<Vec<Entity>>,

    /// The owner of the moveable, can be anything
    pub moveable_owner_id: UniqueIdentity,

    /// The type of the moveable, can be anything, 
    /// example: such as a weapon id, for which we can grab the weapon's projectile behavior data in a hook.
    pub moveable_type_id: u32,

    /// Called when the moveable hits a shape
    /// Return true if we want to finish all further movement computations this frame. 
    /// Useful for things like projectiles that explode on impact.
    pub on_hit: Option<Box<dyn Fn(MoveableHit, &mut Commands, &mut SpatialQuery) -> bool + Send + Sync>>,
}

/// Data about a hit that occurred during movement.
pub struct MoveableHit {
    /// The owner of the moveable, can be anything
    pub moveable_owner_id: UniqueIdentity,
    /// The type of the moveable, can be anything
    pub moveable_type_id: u32,
    /// The entity of the moveable
    pub moveable_entity: Entity,
    /// The time of the hit
    pub fixed_time: Time<Fixed>,
    /// The hit data
    pub hit_data: MoveableHitData,
    /// The transform of the moveable, adjusted after hitting
    pub transform: Transform,
}

fn move_system(
    fixed_time: Res<Time<Fixed>>,
    rollback: Option<Res<Rollback>>,
    predicted: Query<&mut Predicted>,
    mut commands: Commands,
    mut spatial_query: SpatialQuery,
    mut moveable_extras: Query<&mut MoveableExtras>,
    mut simulations: Query<(Entity, &mut Moveable, &mut Transform)>,
) {
    let rolling_back = rollback.map_or(false, |r| r.is_rollback());

    for (entity, mut simulation, mut transform) in simulations.iter_mut() {
        // If we're in a rollback AND the moveable is not predicted, skip it
        // otherwise it will move multiple times in a frame and we dont want 
        // that for non-predicted moveables
        if rolling_back && !predicted.contains(entity) {
            continue;
        }

        const EPSILON: f32 = 0.001;
        
        let collider = match &simulation.collision_shape {
            MoveableShape::Sphere(radius) => {
                Some(Collider::sphere(*radius))
            }
            MoveableShape::Point => {
                None
            }
        };

        let mut velocity = simulation.velocity;
        let mut remaining_motion = velocity * fixed_time.delta_secs();

        let mut ignore_entities = Vec::new();
        ignore_entities.push(entity); // always ignore the moveable itself lol

        let mut extras = moveable_extras.get_mut(entity);
        if let Ok(extras) = &mut extras {
            if let Some(extra_ignore_entities) = &mut extras.ignore_entities {
                ignore_entities.extend(extra_ignore_entities.iter());
            }
        }

        // We loop 4 times because you may hit one wall, then slide into another wall, 
        // we need to make sure we keep deprojecting until we're not hitting anything
        // this technique is taken from Quake 1, Google "PM_Move Quake"
        'outer: for _ in 0..4 {

            // Moveable with a shape collider (anything but the Point based collider)
            if let Some(collider) = &collider {
                if let Some(hit) = spatial_query.cast_shape(
                    &collider,
                    transform.translation,
                    Quat::default(),
                    Dir3::new(remaining_motion.normalize_or_zero()).unwrap_or(Dir3::X),
                    &ShapeCastConfig::from_max_distance(remaining_motion.length()),
                    &SpatialQueryFilter {
                        mask: simulation.collision_mask,
                        ..default()
                    }.with_excluded_entities(ignore_entities.clone()),
                ) {
                    // Move to just before the collision point
                    transform.translation += remaining_motion.normalize_or_zero() * hit.distance;

                    // Prevents sticking
                    transform.translation += hit.normal1 * EPSILON;

                    // Deflect velocity along the surface
                    velocity -= hit.normal1 * velocity.dot(hit.normal1);
                    remaining_motion -= hit.normal1 * remaining_motion.dot(hit.normal1);

                    // Fire the on_hit hook
                    if let Ok(extras) = &mut extras {
                        let moveable_type_id = extras.moveable_type_id;
                        let moveable_owner_id = extras.moveable_owner_id;
                        if let Some(on_hit) = &mut extras.on_hit {
                            if on_hit(MoveableHit {
                                moveable_owner_id,
                                moveable_type_id,
                                moveable_entity: entity,
                                hit_data: MoveableHitData::ShapeCast(hit),
                                fixed_time: *fixed_time,
                                transform: *transform,
                            }, &mut commands, &mut spatial_query) {
                                break 'outer;
                            }
                        }
                    }
                } else {
                    // No collision, move the full distance
                    transform.translation += remaining_motion;
                    break 'outer;
                }
            }
            // Point shaped moveable
            else {
                if let Some(hit) = spatial_query.cast_ray(
                    transform.translation,
                    Dir3::new(remaining_motion.normalize_or_zero()).unwrap_or(Dir3::X),
                    remaining_motion.length(),
                    true,
                    &SpatialQueryFilter {
                        mask: simulation.collision_mask,
                        ..default()
                    }.with_excluded_entities(ignore_entities.clone()),
                ) {
                    // Move to just before the collision point
                    transform.translation += remaining_motion.normalize_or_zero() * hit.distance;
        
                    // Prevents sticking
                    transform.translation += hit.normal * EPSILON;
        
                    // Deflect velocity along the surface
                    velocity -= hit.normal * velocity.dot(hit.normal);
                    remaining_motion -= hit.normal * remaining_motion.dot(hit.normal);

                    // Fire the on_hit hook
                    if let Ok(extras) = &mut extras {
                        let moveable_type_id = extras.moveable_type_id;
                        let moveable_owner_id = extras.moveable_owner_id;
                        if let Some(on_hit) = &mut extras.on_hit {
                            if on_hit(MoveableHit {
                                moveable_owner_id,
                                moveable_type_id,
                                moveable_entity: entity,
                                hit_data: MoveableHitData::RayCast(hit),
                                fixed_time: *fixed_time,
                                transform: *transform,
                            }, &mut commands, &mut spatial_query) {
                                break 'outer;
                            }
                        }
                    }
                } else {
                    // No collision, move the full distance
                    transform.translation += remaining_motion;
                    break 'outer;
                }
            }
        }

        // Update velocity
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

