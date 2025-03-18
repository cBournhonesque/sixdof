use std::time::Duration;

use avian3d::prelude::*;
use bevy::{prelude::*, utils::HashMap};
use bevy::ecs::entity::MapEntities;
use bevy_config_stack::prelude::{ConfigAssetLoadedEvent, ConfigAssetLoaderPlugin};
use leafwing_input_manager::prelude::ActionState;
use lightyear::channel::builder::EntityActionsChannel;
use lightyear::client::prediction::Predicted;
use lightyear::prelude::*;
use lightyear::prelude::client::{Confirmed, Rollback};
use lightyear::prelude::server::{ControlledBy, Replicate, ReplicationTarget, SyncTarget};
use serde::{Deserialize, Serialize};

use crate::{data::weapons::*, prelude::{PlayerInput, UniqueIdentity}, utils::DespawnAfter};
use crate::prelude::{GameLayer, WeaponFiredChannel, PREDICTION_REPLICATION_GROUP_ID};

pub type WeaponId = u32;

pub(crate) struct WeaponsPlugin;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum WeaponsSet {
    Shoot,
}

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        
        // let default_weapons_list = WeaponsListConfig::default();
        // let default_weapons_list_ron = to_string_pretty(&default_weapons_list, PrettyConfig::default()).unwrap();
        // println!("{}", default_weapons_list_ron);

        app.add_event::<ProjectileHitEvent>();
        app.add_plugins(ConfigAssetLoaderPlugin::<WeaponsData>::new("data/weapons.ron"));
        app.add_systems(FixedUpdate, update_weapon_timers_system.run_if(resource_exists::<WeaponsData>));

        app.add_observer(spawn_projectiles);

        // NOTE: keep around these debug systems for now
        // app.add_systems(PreUpdate, debug_projectiles_before_check_rollback
        //     .run_if(is_client)
        //     .after(MainSet::Receive)
        //     .after(PredictionSet::Sync)
        //     .before(PredictionSet::CheckRollback));
        // app.add_systems(PreUpdate, debug_projectiles_on_confirmed_added
        //     .run_if(is_client)
        //     .after(MainSet::Receive)
        //     .after(PredictionSet::SpawnPrediction)
        //     .before(PredictionSet::CheckRollback));
        // app.add_systems(FixedPostUpdate, debug_projectiles
        //     .after(PhysicsSet::StepSimulation)
        //     .after(PredictionSet::UpdateHistory));
    }
}


/// Event triggered when a weapon is fired
/// Can be used to play a firing sound, etc.
#[derive(Event, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeaponFiredEvent {
    /// The identity of the shooter.
    pub shooter_id: UniqueIdentity,
    /// The index of the weapon that was fired.
    pub weapon_index: u32,
    /// The entity that fired the weapon. Used for things like firing sounds & VFX following the shooter.
    pub shooter_entity: Entity,
    /// The absolute origin of the fire. Used for knowing which location to spawn VFX & sounds.
    pub fire_origin: Vec3,
    /// The rotation of the shooter at the time of firing. Used for knowing which direction to spawn VFX.
    pub shooter_rotation: Rotation,
    /// The tick at which the bullet was fired
    pub fire_tick: Tick,
}

impl MapEntities for WeaponFiredEvent {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
        self.shooter_entity = entity_mapper.map_entity(self.shooter_entity);
    }
}


/// Component added on the projectile entity when a weapon is fired.
/// We use this as a component and not an event because we need it on the entity itself
/// (for interpolation + lag compensation)
/// Can be used to play a firing sound, etc.
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectileInfo {
    /// The identity of the shooter.
    pub shooter_id: UniqueIdentity,
    /// The entity that fired the weapon. Used for things like firing sounds & VFX following the shooter.
    pub shooter_entity: Entity,
    /// The index of the weapon that was fired.
    pub weapon_index: u32,
}

impl MapEntities for ProjectileInfo {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
        self.shooter_entity = entity_mapper.map_entity(self.shooter_entity);
    }
}



/// Event that is sent when a projectile hits an entity.
/// Can be used to spawn vfx and play sfx, apply damage, etc.
#[derive(Event, Clone, Debug)]
pub struct ProjectileHitEvent {
    pub shooter_id: UniqueIdentity,
    pub weapon_index: u32,
    pub projectile_entity: Entity,
    pub entity_hit: Option<Entity>,
}

