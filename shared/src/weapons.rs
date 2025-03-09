use std::time::Duration;

use avian3d::{math::Vector, prelude::*};
use bevy::{prelude::*, utils::HashMap};
use bevy_config_stack::prelude::{ConfigAssetLoadedEvent, ConfigAssetLoaderPlugin};
use leafwing_input_manager::prelude::ActionState;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{data::weapons::*, physics::GameLayer, prelude::{PlayerInput, UniqueIdentity}, utils::DespawnAfter};

pub type WeaponId = u32;

pub(crate) struct WeaponsPlugin;

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        
        // let default_weapons_list = WeaponsListConfig::default();
        // let default_weapons_list_ron = to_string_pretty(&default_weapons_list, PrettyConfig::default()).unwrap();
        // println!("{}", default_weapons_list_ron);

        app.add_event::<ProjectileHitEvent>();
        app.add_event::<WeaponFiredEvent>();
        app.add_plugins(ConfigAssetLoaderPlugin::<WeaponsData>::new("data/weapons.ron"));
        app.add_systems(FixedUpdate, update_weapon_timers_system.run_if(resource_exists::<WeaponsData>));
    }
}


/// Event that is sent when a weapon is fired.
/// Can be used to play a firing sound, etc.
#[derive(Event, Clone, Debug)]
pub struct WeaponFiredEvent {
    /// The identity of the shooter.
    pub shooter_id: UniqueIdentity,
    /// The index of the weapon that was fired.
    pub weapon_index: u32,
    /// The entity that fired the weapon. Used for things like firing sounds & VFX following the shooter.
    pub shooter_entity: Entity,
    /// The absolute origin of the fire. Used for knowing which location to spawn VFX & sounds.
    pub fire_origin: Vec3,
    /// The direction of the shooter at the time of firing. Used for knowing which direction to spawn VFX.
    pub fire_direction: Dir3,
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
    /// Cycle to the next weapon in the provided weapon list. Wraps around.
    pub fn next_weapon(&mut self, weapons: &HashMap<WeaponId, Weapon>) {
        let max_id = weapons.keys().max().unwrap_or(&0);
        let mut new_idx = self.0;
        for i in 0..=*max_id {
            new_idx = (self.0 + i + 1) % (max_id + 1);
            if weapons.contains_key(&new_idx) {
                self.0 = new_idx;
                break;
            }
        }
    }

    /// Cycle to the previous weapon in the provided weapon list. Wraps around.
    pub fn previous_weapon(&mut self, weapons: &HashMap<WeaponId, Weapon>) {
        let max_id = weapons.keys().max().unwrap_or(&0);
        let mut new_idx = self.0;
        for i in 0..=*max_id {
            new_idx = self.0.checked_sub(i + 1).unwrap_or(*max_id);
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
#[derive(Component, Debug, Clone)]
pub struct Projectile {
    pub weapon_id: WeaponId,
}

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
    shooter_transform: &Transform,
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
            FireMode::Burst { shots, delay_millis, delay_after_burst_millis } => {
                // @todo-brian: implement burst mode
            }
        }

        if should_fire {
            match weapon_data.barrel_mode {
                BarrelMode::Simultaneous => {
                    let direction = shooter_transform.forward();
                    for barrel_position in weapon_data.barrel_positions.iter() {
                        let rotated_barrel_pos = shooter_transform.rotation * *barrel_position;
                        let mut new_transform = *shooter_transform;
                        new_transform.translation += rotated_barrel_pos;

                        // TODO: spawn an initial-replicated bullet, with

                        // we shoot a non-networked linear bullet
                        // it's trajectory should be deterministic on the client and server
                        // TODO: actually we will need to network the initial replication
                        //  because we want to see enemy bullets fired in the interpolated timeline?
                        // TODO: maybe enemy bullets can be sped up to be in the predicted timeline so that
                        //  they can hit us, similar to what Piefayth does
                        //info!("speed: {}", weapon_data.projectile.speed);
                        commands.spawn((
                            new_transform,
                            WeaponFiredEvent {
                                shooter_id: identity.clone(),
                                weapon_index: current_weapon_idx,
                                shooter_entity: shooting_entity,
                                fire_origin: shooter_transform.translation,
                                fire_direction: direction,
                            },
                            Projectile {
                                weapon_id: current_weapon_idx,
                            },
                            // TODO(cb): for some it's necessary to include both Position and Transform
                            Position(new_transform.translation),
                            LinearVelocity(direction * weapon_data.projectile.speed),
                            DespawnAfter(Timer::new(Duration::from_millis(weapon_data.projectile.lifetime_millis), TimerMode::Once)),

                            // TODO: should we not include collision layers to accelerate collision computation performance?
                            // CollisionLayers::new([GameLayer::Projectile], [GameLayer::Player, GameLayer::Wall]),
                            RigidBody::Kinematic,
                        ));
                    }

                    // send an event so that we can spawn vfx/sfx
                    commands.send_event(WeaponFiredEvent {
                        shooter_id: identity.clone(),
                        weapon_index: current_weapon_idx,
                        shooter_entity: shooting_entity,
                        fire_origin: shooter_transform.translation,
                        fire_direction: direction,
                    });
                }
                BarrelMode::Sequential => {
                    // @todo-brian: implement sequential mode
                }
            }
        }
    }
}