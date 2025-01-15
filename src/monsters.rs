use crate::components::*;
use crate::ids::IdPooler;
use crate::physics::*;
use crate::player::*;
use crate::sfx::*;
use crate::weapons::*;
use bevy::ecs::schedule::NodeConfigs;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use mlua::prelude::*;
use rand::Rng;
use rand::RngCore;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

const BOT_HARASS_RANGE: f32 = 8.0;
const VISUALS_LERP_SPEED: f32 = 0.1;
const VISUALS_MAX_LERP_DISTANCE: f32 = 5.0;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum MonsterKind {
    GruntFusion,
    GruntLasers,
    GruntPlasma,
}

impl From<u8> for MonsterKind {
    fn from(value: u8) -> Self {
        match value {
            0 => MonsterKind::GruntFusion,
            1 => MonsterKind::GruntLasers,
            2 => MonsterKind::GruntPlasma,
            _ => MonsterKind::GruntFusion,
        }
    }
}

impl<'lua> FromLua<'lua> for MonsterKind {
    fn from_lua(value: mlua::Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let value = String::from_lua(value, lua)?;

        match value.as_str() {
            "GruntFusion" => Ok(MonsterKind::GruntFusion),
            "GruntLasers" => Ok(MonsterKind::GruntLasers),
            "GruntPlasma" => Ok(MonsterKind::GruntPlasma),
            _ => Ok(MonsterKind::GruntFusion),
        }
    }
}

impl<'lua> IntoLua<'lua> for MonsterKind {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<mlua::Value<'lua>> {
        let value = match self {
            MonsterKind::GruntFusion => "GruntFusion",
            MonsterKind::GruntLasers => "GruntLasers",
            MonsterKind::GruntPlasma => "GruntPlasma",
        };

        value.into_lua(lua)
    }
}

//======================================================================
// bots
//======================================================================
#[derive(Component)]
pub struct BotAlarmedTimer {
    pub timer: Timer,
}

#[derive(Default, Component)]
pub struct NavigationNode;

#[derive(Component)]
pub struct Monster {
    pub id: u8,
    pub kind: MonsterKind,
    pub active_threat: Option<Entity>,
    pub threat_alarm_time_secs: f32,
    pub threat_detection_range: f32,
}

impl Default for Monster {
    fn default() -> Self {
        Self {
            id: 0,
            kind: MonsterKind::GruntFusion,
            active_threat: None,
            threat_alarm_time_secs: 0.25,
            threat_detection_range: 60.0,
        }
    }
}

impl Monster {
    pub fn systems() -> NodeConfigs<Box<dyn System<In = (), Out = ()>>> {
        (Self::threat_tracker_system, Self::follow_and_attack_system).chain()
    }

    pub fn threat_tracker_system(
        physics_context: Res<RapierContext>,
        local_player: Res<LocalPlayer>,
        mut commands: Commands,
        mut players: Query<(Entity, &Health, &GlobalTransform), (With<Player>, Without<Monster>)>,
        mut bots: Query<(Entity, &GlobalTransform, &mut Monster)>,
    ) {
        if !local_player.has_authority() {
            return;
        }

        let physics_context = Rc::new(physics_context);

        for (bot_entity, bot_transform, mut monster) in bots.iter_mut() {
            let physics_context = physics_context.clone();

            let mut threat_candidate = None;
            let mut closest_threat_distance = f32::MAX;

            monster.active_threat = None;

            for (threat_entity, threat_health, threat_transform) in players.iter_mut() {
                let physics_context = physics_context.clone();

                if threat_health.dead() {
                    continue;
                }

                let distance = bot_transform
                    .translation()
                    .distance(threat_transform.translation());

                // too far? not a threat
                if distance > monster.threat_detection_range {
                    continue;
                }

                if is_threat_visible(
                    physics_context,
                    bot_entity,
                    bot_transform,
                    threat_entity,
                    threat_transform,
                ) {
                    if distance < closest_threat_distance {
                        closest_threat_distance = distance;
                        threat_candidate = Some(threat_entity);
                    }
                }
            }

            if threat_candidate.is_some() {
                monster.active_threat = threat_candidate;
            }

            if monster.active_threat.is_none() {
                commands.entity(bot_entity).remove::<BotAlarmedTimer>();
                commands.entity(bot_entity).remove::<WantsToUseWeapon>();
            }
        }
    }