#[derive(Component, Serialize, Deserialize, PartialEq, Clone)]
pub struct CurrentWeaponIndex(pub WeaponId);

impl CurrentWeaponIndex {
    // TODO: this is inefficient, just make sure that weapons ids are sequential
    //  and select the next one with wrapping
    /// Cycle to the next weapon in the provided weapon list. Wraps around.
    pub fn next_weapon(&mut self, weapons: &HashMap<WeaponId, Weapon>) {
        let max_id = weapons.keys().max().unwrap_or(&0);
        for i in 0..=*max_id {
            let new_idx = (self.0 + i + 1) % (max_id + 1);
            if weapons.contains_key(&new_idx) {
                self.0 = new_idx;
                break;
            }
        }
    }

    // TODO: this is inefficient, just make sure that weapons ids are sequential
    //  and select the next one with wrapping
    /// Cycle to the previous weapon in the provided weapon list. Wraps around.
    pub fn previous_weapon(&mut self, weapons: &HashMap<WeaponId, Weapon>) {
        let max_id = weapons.keys().max().unwrap_or(&0);
        for i in 0..=*max_id {
            let new_idx = self.0.checked_sub(i + 1).unwrap_or(*max_id);
            if weapons.contains_key(&new_idx) {
                self.0 = new_idx;
                break;
            }
        }
    }
}

#[derive(Component, Serialize, Deserialize, PartialEq, Clone)]
pub struct WeaponInventory {
    pub weapons: HashMap<WeaponId, Weapon>,
}

impl WeaponInventory {
    pub fn from_data(
        weapons_data: &WeaponsData, 
        // Indices of the weapons that we grant access to.
        granted_weapon_indices: Vec<WeaponId>
    ) -> Self {
        let mut weapons = HashMap::new();
        for weapon_idx in granted_weapon_indices {
            if let Some(weapon_data) = weapons_data.weapons.get(&weapon_idx) {
                weapons.insert(weapon_idx, Weapon::from_data(weapon_data));
            }
        }
        Self { weapons }
    }
}

/// A weapon component defines the state of a weapon.
/// 
/// @todo-brian: maybe this can eventually be a component too, 
/// so that if we die and drop the weapon, it can be picked up 
/// by someone else with the exact state it was left off with.
#[derive(Component, Default, Serialize, Deserialize, PartialEq, Clone)]
pub struct Weapon {
    pub fire_timer_auto: Timer,
    pub fire_timer_burst: Timer,
    pub fire_timer_post_burst: Timer,
    pub burst_shots_left: u32,
    pub ammo_left: u32,
}

impl Weapon {
    pub fn from_data(weapon_data: &WeaponBehavior) -> Self {
        // sane defaults
        let mut fire_timer_auto = Timer::new(Duration::from_millis(750), TimerMode::Once);
        let mut fire_timer_burst = Timer::new(Duration::from_millis(100), TimerMode::Once);
        let mut fire_timer_post_burst = Timer::new(Duration::from_millis(750), TimerMode::Once);
        let mut burst_shots_left = 3;
        let ammo_left = weapon_data.starting_ammo;

        match weapon_data.fire_mode {
            FireMode::Auto { delay_millis } => {
                fire_timer_auto.set_duration(Duration::from_millis(delay_millis));
            }
            FireMode::Burst { shots, delay_millis, delay_after_burst_millis } => {
                fire_timer_burst.set_duration(Duration::from_millis(delay_millis));
                fire_timer_post_burst.set_duration(Duration::from_millis(delay_after_burst_millis));
                burst_shots_left = shots;
            }
        }

        Self {
            fire_timer_auto,
            fire_timer_burst,
            fire_timer_post_burst,
            burst_shots_left,
            ammo_left,
        }
    }
}

// TODO: maybe make this an enum with the type of projectile?
#[derive(Component, Debug, Clone, PartialEq, Deserialize, Serialize)]
// add a Transform for each Projectile. Otherwise projectiles that have a Position don't get a Transform..
#[require(Transform)]
pub struct Projectile;

/// The resource that contains all the weapon configurations.
#[derive(Resource, Asset, TypePath, Debug, Deserialize, Serialize)]
pub struct WeaponsData {
    pub weapons: HashMap<WeaponId, WeaponBehavior>,
}

impl Default for WeaponsData {
    fn default() -> Self {
        let mut weapons = HashMap::new();
        weapons.insert(0, WeaponBehavior::default());
        WeaponsData {
            weapons,
        }
    }
}

