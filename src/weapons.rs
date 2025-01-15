use crate::components::*;
use crate::fx::*;
use crate::fx2::Explosion;
use crate::fx2::ParticleEvent;
use crate::ids::IdPooler;
use crate::monsters::Monster;
use crate::net::input::*;
use crate::net::server::*;
use crate::physics::MovementState;
use crate::player::*;
use crate::sfx::*;
use crate::snapshot::history::SnapshotHistory;
use crate::snapshot::Snapshot;
use crate::spawn::*;
use crate::WorldState;
use bevy::ecs::schedule::NodeConfigs;
use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use bevy_fmod::components::bundles::SpatialAudioBundle;
use bevy_fmod::fmod_studio::FmodStudio;
use bevy_rapier3d::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const WEAPON_KEY_SHOTGUN: u8 = 3;

pub const SELF_PROJECTILE_VISIBLE_DISTANCE: f32 = 1.5;
pub const RIGHT_BARREL: Vec3 = Vec3::new(0.45, -0.20, 0.0);
pub const LEFT_BARREL: Vec3 = Vec3::new(-0.45, -0.20, 0.0);
pub const CENTER_BARREL: Vec3 = Vec3::new(0.0, -0.0, 0.0);

#[derive(Event)]
pub struct SpawnProjectileVisualsEvent {
    pub entity: Entity,
    pub weapon_key: u8,
}

#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct SpawnProjectileEvent {
    pub id: u16,
    pub weapon_key: u8,
    pub spawn_translation: Vec3,
    pub current_translation: Vec3,
    pub velocity: Vec3,
    pub owner_id: u8,
    pub input_id: Option<u64>,

    #[serde(skip)]
    pub nudge_delta_millis: u8,

    #[serde(skip)]
    pub predicted: bool,
}

impl SpawnProjectileEvent {
    pub fn get_owner_entity(
        &self,
        players: &Query<(Entity, &Player)>,
        bots: &Query<(Entity, &Monster)>,
    ) -> Option<Entity> {
        if self.input_id.is_some() {
            players.iter().find_map(|(entity, player)| {
                if player.id == self.owner_id {
                    Some(entity)
                } else {
                    None
                }
            })
        } else {
            bots.iter().find_map(|(entity, bot)| {
                if bot.id == self.owner_id {
                    Some(entity)
                } else {
                    None
                }
            })
        }
    }
}

#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct DespawnProjectileEvent {
    pub id: u16,
    pub input_id: Option<u64>,
    pub spawn_translation: Vec3,
    pub velocity: Vec3,
    pub weapon_key: u8,
    pub hit_damageable: bool,
    pub normal: Vec3,
    pub translation: Vec3,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerProjectileOwner {
    pub id: u8,
    pub input_id: u64,
    pub weapon_key: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BotProjectileOwner {
    pub id: u8,
}

struct ProjectileHitResult {
    entity: Entity,
    position: Vec3,
    normal: Vec3,
}

struct FireProjectile {
    weapon_key: u8,
    barrel_position: Vec3,
    relative_rotation: Quat,
}

pub struct FireWeapon<'a> {
    pub proj_nudge_time_millis: u8,
    pub owner_id: u8,
    pub input_id: Option<u64>,
    pub origin_transform: &'a Transform,
    pub predicted: bool,
    pub seed: u64,
}

#[derive(Component, Default)]
pub struct PredictedProjectile {
    pub input_id: u64,
    pub id: u16,
    pub deleted: bool,
}

#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct ShotgunFireEvent {
    pub origin_transform: Transform,
    pub seed: u64,
    pub owner_id: u8,
    pub owner_type: OwnerType,
}

struct EnemyHit {
    entity: Entity,
    toi: f32,
    normal: Vec3,
}

#[derive(Serialize, Deserialize)]
pub struct ProjectileFxConfig {
    half_size: f32,
    color: Color,
    hit_particles: Option<Explosion>,
    hit_sound: Option<String>,
    impact_decal: Option<DecalKind>,
    flight_sound: Option<String>,
    light_color: Option<Color>,
    light_intensity: Option<f32>,
}

#[derive(Serialize, Deserialize)]
pub struct ProjectileConfig {
    speed: f32,
    damage: i16,
    splash_damage_radius: f32,
    splash_damage_knockback: f32,
    fx: ProjectileFxConfig,
}

#[derive(Serialize, Deserialize)]
pub struct BurstConfig {
    pub shots_per_burst: u8,
    pub burst_rate_millis: u16,
}