    pub fn follow_and_attack_system(
        mut commands: Commands,
        local_player: Res<LocalPlayer>,
        physics_context: Res<RapierContext>,
        threats: Query<(Entity, &GlobalTransform), (With<Player>, Without<Monster>)>,
        nav_nodes: Query<&GlobalTransform, With<NavigationNode>>,
        mut bots: Query<(Entity, &GlobalTransform, &Seed, &mut WishMove, &Monster)>,
    ) {
        if !local_player.has_authority() {
            return;
        }

        let physics_context = Rc::new(physics_context);

        for (bot_entity, bot_transform, bot_seed, mut bot_wish_move, bot) in bots.iter_mut() {
            if let Some(threat) = bot.active_threat {
                if let Ok((threat_entity, threat_transform)) = threats.get(threat) {
                    let threat_visible = is_threat_visible(
                        physics_context.clone(),
                        bot_entity,
                        bot_transform,
                        threat_entity,
                        threat_transform,
                    );

                    let look_at_direction =
                        threat_transform.translation() - bot_transform.translation();

                    let rotation = Quat::from_rotation_arc(Vec3::Z, -look_at_direction.normalize());

                    bot_wish_move.rotation = rotation;

                    // start moving them
                    let position = bot_transform.translation();
                    let target_position = threat_transform.translation();

                    let direction_to_target = target_position - position;
                    let distance_to_target = direction_to_target.length();
                    let orbit_distance = BOT_HARASS_RANGE; // Set your desired orbit distance

                    // can we see the threat?
                    let ray_pos = position;

                    // so if we're close & the threat is visible
                    // we want to start harassing it by orbiting around it
                    bot_wish_move.direction =
                        if threat_visible && distance_to_target <= orbit_distance {
                            if let Some(mut entity_cmd) = commands.get_entity(bot_entity) {
                                entity_cmd.insert(WantsToUseWeapon);
                            }

                            // Orbit around the target if it's too close
                            let perpendicular =
                                Vec3::new(-direction_to_target.y, direction_to_target.x, 0.0)
                                    .normalize();

                            let mut rng = rand::rngs::StdRng::seed_from_u64(bot_seed.0 as u64);

                            // Add some randomness to the orbit direction
                            let randomness = Vec3::new(
                                rng.next_u64() as f32 * 0.5,
                                rng.next_u64() as f32 * 0.5,
                                rng.next_u64() as f32 * 0.5,
                            );

                            // Combine the perpendicular direction with randomness
                            let orbit_direction = (perpendicular + randomness).normalize();

                            orbit_direction
                        } else {
                            // otherwise, we need to try and move towards it
                            if threat_visible {
                                if let Some(mut entity_cmd) = commands.get_entity(bot_entity) {
                                    entity_cmd.insert(WantsToUseWeapon);
                                }
                                direction_to_target.normalize()
                            } else {
                                if let Some(mut entity_cmd) = commands.get_entity(bot_entity) {
                                    entity_cmd.remove::<WantsToUseWeapon>();
                                }

                                // if we can't see the threat, we need to try and move towards it
                                let mut visible_nodes = Vec::<Vec3>::new();
                                for nav_node_transform in nav_nodes.iter() {
                                    let ray_dir = nav_node_transform.translation() - ray_pos;
                                    let max_toi = ray_dir.length();
                                    let solid = true;
                                    let filter = QueryFilter {
                                        flags: QueryFilterFlags::EXCLUDE_SENSORS,
                                        exclude_collider: Some(bot_entity),
                                        ..default()
                                    };

                                    if let None = physics_context
                                        .cast_ray(ray_pos, ray_dir, max_toi, solid, filter)
                                    {
                                        visible_nodes.push(nav_node_transform.translation());
                                    }
                                }

                                if let Some(best_node) = Self::find_best_node_in_direction(
                                    &visible_nodes,
                                    position,
                                    direction_to_target.normalize(),
                                ) {
                                    let direction_to_target = best_node - position;
                                    direction_to_target.normalize()
                                } else {
                                    Vec3::ZERO
                                }
                            }
                        };
                }
            }

            if bot.active_threat.is_none() {
                bot_wish_move.direction = Vec3::ZERO;
            }
        }
    }