/// Updates the weapon timer durations when the weapon data is loaded/reloaded.
fn update_weapon_timers_system(
    weapons_data: Res<WeaponsData>,
    mut events: EventReader<ConfigAssetLoadedEvent<WeaponsData>>,
    mut weapon_inventories: Query<&mut WeaponInventory>,
) {
    // Our data has loaded/reloaded so update the weapon timer durations on every weapon in the world
    for _ in events.read() {
        for mut inventory in weapon_inventories.iter_mut() {
            for (weapon_idx, weapon_state) in inventory.weapons.iter_mut() {
                match &weapons_data.weapons[weapon_idx].fire_mode {
                    FireMode::Auto { delay_millis } => {
                        weapon_state.fire_timer_auto.set_duration(Duration::from_millis(*delay_millis));
                    }
                    FireMode::Burst { shots: _, delay_millis, delay_after_burst_millis } => {
                        weapon_state.fire_timer_burst.set_duration(Duration::from_millis(*delay_millis));
                        weapon_state.fire_timer_post_burst.set_duration(Duration::from_millis(*delay_after_burst_millis));
                    }
                }
            }
        }
    }
}

/// Generic function for shooting a weapon.
/// 
/// Should be called for both predicted and replicated entities.
pub fn handle_shooting(
    shooting_entity: Entity,
    identity: &UniqueIdentity,
    tick: Tick,
    is_server: bool,
    mut to_clients: Option<&mut Events<ToClients<WeaponFiredEvent>>>,
    shooter_position: &Position,
    shooter_rotation: &Rotation,
    current_weapon_idx: WeaponId,
    inventory: &mut WeaponInventory,
    action: &ActionState<PlayerInput>,
    fixed_time: &Time<Fixed>,
    weapons_data: &WeaponsData,
    commands: &mut Commands,
) {
    // grab the necessary data and state for the current weapon
    if let (Some(weapon_data), Some(weapon_state)) = (
        weapons_data.weapons.get(&current_weapon_idx), 
        inventory.weapons.get_mut(&current_weapon_idx)
    ) {
        let mut should_fire = false;
        match weapon_data.fire_mode {
            FireMode::Auto { delay_millis: _ } => {
                // TODO: the fire timer auto needs to be reset during rollbacks
                //  maybe lightyear should provide a rollbackable timer? or we can rollback the entire
                //  WeaponInventory component, but that might not be an efficient way to do it
                weapon_state.fire_timer_auto.tick(fixed_time.delta());
                
                // If the timer is finished (no cooldown) and button is pressed, fire
                if weapon_state.fire_timer_auto.finished() && action.pressed(&PlayerInput::ShootPrimary) {
                    should_fire = true;
                    weapon_state.fire_timer_auto.reset();
                }
            }
            FireMode::Burst { .. } => {
                // @todo-brian: implement burst mode
            }
        }

        if should_fire {
            match weapon_data.barrel_mode {
                BarrelMode::Simultaneous => {
                    // Trigger the event on the server and the client that fired the weapon
                    // For the remote players we want to fire the event at a delay in the interpolation timeline
                    let weapon_fired_event = WeaponFiredEvent {
                        shooter_id: identity.clone(),
                        weapon_index: current_weapon_idx,
                        shooter_entity: shooting_entity,
                        fire_origin: shooter_position.0,
                        shooter_rotation: *shooter_rotation,
                        fire_tick: tick,
                    };
                    commands.trigger(weapon_fired_event.clone());
                    debug!(?tick, "Firing weapon: {:?}", weapon_fired_event);
                    // also send an event for remote players
                    if is_server {
                        if let UniqueIdentity::Player(client_id) = identity {
                            to_clients.as_mut().unwrap().send(ToClients::new_with_target::<WeaponFiredChannel>(
                                weapon_fired_event, NetworkTarget::AllExceptSingle(*client_id)
                            ));
                        } else {
                            to_clients.as_mut().unwrap().send(ToClients::new_with_target::<WeaponFiredChannel>(
                                weapon_fired_event, NetworkTarget::All
                            ));
                        }
                    }

                }
                BarrelMode::Sequential => {
                    // @todo-brian: implement sequential mode
                }
            }
        }
    }
}