#[derive(Serialize, Deserialize, Asset, TypePath)]
pub struct WeaponConfig {
    pub key: u8,
    pub display_name: String,
    pub fire_rate_millis: u16,
    pub burst_config: Option<BurstConfig>,
    pub ammo: Option<u16>,
    pub spread: f32,
    pub barrel_positions: Vec<Vec3>,
    pub projectiles_per_barrel: u8,
    pub projectile_config: ProjectileConfig,
    pub fire_sound: String,
    pub recoil: f32,
}

impl WeaponConfig {
    pub fn get<'a>(key: u8, configs: &'a Res<Assets<WeaponConfig>>) -> Option<&'a Self> {
        configs.iter().find(|c| c.1.key == key).map(|c| c.1)
    }
}

pub struct WeaponContainerConfig {
    pub owner_id: u8,
    pub weapons: Vec<Weapon>,
}

pub struct WeaponContainerTickConfig<'a> {
    pub try_fire: bool,
    pub wish_weapon_key: Option<u8>,
    pub input_id: Option<u64>,
    pub origin_transform: &'a Transform,
    pub movement_state: Option<&'a mut MovementState>,
    pub projectile_nudge_time_millis: Option<u8>,
    pub predict: bool,
    pub seed: u64,
}

#[derive(Component)]
pub struct WeaponContainer {
    pub owner_id: u8,
    pub weapons: Vec<Weapon>,
    pub current_weapon_key: u8,
    pub fire_timer: Timer,
    pub num_left_in_burst: u8,
}

impl WeaponContainer {
    pub fn new(config: WeaponContainerConfig) -> Self {
        if config.weapons.len() == 0 {
            panic!("WeaponContainerConfig must have at least one weapon");
        }

        let current_weapon_key = config.weapons[0].key;
        Self {
            owner_id: config.owner_id,
            weapons: config.weapons,
            current_weapon_key,
            fire_timer: Timer::new(Duration::from_millis(0), TimerMode::Once),
            num_left_in_burst: 0,
        }
    }

    /// returns true if fired
    pub fn tick(
        &mut self,
        config: WeaponContainerTickConfig,
        time: &Res<Time>,
        idpooler: &mut IdPooler,
        spawns: &mut EventWriter<SpawnProjectileEvent>,
        audio_events: &mut EventWriter<AudioEvent>,
        weapon_configs: &Res<Assets<WeaponConfig>>,
    ) -> bool {
        self.fire_timer.tick(time.delta());

        let mut current_weapon_opt = None;
        let mut wish_weapon_opt = None;

        // Identify current and wish weapons
        for weapon in self.weapons.iter_mut() {
            if weapon.key == self.current_weapon_key {
                current_weapon_opt = Some(weapon);
            } else if let Some(wish_weapon_kind) = &config.wish_weapon_key {
                if weapon.key == *wish_weapon_kind {
                    wish_weapon_opt = Some(weapon);
                }
            }
        }

        if let Some(mut current_weapon) = current_weapon_opt {
            if let Some(mut current_weapon_config) =
                WeaponConfig::get(current_weapon.key, weapon_configs)
            {
                let in_burst = if let Some(burst_config) = &current_weapon_config.burst_config {
                    self.num_left_in_burst > 0
                        && self.num_left_in_burst < burst_config.shots_per_burst
                } else {
                    false
                };

                // Check if it's time to switch weapons or initiate a burst
                if self.fire_timer.finished() {
                    // Switch weapons if applicable
                    if let Some(wish_weapon) = wish_weapon_opt {
                        if let Some(wish_weapon_config) =
                            WeaponConfig::get(wish_weapon.key, weapon_configs)
                        {
                            if wish_weapon.key != self.current_weapon_key {
                                self.current_weapon_key = wish_weapon.key;
                                self.num_left_in_burst = wish_weapon_config
                                    .burst_config
                                    .as_ref()
                                    .map_or(0, |b| b.shots_per_burst);
                                current_weapon = wish_weapon;
                                current_weapon_config = wish_weapon_config;
                            }
                        }
                    } else if !in_burst {
                        // Not in burst, initiate burst if applicable
                        self.num_left_in_burst = current_weapon_config
                            .burst_config
                            .as_ref()
                            .map_or(0, |b| b.shots_per_burst);
                    }
                }

                // Fire if it's time to fire or in the middle of a burst
                if (config.try_fire || in_burst) && self.fire_timer.finished() {
                    self.fire_timer.reset();
                    self.fire_timer.set_duration(Duration::from_millis(
                        current_weapon_config.fire_rate_millis as u64,
                    ));

                    if let Some(burst_config) = &current_weapon_config.burst_config {
                        if in_burst {
                            self.num_left_in_burst -= 1;

                            if self.num_left_in_burst > 0 {
                                self.fire_timer.set_duration(Duration::from_millis(
                                    burst_config.burst_rate_millis as u64,
                                ));
                            }
                        } else {
                            self.num_left_in_burst = burst_config.shots_per_burst - 1; // First shot fired, decrement burst count
                            self.fire_timer.set_duration(Duration::from_millis(
                                burst_config.burst_rate_millis as u64,
                            ));
                        }
                    }

                    // Fire the weapon
                    current_weapon.fire(
                        &FireWeapon {
                            proj_nudge_time_millis: config
                                .projectile_nudge_time_millis
                                .unwrap_or_default(),
                            owner_id: self.owner_id,
                            input_id: config.input_id,
                            origin_transform: config.origin_transform,
                            predicted: config.predict,
                            seed: config.seed,
                        },
                        idpooler,
                        spawns,
                        audio_events,
                        current_weapon_config,
                    );

                    return true;
                }
            }
        }
        false
    }