    fn find_best_node_in_direction(
        nodes: &Vec<Vec3>,
        monster: Vec3,
        monster_direction: Vec3,
    ) -> Option<Vec3> {
        let cone_half_angle_degrees: f32 = 90.0;

        // find the furthest node in the cone
        let mut best_node = None;

        let mut best_distance = 0.0;

        for node in nodes.iter() {
            let direction_to_node = *node - monster;
            let distance_to_node = direction_to_node.length();

            let angle_between = monster_direction
                .angle_between(direction_to_node.normalize())
                .to_degrees();

            if angle_between <= cone_half_angle_degrees {
                if distance_to_node > best_distance {
                    best_distance = distance_to_node;
                    best_node = Some(*node);
                }
            }
        }

        best_node
    }
}

#[derive(Component)]
pub struct MonsterVisuals;

pub fn bot_fire_system<'a>(
    local_player: Res<LocalPlayer>,
    time: Res<Time>,
    wants_to_use_weapon: Query<&WantsToUseWeapon>,
    mut idpooler: ResMut<IdPooler>,
    mut weapon_containers: Query<(Entity, &mut WeaponContainer, &Transform), With<Monster>>,
    mut spawn_events: EventWriter<SpawnProjectileEvent>,
    mut audio_events: EventWriter<AudioEvent>,
    weapon_configs: Res<Assets<WeaponConfig>>,
) {
    if !local_player.has_authority() {
        return;
    }

    for (entity, mut weapon_container, transform) in weapon_containers.iter_mut() {
        weapon_container.tick(
            WeaponContainerTickConfig {
                try_fire: wants_to_use_weapon.get(entity).is_ok(),
                input_id: None,
                origin_transform: transform,
                projectile_nudge_time_millis: None,
                predict: false,
                movement_state: None,
                wish_weapon_key: None,
                seed: rand::rngs::ThreadRng::default().gen::<u64>(),
            },
            &time,
            &mut idpooler,
            &mut spawn_events,
            &mut audio_events,
            &weapon_configs,
        );
    }
}

fn is_threat_visible(
    physics_context: Rc<Res<RapierContext>>,
    bot_entity: Entity,
    bot_transform: &GlobalTransform,
    threat_entity: Entity,
    threat_transform: &GlobalTransform,
) -> bool {
    let ray_pos = bot_transform.translation();
    let ray_dir = threat_transform.translation() - bot_transform.translation();
    let max_toi = ray_dir.length();
    let solid = true;
    let filter = QueryFilter {
        flags: QueryFilterFlags::EXCLUDE_SENSORS,
        exclude_collider: Some(bot_entity),
        ..default()
    };

    if let Some((raycasted_entity, _toi)) =
        physics_context.cast_ray(ray_pos, ray_dir, max_toi, solid, filter)
    {
        // we hit something, but is it the threat candidate?
        if raycasted_entity == threat_entity {
            return true;
        }
    }

    false
}

pub fn visuals_system(time: Res<Time>, mut visuals: Query<&mut Transform, With<MonsterVisuals>>) {
    if let Ok(mut transform) = visuals.get_single_mut() {
        if transform.translation.length() < VISUALS_MAX_LERP_DISTANCE {
            transform.translation = transform.translation.lerp(
                Vec3::ZERO,
                (VISUALS_LERP_SPEED * time.delta_seconds()).min(1.0),
            );

            // if it's close enough just snap it
            if transform.translation.length_squared() < 0.001 {
                transform.translation = Vec3::ZERO;
            }
        } else {
            transform.translation = Vec3::ZERO;
        }
    }
}
