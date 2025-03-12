use std::ops::DerefMut;
use bevy::utils::{Duration, HashMap};
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::time::Stopwatch;
use lightyear::prelude::*;
use lightyear::prelude::server::*;
use lightyear_avian::prelude::LagCompensationHistory;
use rand::Rng;
use shared::bot::{Bot, BotAttackKind};
use shared::player::Player;
use shared::prelude::{Damageable, GameLayer, UniqueIdentity};
use shared::ships::{get_shared_ship_components, move_ship, ShipId, ShipIndex, ShipsData};
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
            // Randomly choose new orbit kind
            self.orbit_kind = match rand::random::<u8>() % 4 {
                0 => OrbitKind::HorizontalClockwise,
                1 => OrbitKind::HorizontalCounterClockwise,
                2 => OrbitKind::VerticalClockwise,
                _ => OrbitKind::VerticalCounterClockwise,
            };
            self.orbit_timer = 0.0;
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
            Bot,
            Damageable {
                health: 50,
            },
            ShipIndex(1),
            // TODO: UNDERSTAND WHY IT IS NECESSARY TO MANUALLY INSERT THE CORRECT POSITION/ROTATION
            //  ON THE ENTITY! I THOUGHT THE PREPARE_SET WOULD DO THIS AUTOMATICALLY
            position,
            rotation,
            get_shared_ship_components(Collider::sphere(0.5))
            //LagCompensationHistory::default(),
        )
    );
    bot_manager.next_bot_id += 1;
}

/// Move bots up and down
/// For some reason we cannot use the TimeManager.delta() here, maybe because we're running in FixedUpdate?
fn move_system(
    fixed_time: Res<Time<Fixed>>,
    mut targets: Query<&mut BotTarget>,
    transforms: Query<&Transform>,
    mut bots: Query<(Entity, &mut LinearVelocity, &mut AngularVelocity, &ShipIndex), With<Bot>>, 
    ships_data: Res<ShipsData>,
) {
    let delta = fixed_time.delta_secs();
    
    for (bot_entity, mut linear_velocity, mut angular_velocity, ship_index) in bots.iter_mut() {
        if let Some(ship_behavior) = ships_data.ships.get(&ship_index.0) {
            let mut wish_dir = Vec3::ZERO;

            if let Ok(mut bot_target) = targets.get_mut(bot_entity) {
                if let (Ok(target_transform), Ok(bot_transform)) = (transforms.get(bot_target.entity), transforms.get(bot_entity)) {
                    let target_pos = target_transform.translation;
                    let bot_pos = bot_transform.translation;
                    let distance = target_pos.distance(bot_pos);
                    let dir_to_target = (target_pos - bot_pos).normalize_or_zero();

                    match ship_behavior.bot_behavior.attack_kind {
                        BotAttackKind::Aggressive { target_distance, change_attack_direction_interval, back_off_distance } => {
                            bot_target.update(delta, change_attack_direction_interval);

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
                            if distance > target_distance * 1.2 {
                                wish_dir = (orbit_dir + dir_to_target).normalize();
                            }
                            // If too close, blend in some outward movement
                            else if distance < target_distance * 0.8 {
                                wish_dir = (orbit_dir - dir_to_target).normalize();
                            }
                            // Otherwise pure orbital motion
                            else {
                                wish_dir = orbit_dir;
                            }
                        }
                        BotAttackKind::Standard { target_distance } => {
                            if distance > target_distance {
                                wish_dir = dir_to_target;
                            } else {
                                wish_dir = -dir_to_target;
                            }
                        }
                    }
                }
            }
            
            // fallback
            if wish_dir.length_squared() < 0.001 {
                wish_dir = Vec3::new(0.0, 0.0, -1.0);
            }
            
            move_ship(&fixed_time, &ship_behavior, &mut linear_velocity, &mut angular_velocity, wish_dir, None);
        }
    }
}

/// Track the nearest visible player and set the bot's target to the player
fn target_tracking_system(
    spatial_query: SpatialQuery,
    mut commands: Commands,
    targets: Query<&BotTarget>,
    bots: Query<(Entity, &Transform), With<Bot>>,
    players: Query<(Entity, &Transform), With<Player>>,
) {
    for (bot_entity, bot_transform) in bots.iter() {
        let mut nearest_player = None;
        let mut nearest_distance = f32::MAX;
        for (player_entity, player_transform) in players.iter() {
            let distance = bot_transform.translation.distance(player_transform.translation);
            let direction = (player_transform.translation - bot_transform.translation).normalize();
            
            // check if the player is visible from the bot's perspective
            if let Some(hit) = spatial_query.cast_ray(
                bot_transform.translation,
                Dir3::new(direction).unwrap_or(Dir3::NEG_Z),
                distance,
                true,
                &SpatialQueryFilter::default()
                    .with_excluded_entities([bot_entity]),
            ) {
                if hit.entity == player_entity && distance < nearest_distance {
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
