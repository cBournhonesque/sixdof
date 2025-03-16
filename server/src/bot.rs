use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::server::*;
use lightyear_avian::prelude::LagCompensationHistory;
use shared::bot::{Bot, BotAttackKind};
use shared::player::Player;
use shared::prelude::{Damageable, GameLayer, UniqueIdentity};
use shared::ships::{get_shared_ship_components, move_ship, ShipIndex, ShipsData};
// TODO: should bots be handled similarly to players? i.e. they share most of the same code (visuals, collisions)
//  but they are simply controlled by the server. The server could be sending fake inputs to the bots so that their movement
//  is the same as players
//  For now i'm just using them to debug lag compensation

pub(crate) struct BotPlugin;
impl Plugin for BotPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BotManager { next_bot_id: 0 });
        app.add_systems(Startup, spawn_bot);
        app.add_systems(FixedUpdate, (
            target_tracking_system,
            move_system.run_if(resource_exists::<ShipsData>),
        ));
    }
}

#[derive(Resource)]
struct BotManager {
    next_bot_id: u32,
}

#[derive(Component)]
enum OrbitKind {
    HorizontalClockwise,
    HorizontalCounterClockwise,
    VerticalClockwise,
    VerticalCounterClockwise,
}

#[derive(Component)]
struct BotTarget {
    entity: Entity,
    orbit_kind: OrbitKind,
    orbit_timer: f32,
}

impl BotTarget {
    fn new(entity: Entity) -> Self {
        let orbit_kind = match rand::random::<u8>() % 4 {
            0 => OrbitKind::HorizontalClockwise,
            1 => OrbitKind::HorizontalCounterClockwise,
            2 => OrbitKind::VerticalClockwise,
            _ => OrbitKind::VerticalCounterClockwise,
        };

        Self {
            entity,
            orbit_kind,
            orbit_timer: 0.0,
        }
    }

    fn update(&mut self, delta_time: f32, change_interval: f32) {
        self.orbit_timer += delta_time;
        if self.orbit_timer >= change_interval {
            self.choose_new_orbit_direction();
            self.orbit_timer = 0.0;
        }
    }

    fn choose_new_orbit_direction(&mut self) {
        self.orbit_kind = match rand::random::<u8>() % 4 {
            0 => OrbitKind::HorizontalClockwise,
            1 => OrbitKind::HorizontalCounterClockwise,
            2 => OrbitKind::VerticalClockwise,
            _ => OrbitKind::VerticalCounterClockwise,
        }
    }
}

fn spawn_bot(mut commands: Commands, mut bot_manager: ResMut<BotManager>) {
    // TODO: use spawn-events so we can control spawn position, etc.
    let position = Position(Vec3::new(1.0, 4.0, -1.0));
    let rotation = Rotation(Quat::from_rotation_arc(Vec3::Y, Vec3::NEG_Z));
    commands.spawn(
        (
            Name::from("Bot"),
            Replicate {
                sync: SyncTarget {
                    interpolation: NetworkTarget::All,
                    ..default()
                },
                // in case the renderer is enabled on the server, we don't want the visuals to be replicated!
                hierarchy: ReplicateHierarchy {
                    enabled: false,
                    recursive: false,
                },
                // TODO: all predicted entities must be part of the same replication group
                ..default()
            },
            UniqueIdentity::Bot(bot_manager.next_bot_id),
            Bot {
                wish_dir: Vec3::ZERO,
            },
            Damageable {
                health: 50,
            },
            ShipIndex(1),
            // TODO: UNDERSTAND WHY IT IS NECESSARY TO MANUALLY INSERT THE CORRECT POSITION/ROTATION
            //  ON THE ENTITY! I THOUGHT THE PREPARE_SET WOULD DO THIS AUTOMATICALLY
            position,
            rotation,
            get_shared_ship_components(Collider::sphere(0.5)),
            LagCompensationHistory::default(),
        )
    );
    bot_manager.next_bot_id += 1;
}

