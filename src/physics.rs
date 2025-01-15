use crate::net::*;
use crate::player::*;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

pub const COLLISION_GROUP_NO_COLLISION: Group = Group::from_bits_truncate(0b0001);
pub const COLLISION_GROUP_DYNAMIC: Group = Group::from_bits_truncate(0b0010);
pub const COLLISION_GROUP_MAX: Group = Group::from_bits_truncate(u32::MAX);

#[derive(Component)]
pub struct MovementState {
    pub max_speed: f32,
    pub acceleration: f32,
    pub rotation_speed: f32,
    pub drag: f32,
    pub velocity: Vec3,
    pub last_velocity: Vec3,
}

impl Default for MovementState {
    fn default() -> Self {
        Self {
            max_speed: 8.0,
            acceleration: 1.5,
            rotation_speed: 2.0,
            drag: 0.2,
            velocity: Vec3::ZERO,
            last_velocity: Vec3::ZERO,
        }
    }
}

#[derive(Component)]
pub struct WishMove {
    pub direction: Vec3,
    pub rotation: Quat,
}
impl Default for WishMove {
    fn default() -> Self {
        WishMove {
            direction: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        }
    }
}

pub fn movement_system(
    physics_context: Res<RapierContext>,
    time: Res<Time>,
    local_player: Res<LocalPlayer>,
    mut query: Query<
        (
            Entity,
            &mut Transform,
            &mut WishMove,
            &mut MovementState,
            &mut Collider,
        ),
        (Without<LocallyOwned>, Without<Player>), // locally owned entities are moved by the player
    >,
) {
    if !local_player.has_authority() {
        return;
    }

    for (moving_entity, mut transform, wish_move, mut movement_state, collider) in query.iter_mut()
    {
        crate::physics::move_entity(
            &moving_entity,
            wish_move.direction,
            wish_move.rotation,
            &mut transform,
            &mut movement_state,
            &collider,
            &physics_context,
            time.delta_seconds(),
        );
    }
}

pub fn move_entity(
    entity: &Entity,
    wish_direction: Vec3,
    wish_rotation: Quat,
    transform: &mut Transform,
    movement_state: &mut MovementState,
    collider: &Collider,
    physics: &RapierContext,
    delta_seconds: f32,
) {
    let move_accel = movement_state.acceleration;
    let move_speed = movement_state.max_speed;
    let move_drag = movement_state.drag;

    // save last velocity
    // we use this in places like the player camera system
    movement_state.last_velocity = movement_state.velocity;

    // air drag
    movement_state.velocity = decelerate(
        movement_state.velocity,
        movement_state.velocity.length(),
        move_drag,
        delta_seconds,
    );

    // accelerate
    let current_speed = movement_state.velocity.dot(wish_direction);
    movement_state.velocity += accelerate(
        wish_direction,
        move_speed,
        current_speed,
        move_accel,
        delta_seconds,
    );

    // clamp to max speed
    //movement_state.velocity = movement_state.velocity.clamp_length_max(move_speed);

    transform.rotation = wish_rotation;

    let mut dt = delta_seconds;
    let overbounce = 1.1;
    let padding = 0.1;

    for _ in 0..4 {
        let ray_pos = transform.translation;
        let ray_rot = transform.rotation;
        let ray_dir = movement_state.velocity.normalize();
        let max_toi = (movement_state.velocity.length() * dt) + padding;
        let filter = QueryFilter {
            flags: QueryFilterFlags::EXCLUDE_SENSORS,
            groups: Some(CollisionGroups::new(
                COLLISION_GROUP_DYNAMIC,
                COLLISION_GROUP_MAX,
            )),
            exclude_collider: Some(*entity),
            ..default()
        };

        if let Some((_, toi)) =
            physics.cast_shape(ray_pos, ray_rot, ray_dir, &collider, max_toi, true, filter)
        {
            if let Some(details) = toi.details {
                let mut backoff = movement_state.velocity.dot(details.normal1) - overbounce;
                if backoff < 0.0 {
                    backoff *= overbounce;
                } else {
                    backoff /= overbounce;
                }

                backoff += padding;
                movement_state.velocity -= backoff * details.normal1;
                transform.translation += movement_state.velocity * dt;
                dt -= dt * toi.toi;
            }
        } else {
            transform.translation += movement_state.velocity * dt;
            break;
        }
    }
}

///
/// Framerate-independent acceleration
///
pub fn accelerate(
    wish_direction: Vec3,
    wish_speed: f32,
    current_speed: f32,
    accel: f32,
    delta_seconds: f32,
) -> Vec3 {
    let add_speed = wish_speed - current_speed;

    if add_speed <= 0.0 {
        return Vec3::ZERO;
    }

    let mut accel_speed = accel * delta_seconds * wish_speed;
    if accel_speed > add_speed {
        accel_speed = add_speed;
    }

    wish_direction * accel_speed
}

///
/// Framerate-independent deceleration
///
pub fn decelerate(velocity: Vec3, current_speed: f32, drag: f32, delta_seconds: f32) -> Vec3 {
    let mut new_speed;
    let mut drop = 0.0;

    drop += current_speed * drag * delta_seconds;

    new_speed = current_speed - drop;
    if new_speed < 0.0 {
        new_speed = 0.0;
    }

    if new_speed != 0.0 {
        new_speed /= current_speed;
    }

    velocity * new_speed
}

pub fn step_physics(time: &Time, physics_context: &mut RapierContext) {
    physics_context.step_simulation(
        Vec3::ZERO,
        TimestepMode::Fixed {
            dt: time.delta_seconds(),
            substeps: 1,
        },
        None,
        &(),
        &time,
        &mut SimulationToRenderTime { diff: 0.0 },
        None,
    );
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_rapier3d::prelude::*;
    #[test]
    fn hit_enemy() {
        let mut app = App::new();
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());

        // this is a hit
        let collider = Collider::ball(0.5);
        let time_of_impact = collider.cast_ray(
            Vect::ZERO,
            Rot::IDENTITY,
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, -1.0),
            1000.0,
            true,
        );

        assert!(time_of_impact.is_some());

        // this is a miss
        let time_of_impact = collider.cast_ray(
            Vect::ZERO,
            Rot::IDENTITY,
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
            1000.0,
            true,
        );

        assert!(time_of_impact.is_none());
    }
}