pub fn spawn_projectiles(
    trigger: Trigger<WeaponFiredEvent>,
    mut commands: Commands,
    weapons_data: Res<WeaponsData>,
) {
    let weapon_fired_event = trigger.event();
    if let Some(weapon_data) = weapons_data.weapons.get(&weapon_fired_event.weapon_index) {
        // spawn the projectiles
        let direction = weapon_fired_event.shooter_rotation.0 * Vec3::NEG_Z;
        for (_i, barrel_position) in weapon_data.barrel_positions.iter().enumerate() {
            let rotated_barrel_pos = weapon_fired_event.shooter_rotation * *barrel_position;
            let new_position = Position(weapon_fired_event.fire_origin + rotated_barrel_pos);
            commands.spawn((
                ProjectileInfo {
                    shooter_id: weapon_fired_event.shooter_id,
                    shooter_entity: weapon_fired_event.shooter_entity,
                    weapon_index: weapon_fired_event.weapon_index,
                },
                Projectile,
                // TODO(cb): we shouldn't need to include this Transform, because we have a position->transform system
                //  running in PostUpdate::PhysicsSet::SyncSet (which itself is before TransformPropagate) and we trigger
                //  the WeaponFiredEvent before that set. However it seems that otherwise the entity is visually at (0.0, 0.0, 0.0)
                //  for the first frame.
                Transform::from_translation(new_position.0),
                RigidBody::Dynamic,
                new_position,
                LinearVelocity(direction * weapon_data.projectile.speed),
                DespawnAfter(Timer::new(Duration::from_millis(weapon_data.projectile.lifetime_millis), TimerMode::Once)),
                // NOTE: we include collisions with players so that we can play VFX on the client, independently
                //  from hit data received from the server
                CollisionLayers::new([GameLayer::Projectile], [GameLayer::Ship, GameLayer::Wall]),
            ));
            debug!(?weapon_fired_event.fire_tick, "Shooting projectile at pos: {:?}", new_position);
        }
    }
}

/// Print the inputs at FixedUpdate, after they have been updated on the client/server
/// Also prints the Transform before `move_player` is applied (inputs handled)
pub fn debug_projectiles(
    tick_manager: Res<TickManager>,
    rollback: Option<Res<Rollback>>,
    query: Query<(Entity, (&Position, Option<&HistoryBuffer<Position>>, &LinearVelocity)),
        (Or<(With<Predicted>, With<PreSpawnedPlayerObject>, With<Replicating>)>, With<Projectile>)>
) {
    let tick = rollback.as_ref().map_or(tick_manager.tick(), |r| {
        tick_manager.tick_or_rollback_tick(r.as_ref())
    });
    let is_rollback = rollback.map_or(false, |r| r.is_rollback());
    for (entity, info) in query.iter() {
        info!(
            ?is_rollback,
            ?tick,
            ?entity,
            ?info,
            "FixedPostUpdate Projectile"
        );
    }
}

/// Print the inputs at FixedUpdate, after they have been updated on the client/server
/// Also prints the Transform before `move_player` is applied (inputs handled)
pub fn debug_projectiles_before_check_rollback(
    tick_manager: Res<TickManager>,
    rollback: Option<Res<Rollback>>,
    query: Query<(Entity, (&Position, Option<&HistoryBuffer<Position>>, &LinearVelocity)),
        (Or<(With<Predicted>, With<PreSpawnedPlayerObject>, With<Replicating>)>, With<Projectile>)>
) {
    let tick = rollback.as_ref().map_or(tick_manager.tick(), |r| {
        tick_manager.tick_or_rollback_tick(r.as_ref())
    });
    let is_rollback = rollback.map_or(false, |r| r.is_rollback());
    for (entity, info) in query.iter() {
        info!(
            ?is_rollback,
            ?tick,
            ?entity,
            ?info,
            "PreUpdate Projectile before CheckRollback"
        );
    }
}

/// Print the value of the projectile right after confirmed is added, after Receive and PredictionSpawn
pub fn debug_projectiles_on_confirmed_added(
    tick_manager: Res<TickManager>,
    rollback: Option<Res<Rollback>>,
    query: Query<(Entity, (&Confirmed, &Position)),
        (Added<Confirmed>, With<Projectile>)>
) {
    let tick = rollback.as_ref().map_or(tick_manager.tick(), |r| {
        tick_manager.tick_or_rollback_tick(r.as_ref())
    });
    let is_rollback = rollback.map_or(false, |r| r.is_rollback());
    for (entity, info) in query.iter() {
        info!(
            ?is_rollback,
            ?tick,
            ?entity,
            ?info,
            "PreUpdate Confirmed projectile upon replication"
        );
    }
}