/// The main bot movement system, this dictates how bots go after their target and navigate around the map.
/// This system will manipulate the bot's velocity directly. Actual movement & collision is handled by Avian3d's physics system.
///
/// Most of the bot's behavior is documented in the BotBehavior struct.
/// - But for the most part, aggressive bots will orbit around their target and attack when in range.
/// - Standard bots will move towards their target and attack when in range.
/// - Bots will also avoid walls and other obstacles.
/// - We compute a wish direction for the bot, this is quite simply the direction the bot wishes to move in at any given time.
/// - Bots will not wish to move if there is no target.
fn move_system(
    spatial_query: SpatialQuery,
    fixed_time: Res<Time<Fixed>>,
    mut targets: Query<&mut BotTarget>,
    positions: Query<&Position>,
    mut bots: Query<(Entity, &Position, &mut LinearVelocity, &mut AngularVelocity, &ShipIndex, &mut Bot)>,
    ships_data: Res<ShipsData>,
) {
    let delta = fixed_time.delta_secs();

    for (bot_entity, bot_position, mut linear_velocity, mut angular_velocity, ship_index, mut bot) in bots.iter_mut() {
        if let Some(ship_behavior) = ships_data.ships.get(&ship_index.0) {
            let mut wish_dir = Vec3::ZERO;
            let mut found_bot_target = None;
            if let Ok(mut bot_target) = targets.get_mut(bot_entity) {
                if let (Ok(target_position), Ok(bot_position)) = (positions.get(bot_target.entity), positions.get(bot_entity)) {
                    let target_pos = target_position.0;
                    let bot_pos = bot_position.0;
                    let distance = target_pos.distance(bot_pos);
                    let dir_to_target = (target_pos - bot_pos).normalize_or_zero();

                    match ship_behavior.bot_behavior.attack_kind {
                        BotAttackKind::Aggressive {
                            target_distance,
                            change_orbit_dir_interval,
                            orbit_dir_back_off_blend_amount,
                            orbit_dir_blend_amount,
                            orbit_dir_target_blend_amount,
                            wish_dir_random_factor
                        } => {
                            bot_target.update(delta, change_orbit_dir_interval);

                            let to_target = target_pos - bot_pos;

                            let orbit_dir = match bot_target.orbit_kind {
                                OrbitKind::HorizontalClockwise => {
                                    let to_target_flat = Vec3::new(to_target.x, 0.0, to_target.z);
                                    Vec3::new(-to_target_flat.z, 0.0, to_target_flat.x)
                                }
                                OrbitKind::HorizontalCounterClockwise => {
                                    let to_target_flat = Vec3::new(to_target.x, 0.0, to_target.z);
                                    Vec3::new(to_target_flat.z, 0.0, -to_target_flat.x)
                                }
                                OrbitKind::VerticalClockwise => {
                                    let to_target_vertical = Vec3::new(to_target.x, to_target.y, 0.0);
                                    Vec3::new(-to_target_vertical.y, to_target_vertical.x, 0.0)
                                }
                                OrbitKind::VerticalCounterClockwise => {
                                    let to_target_vertical = Vec3::new(to_target.x, to_target.y, 0.0);
                                    Vec3::new(to_target_vertical.y, -to_target_vertical.x, 0.0)
                                }
                            }.normalize();

                            // If too far, blend in some inward movement
                            if distance > target_distance {
                                wish_dir = ((orbit_dir * orbit_dir_blend_amount) + dir_to_target).normalize();
                            }
                            // If too close, blend in some outward movement
                            else if distance < ship_behavior.bot_behavior.back_off_distance {
                                wish_dir = ((orbit_dir * orbit_dir_back_off_blend_amount) - dir_to_target).normalize();
                            }
                            // Otherwise pure orbital motion with a slight bias towards the target
                            else {
                                wish_dir = (orbit_dir + (dir_to_target * orbit_dir_target_blend_amount)).normalize();
                            }

                            // add some slight random movement
                            wish_dir += Vec3::new(
                                (rand::random::<f32>() * 2.0 - 1.0) * wish_dir_random_factor,
                                (rand::random::<f32>() * 2.0 - 1.0) * wish_dir_random_factor,
                                (rand::random::<f32>() * 2.0 - 1.0) * wish_dir_random_factor,
                            );
                        }
                        BotAttackKind::Standard { target_distance, .. } => {
                            if distance > target_distance {
                                wish_dir = dir_to_target;
                            } else if distance < ship_behavior.bot_behavior.back_off_distance {
                                wish_dir = -dir_to_target;
                            }
                        }
                    }
                }

                found_bot_target = Some(bot_target);
            }

            wish_dir = wish_dir.normalize_or_zero();

            // deflect the ship if it's about to hit a wall
            if let Some(hit) = spatial_query.cast_ray(
                bot_position.0,
                Dir3::new(wish_dir).unwrap_or(Dir3::NEG_Z),
                ship_behavior.bot_behavior.wall_avoidance_distance,
                true,
                &SpatialQueryFilter::default()
                    .with_mask([GameLayer::Wall])
                    .with_excluded_entities([bot_entity]),
            ) {
                // wish to move along the wall
                wish_dir = hit.normal;

                // try and change the orbit direction if we have a target
                if let Some(mut bot_target) = found_bot_target {
                    bot_target.choose_new_orbit_direction();
                }
            }

            // blend the wish direction over time so we're not changing it abruptly,
            // a lower wish_dir_change_speed will make the bot change direction slower
            bot.wish_dir = bot.wish_dir.lerp(wish_dir, ship_behavior.bot_behavior.wish_dir_change_speed * delta);

            move_ship(&fixed_time, &ship_behavior, &mut linear_velocity, &mut angular_velocity, bot.wish_dir, None);
        }
    }
}

/// Track the nearest visible player and set the bot's target to the player
fn target_tracking_system(
    spatial_query: SpatialQuery,
    mut commands: Commands,
    targets: Query<&BotTarget>,
    bots: Query<(Entity, &Position), With<Bot>>,
    players: Query<(Entity, &Position), With<Player>>,
) {
    for (bot_entity, bot_position) in bots.iter() {
        let mut nearest_player = None;
        let mut nearest_distance = f32::MAX;
        for (player_entity, player_position) in players.iter() {
            let distance = bot_position.0.distance(player_position.0);
            let direction = (player_position.0 - bot_position.0).normalize();
            info!("Checking player distance and direction: {:?}, {:?}", distance, direction);

            // check if the player is visible from the bot's perspective
            if let Some(hit) = spatial_query.cast_ray(
                bot_position.0,
                Dir3::new(direction).unwrap_or(Dir3::NEG_Z),
                distance,
                true,
                &SpatialQueryFilter::default()
                    .with_excluded_entities([bot_entity]),
            ) {
                info!("Hit: {:?}", hit.entity);
                if hit.entity == player_entity && distance < nearest_distance {
                    info!("Player hit");
                    nearest_player = Some(player_entity);
                    nearest_distance = distance;
                }
            }
        }

        if let Some(player_entity) = nearest_player {
            if !targets.contains(bot_entity) {
                commands.entity(bot_entity).insert(BotTarget::new(player_entity));
            }
        } else {
            commands.entity(bot_entity).remove::<BotTarget>();
        }
    }
}