    pub fn server_process_player_fired_event(
        &mut self,
        config: WeaponContainerTickConfig,
        event: Option<&NetPlayerFiredEvent>,
        idpooler: &mut IdPooler,
        spawns: &mut EventWriter<SpawnProjectileEvent>,
        audio_events: &mut EventWriter<AudioEvent>,
        weapon_configs: &Res<Assets<WeaponConfig>>,
    ) -> bool {
        if let Some(event) = event {
            if let Some(weapon_fired_key) = &event.input.weapon_key {
                if let Some(weapon_fired) = self.weapons.iter().find(|w| w.key == *weapon_fired_key)
                {
                    if let Some(weapon_config) =
                        WeaponConfig::get(weapon_fired.key, &weapon_configs)
                    {
                        weapon_fired.fire(
                            &FireWeapon {
                                proj_nudge_time_millis: config
                                    .projectile_nudge_time_millis
                                    .unwrap_or_default(),
                                owner_id: self.owner_id,
                                input_id: config.input_id,
                                origin_transform: config.origin_transform,
                                predicted: config.predict,
                                seed: config.seed,
                            },
                            idpooler,
                            spawns,
                            audio_events,
                            weapon_config,
                        );
                        return true;
                    }
                }
            }
        }

        false
    }
}

pub struct Weapon {
    pub key: u8,
    pub ammo: Option<u16>,
}

impl Weapon {
    pub fn new(key: u8, ammo: Option<u16>) -> Self {
        Self { key, ammo: ammo }
    }

    pub fn fire(
        &self,
        fire_weapon: &FireWeapon,
        id_tracker: &mut IdPooler,
        spawns: &mut EventWriter<SpawnProjectileEvent>,
        audio_events: &mut EventWriter<AudioEvent>,
        config: &WeaponConfig,
    ) {
        for barrel_pos in config.barrel_positions.iter() {
            let mut seedable = rand::rngs::StdRng::seed_from_u64(fire_weapon.seed as u64);

            audio_events.send(AudioEvent {
                event_name: config.fire_sound.clone(),
                translation: fire_weapon.origin_transform.translation,
                first_person: fire_weapon.predicted,
                randomness: rand::rngs::ThreadRng::default().gen_range(0.0..1.0),
            });

            for _ in 0..config.projectiles_per_barrel {
                Self::fire_projectile(
                    &config,
                    fire_weapon,
                    &FireProjectile {
                        weapon_key: self.key,
                        barrel_position: *barrel_pos,
                        // spread
                        relative_rotation: Quat::from_rotation_x(
                            (seedable.gen_range(0.0..1.0) - 0.5)
                                * config.spread
                                * std::f32::consts::PI
                                * 2.0,
                        ) * Quat::from_rotation_y(
                            (seedable.gen_range(0.0..1.0) - 0.5)
                                * config.spread
                                * std::f32::consts::PI
                                * 2.0,
                        ) * Quat::from_rotation_z(
                            (seedable.gen_range(0.0..1.0) - 0.5)
                                * config.spread
                                * std::f32::consts::PI
                                * 2.0,
                        ),
                    },
                    id_tracker,
                    spawns,
                );
            }
        }
    }

    fn fire_projectile(
        config: &WeaponConfig,
        fire_weapon: &FireWeapon,
        fire_projectile: &FireProjectile,
        id_tracker: &mut IdPooler,
        events: &mut EventWriter<SpawnProjectileEvent>,
    ) {
        events.send(SpawnProjectileEvent {
            id: id_tracker.next_projectile_id(),
            weapon_key: fire_projectile.weapon_key,
            velocity: fire_projectile.relative_rotation
                * (fire_weapon.origin_transform.rotation
                    * -Vec3::Z
                    * config.projectile_config.speed),
            spawn_translation: fire_weapon.origin_transform.translation
                + fire_weapon.origin_transform.rotation * fire_projectile.barrel_position,
            current_translation: fire_weapon.origin_transform.translation
                + fire_weapon.origin_transform.rotation * fire_projectile.barrel_position,
            owner_id: fire_weapon.owner_id,
            input_id: fire_weapon.input_id,
            predicted: fire_weapon.predicted,
            nudge_delta_millis: fire_weapon.proj_nudge_time_millis,
        });
    }

    pub fn has_ammo(&self) -> bool {
        if let Some(ammo) = self.ammo {
            ammo > 0
        } else {
            true
        }
    }
}

pub fn weapons_system(
    time: Res<Time>,
    local_player: Res<LocalPlayer>,
    mut client_fired_events: EventReader<NetPlayerFiredEvent>,
    mut idpooler: ResMut<IdPooler>,
    mut shotgun_fire_events: EventWriter<ShotgunFireEvent>,
    mut spawn_events: EventWriter<SpawnProjectileEvent>,
    mut saved_inputs: ResMut<SavedInputs>,
    mut audio_events: EventWriter<AudioEvent>,
    mut player_extras: Query<(
        &Health,
        &Transform,
        &mut MovementState,
        &mut WeaponContainer,
    )>,
    players: Query<(Entity, &Player)>,
    weapon_configs: Res<Assets<WeaponConfig>>,
) {
    // server
    if local_player.has_authority() {
        let player_fired_events = client_fired_events.read().collect::<Vec<_>>();
        for (entity, player) in players.iter() {
            // local player is handled separately
            if player.id == local_player.player_id {
                continue;
            }

            if let Ok((health, transform, mut movement_state, mut weapon_container)) =
                player_extras.get_mut(entity)
            {
                if health.dead() {
                    continue;
                }

                let event = player_fired_events
                    .iter()
                    .find(|ev| ev.player_id == player.id);

                if weapon_container.server_process_player_fired_event(
                    WeaponContainerTickConfig {
                        try_fire: event.is_some(),
                        wish_weapon_key: {
                            if let Some(ev) = event {
                                ev.input.wish_weapon_key.clone()
                            } else {
                                None
                            }
                        },
                        input_id: {
                            if let Some(ev) = event {
                                Some(ev.input_id)
                            } else {
                                None
                            }
                        },
                        origin_transform: transform,
                        movement_state: Some(&mut movement_state),
                        projectile_nudge_time_millis: {
                            if let Some(ev) = event {
                                Some(ev.projectile_nudge_time_millis)
                            } else {
                                None
                            }
                        },
                        predict: false,
                        seed: {
                            if let Some(ev) = event {
                                ev.input_id
                            } else {
                                0
                            }
                        },
                    },
                    event.copied(),
                    &mut idpooler,
                    &mut spawn_events,
                    &mut audio_events,
                    &weapon_configs,
                ) {
                    if let Some(event) = event {
                        // special event for shotguns
                        if weapon_container.current_weapon_key == WEAPON_KEY_SHOTGUN {
                            shotgun_fire_events.send(ShotgunFireEvent {
                                origin_transform: transform.clone(),
                                seed: event.input_id,
                                owner_id: player.id,
                                owner_type: OwnerType::Player,
                            });
                        }
                    }
                }
            }
        }
    }

    // local player
    if let Some((entity, player)) = players
        .iter()
        .find(|(_, player)| player.id == local_player.player_id)
    {
        if let Ok((health, transform, mut movement_state, mut weapon_container)) =
            player_extras.get_mut(entity)
        {
            if health.dead() {
                return;
            }

            if let Some(latest_input) = saved_inputs.latest_input() {
                let fired = weapon_container.tick(
                    WeaponContainerTickConfig {
                        try_fire: latest_input.input.holding_down_fire,
                        wish_weapon_key: latest_input.input.wish_weapon_key.clone(),
                        input_id: Some(latest_input.input.id),
                        origin_transform: transform,
                        movement_state: Some(&mut movement_state),
                        projectile_nudge_time_millis: None,
                        predict: true,
                        seed: latest_input.input.id,
                    },
                    &time,
                    &mut idpooler,
                    &mut spawn_events,
                    &mut audio_events,
                    &weapon_configs,
                );

                if fired {
                    if let Some(latest_input) = saved_inputs.latest_input_mut() {
                        latest_input.input.weapon_key =
                            Some(weapon_container.current_weapon_key.clone());

                        // special event for shotguns
                        if local_player.has_authority()
                            && weapon_container.current_weapon_key == WEAPON_KEY_SHOTGUN
                        {
                            shotgun_fire_events.send(ShotgunFireEvent {
                                origin_transform: transform.clone(),
                                seed: latest_input.input.id,
                                owner_id: player.id,
                                owner_type: OwnerType::Player,
                            });
                        }
                    }
                }
            }
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct Projectile {
    pub id: u16,
    pub input_id: Option<u64>,
    pub weapon_key: u8,
    pub velocity: Vec3,
    pub owner_id: u8,
    pub owner_entity: Entity,
    pub spawn_translation: Vec3,
    pub flight_timer: Timer,

    // networking
    pub nudge_delta_millis: u8,
    pub queued_for_deletion: bool,
}
impl Projectile {
    pub fn update_systems() -> NodeConfigs<Box<dyn System<In = (), Out = ()>>> {
        (Projectile::spawn_system, Projectile::despawn_system).chain()
    }

    pub fn fixed_update_systems() -> NodeConfigs<Box<dyn System<In = (), Out = ()>>> {
        (Projectile::move_and_hit_system,).chain()
    }

    pub fn spawn_system(
        mut commands: Commands,
        mut spawns: EventReader<SpawnProjectileEvent>,
        mut spawn_visuals: EventWriter<SpawnProjectileVisualsEvent>,
        players: Query<(Entity, &Player)>,
        bots: Query<(Entity, &Monster)>,
    ) {
        for event in spawns.read() {
            // find the owner
            // we do it this way for multiplayer reasons (cant send Entity over the network because they wont be in sync)
            let owner_entity = event.get_owner_entity(&players, &bots);
            if owner_entity.is_none() {
                continue;
            }

            let projectile = spawn_gameplay_entity(
                &mut commands,
                (
                    Projectile {
                        id: event.id,
                        input_id: event.input_id,
                        weapon_key: event.weapon_key,
                        velocity: event.velocity,
                        owner_entity: owner_entity.unwrap(),
                        owner_id: event.owner_id,
                        flight_timer: Timer::from_seconds(5.0, TimerMode::Once),
                        nudge_delta_millis: event.nudge_delta_millis,
                        spawn_translation: event.spawn_translation,
                        queued_for_deletion: false,
                    },
                    TransformBundle {
                        local: Transform::from_translation(event.current_translation),
                        ..default()
                    },
                ),
            );

            // if it's a fake projectile, mark it as so
            if event.predicted {
                commands.entity(projectile).insert((
                    PredictedProjectile {
                        input_id: event.input_id.unwrap_or_default(),
                        id: event.id,
                        ..default()
                    },
                    // start it off hidden so we can control when it becomes visible
                    VisibilityBundle {
                        visibility: Visibility::Hidden,
                        ..default()
                    },
                ));
            }

            spawn_visuals.send(SpawnProjectileVisualsEvent {
                entity: projectile,
                weapon_key: event.weapon_key.clone(),
            });
        }
    }

    pub fn despawn_system(
        mut commands: Commands,
        mut despawns: EventReader<DespawnProjectileEvent>,
        projectiles: Query<(Entity, &Projectile)>,
    ) {
        for event in despawns.read() {
            for (entity, projectile) in projectiles.iter() {
                if projectile.id == event.id {
                    if let Some(e) = commands.get_entity(entity) {
                        e.despawn_recursive();
                    }
                }
            }
        }
    }

    pub fn move_and_hit_system(
        time: Res<Time>,
        physics_context: Res<RapierContext>,
        local_player: Res<LocalPlayer>,
        world_state: Res<State<WorldState>>,
        mut projectiles: Query<(Entity, &mut Projectile, &mut Transform)>,
        mut ev_writer: EventWriter<DamageEvent>,
        mut despawns: EventWriter<DespawnProjectileEvent>,
        mut movement_states: Query<&mut MovementState>,
        damageables: Query<(Entity, &Transform), (With<Health>, Without<Projectile>)>,
        snapshot_history: Res<SnapshotHistory>,
        configs: Res<Assets<WeaponConfig>>,
    ) {
        for (_, mut projectile, mut transform) in projectiles.iter_mut() {
            let tick_projectile = projectile.flight_timer.tick(time.delta());
            if tick_projectile.just_finished() && !projectile.queued_for_deletion {
                projectile.queued_for_deletion = true;
                despawns.send(DespawnProjectileEvent {
                    id: projectile.id,
                    input_id: projectile.input_id,
                    spawn_translation: projectile.spawn_translation,
                    velocity: projectile.velocity,
                    weapon_key: projectile.weapon_key,
                    hit_damageable: false,
                    normal: Vec3::ZERO,
                    translation: transform.translation,
                });
            } else {
                let mut dt = time.delta_seconds();

                // server bumps the time forward by the nudge amount (owner's half ping)
                if projectile.nudge_delta_millis > 0 && local_player.has_authority() {
                    if projectile.flight_timer.elapsed() <= Duration::from_millis(0) {
                        dt += projectile.nudge_delta_millis as f32 * 0.001;
                    }
                }

                let mut translation = transform.translation;
                let mut velocity = projectile.velocity;

                // single player uses the latest snapshot
                let snapshot = if world_state.get() == &WorldState::SinglePlayer {
                    snapshot_history.latest()
                }
                // multiplayer uses the second latest because the clients
                // are roughly one frame behind due to interpolation
                // yes this means the server player's projectiles will
                // technically be getting compared to the past instead of the present, but whatever.
                else {
                    snapshot_history.second_latest()
                };

                if let Some(result) = Projectile::compute_trajectory(
                    dt,
                    0.0,
                    &mut velocity,
                    &mut translation,
                    &physics_context,
                    projectile.owner_entity,
                    snapshot,
                    &local_player,
                ) {
                    let mut damaged_entity = None;

                    if let Some(weapon_config) = WeaponConfig::get(projectile.weapon_key, &configs)
                    {
                        if let Ok(_) = damageables.get(result.entity) {
                            damaged_entity = Some(result.entity);
                            ev_writer.send(DamageEvent {
                                amount: weapon_config.projectile_config.damage,
                                victim: result.entity,
                                instigator: projectile.owner_entity,
                            });
                        }

                        if !projectile.queued_for_deletion {
                            projectile.queued_for_deletion = true;

                            despawns.send(DespawnProjectileEvent {
                                id: projectile.id,
                                input_id: projectile.input_id,
                                spawn_translation: projectile.spawn_translation,
                                velocity: projectile.velocity,
                                weapon_key: projectile.weapon_key.clone(),
                                hit_damageable: damaged_entity.is_some(),
                                normal: result.normal,
                                translation: result.position,
                            });

                            // apply splash damage
                            if local_player.has_authority()
                                && weapon_config.projectile_config.splash_damage_radius > 0.0
                            {
                                for (victim, victim_transform) in damageables.iter() {
                                    // don't damage someone twice
                                    if let Some(damaged_entity) = damaged_entity {
                                        if damaged_entity == victim {
                                            continue;
                                        }
                                    }

                                    if translation.distance(victim_transform.translation)
                                        <= weapon_config.projectile_config.splash_damage_radius
                                    {
                                        // falloff damage (linear)
                                        let distance =
                                            translation.distance(victim_transform.translation);
                                        let falloff = 1.0
                                            - (distance
                                                / weapon_config
                                                    .projectile_config
                                                    .splash_damage_radius);
                                        ev_writer.send(DamageEvent {
                                            amount: (weapon_config.projectile_config.damage as f32
                                                * falloff)
                                                as i16,
                                            victim: victim,
                                            instigator: projectile.owner_entity,
                                        });

                                        // apply knockback force
                                        if let Ok(mut victim_movement_state) =
                                            movement_states.get_mut(victim)
                                        {
                                            let knockback_direction =
                                                (victim_transform.translation - translation)
                                                    .normalize_or_zero();
                                            let knockback = (knockback_direction
                                                * weapon_config
                                                    .projectile_config
                                                    .splash_damage_knockback)
                                                * falloff;
                                            victim_movement_state.velocity += knockback;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                transform.translation = translation;
                projectile.velocity = velocity;
            }
        }
    }

    // returns the entity we hit and position + normal
    fn compute_trajectory(
        dt: f32,
        radius: f32,
        velocity: &mut Vec3,
        translation: &mut Vec3,
        physics_context: &Res<RapierContext>,
        exclude: Entity,
        snapshot: Option<&Snapshot>,
        local_player: &Res<LocalPlayer>,
    ) -> Option<ProjectileHitResult> {
        // only hit walls
        let filter = QueryFilter {
            flags: QueryFilterFlags::EXCLUDE_SENSORS,
            exclude_collider: Some(exclude),
            groups: Some(CollisionGroups::new(
                crate::physics::COLLISION_GROUP_DYNAMIC,
                crate::physics::COLLISION_GROUP_NO_COLLISION,
            )),
            ..default()
        };

        let ray_pos = translation.clone();
        let ray_dir = velocity.normalize();
        let max_toi = velocity.length() * dt;

        let stop_at_penetration = true;
        if let Some((entity, toi)) = physics_context.cast_shape(
            ray_pos,
            Quat::IDENTITY,
            ray_dir,
            &Collider::ball(radius),
            max_toi,
            stop_at_penetration,
            filter,
        ) {
            let wall_hit_toi = toi.toi;

            if local_player.has_authority() {
                if let Some(snapshot) = snapshot {
                    if let Some(enemy_hit) = Projectile::try_hit_any_enemy(
                        exclude,
                        snapshot,
                        ray_pos,
                        ray_dir,
                        wall_hit_toi,
                    ) {
                        // hit an enemy
                        return Some(ProjectileHitResult {
                            entity: enemy_hit.entity,
                            position: ray_pos + ray_dir * enemy_hit.toi,
                            normal: enemy_hit.normal,
                        });
                    }
                }
            }

            // hit a wall
            return Some(ProjectileHitResult {
                entity: entity,
                position: ray_pos + ray_dir * wall_hit_toi,
                normal: toi.details.map(|d| d.normal1).unwrap_or(Vec3::ZERO),
            });
        } else {
            if local_player.has_authority() {
                if let Some(snapshot) = snapshot {
                    if let Some(enemy_hit) =
                        Projectile::try_hit_any_enemy(exclude, snapshot, ray_pos, ray_dir, max_toi)
                    {
                        // hit an enemy
                        return Some(ProjectileHitResult {
                            entity: enemy_hit.entity,
                            position: ray_pos + ray_dir * enemy_hit.toi,
                            normal: enemy_hit.normal,
                        });
                    }
                }
            }
        }

        // hit nothing
        *translation += *velocity * dt;
        None
    }

    fn try_hit_any_enemy(
        exclude: Entity,
        snapshot: &Snapshot,
        ray_pos: Vec3,
        ray_dir: Vec3,
        max_toi: f32,
    ) -> Option<EnemyHit> {
        for player in &snapshot.players {
            if let Some(translation) = player.translation {
                if let Some(rotation) = player.rotation() {
                    // don't hit the projectile's owner
                    if let Some(entity) = player.entity {
                        if exclude == entity {
                            continue;
                        }
                    }

                    // only consider players that are in front of the ray
                    let to_player = translation - ray_pos;
                    let to_player = to_player.normalize();
                    let dot = to_player.dot(ray_dir);

                    if dot > 0.0 {
                        let collider = Collider::ball(0.75);
                        if let Some(intersection) = collider.cast_ray_and_get_normal(
                            translation,
                            rotation,
                            ray_pos,
                            ray_dir,
                            max_toi,
                            true,
                        ) {
                            if let Some(entity) = player.entity {
                                return Some(EnemyHit {
                                    entity: entity,
                                    toi: intersection.toi,
                                    normal: intersection.normal,
                                });
                            }
                        }
                    }
                }
            }
        }

        for bot in &snapshot.monsters {
            if let Some(translation) = bot.translation() {
                if let Some(rotation) = bot.rotation() {
                    // don't hit the projectile's owner
                    if let Some(entity) = bot.entity {
                        if exclude == entity {
                            continue;
                        }
                    }

                    // only consider bots that are in front of the ray
                    let to_bot = translation - ray_pos;
                    let to_bot = to_bot.normalize();
                    let dot = to_bot.dot(ray_dir);

                    if dot > 0.0 {
                        let collider = Collider::ball(0.75);
                        if let Some(intersection) = collider.cast_ray_and_get_normal(
                            translation,
                            rotation,
                            ray_pos,
                            ray_dir,
                            max_toi,
                            true,
                        ) {
                            if let Some(entity) = bot.entity {
                                return Some(EnemyHit {
                                    entity: entity,
                                    toi: intersection.toi,
                                    normal: intersection.normal,
                                });
                            }
                        }
                    }
                }
            }
        }

        None
    }
}

#[derive(Component, Default)]
pub struct ProjectileFx {
    pub needs_smoothing: bool,
    pub smooth_timer: Timer,
}

impl ProjectileFx {
    pub fn systems() -> NodeConfigs<Box<dyn System<In = (), Out = ()>>> {
        (Self::spawn_system, Self::tick_system, Self::despawn_system).chain()
    }

    pub fn tick_system(
        time: Res<Time>,
        mut commands: Commands,
        mut projectiles: Query<(Entity, &Transform, &Projectile), With<Visibility>>,
        mut projectile_visuals: Query<&mut Transform, (With<ProjectileFx>, Without<Projectile>)>,
    ) {
        for (entity, transform, projectile) in projectiles.iter_mut() {
            if transform.translation.distance(projectile.spawn_translation)
                > SELF_PROJECTILE_VISIBLE_DISTANCE
            {
                commands.entity(entity).try_insert(Visibility::Visible);
            }
        }

        for mut transform in projectile_visuals.iter_mut() {
            transform.translation = transform
                .translation
                .lerp(Vec3::ZERO, (2.0 * time.delta_seconds()).min(1.0));
        }
    }

    pub fn spawn_system(
        mut commands: Commands,
        mut events: EventReader<SpawnProjectileVisualsEvent>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut plasma_materials: ResMut<Assets<PlasmaMaterial>>,
        studio: Res<FmodStudio>,
        weapon_configs: Res<Assets<WeaponConfig>>,
    ) {
        for event in events.read() {
            if let Some(mut entity) = commands.get_entity(event.entity) {
                entity.with_children(|parent| {
                    if let Some(weapon_config) =
                        WeaponConfig::get(event.weapon_key, &weapon_configs)
                    {
                        if let Some(flight_sound) = &weapon_config.projectile_config.fx.flight_sound
                        {
                            if let Ok(event_description) = studio.0.get_event(flight_sound) {
                                parent.spawn(SpatialAudioBundle::new(event_description));
                            }
                        }

                        let material = plasma_materials.add(PlasmaMaterial {
                            color: weapon_config.projectile_config.fx.color,
                        });

                        let mesh = meshes.add(Mesh::from(Rectangle {
                            half_size: Vec2::splat(weapon_config.projectile_config.fx.half_size),
                            ..default()
                        }));

                        parent
                            .spawn((
                                Billboard,
                                ProjectileFx {
                                    smooth_timer: Timer::from_seconds(0.5, TimerMode::Once),
                                    ..default()
                                },
                                MaterialMeshBundle {
                                    mesh: mesh,
                                    material: material,
                                    ..default()
                                },
                                NotShadowCaster,
                            ))
                            .with_children(|children| {
                                if let Some(light_color) =
                                    weapon_config.projectile_config.fx.light_color
                                {
                                    children.spawn(PointLightBundle {
                                        point_light: PointLight {
                                            color: light_color,
                                            intensity: weapon_config
                                                .projectile_config
                                                .fx
                                                .light_intensity
                                                .unwrap_or(8.0),
                                            ..default()
                                        },
                                        ..default()
                                    });
                                }
                            });
                    }
                });
            }
        }
    }

    pub fn despawn_system(
        mut despawns: EventReader<DespawnProjectileEvent>,
        mut particle_events: EventWriter<ParticleEvent>,
        mut decal_events: EventWriter<DecalEvent>,
        mut audio_events: EventWriter<AudioEvent>,
        configs: Res<Assets<WeaponConfig>>,
    ) {
        for event in despawns.read() {
            if let Some(weapon_config) = WeaponConfig::get(event.weapon_key, &configs) {
                if let Some(explosion_config) = &weapon_config.projectile_config.fx.hit_particles {
                    let mut explosion = explosion_config.clone();
                    // places it slightly above the surface
                    explosion.translation = event.translation + (event.normal * 0.1);
                    explosion.direction = Some(event.normal);
                    particle_events.send(ParticleEvent::Explosion(explosion.clone()));
                }

                if let Some(explosion_sound) = &weapon_config.projectile_config.fx.hit_sound {
                    audio_events.send(AudioEvent {
                        event_name: explosion_sound.clone(),
                        translation: event.translation,
                        first_person: false,
                        randomness: rand::rngs::ThreadRng::default().gen_range(0.0..1.0),
                    });
                }

                if !event.hit_damageable {
                    if let Some(decal_kind) = &weapon_config.projectile_config.fx.impact_decal {
                        decal_events.send(DecalEvent {
                            translation: event.translation,
                            normal: event.normal,
                            kind: decal_kind.clone(),
                        });
                    }
                }
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct WeaponAssetHandles {
    pub assets: Vec<Handle<WeaponConfig>>,
}

pub fn load_weapon_assets_system(
    asset_server: Res<AssetServer>,
    mut weapon_assets: ResMut<WeaponAssetHandles>,
) {
    let weapon_names = [
        "standard_plasma",
        "dual_lasers",
        "shotgun",
        "rocket_launcher",
        "hyper_plasma",
        "grunt_plasma",
        "grunt_lasers",
        "grunt_fusion",
    ];

    for weapon_name in weapon_names {
        // load them
        weapon_assets
            .assets
            .push(asset_server.load(format!("data/weapons/{weapon_name}.weapon.ron")));
    }
}
