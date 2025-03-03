use std::time::Duration;

use avian3d::{math::Vector, prelude::{Collider, CollisionLayers, LinearVelocity, Position, RigidBody, SpatialQuery}};
use bevy::{prelude::*, scene::ron::ser::{to_string_pretty, PrettyConfig}, utils::HashMap};
use bevy_config_stack::prelude::{ConfigAssetLoadedEvent, ConfigAssetLoaderPlugin};
use leafwing_input_manager::prelude::ActionState;
use lightyear::{client::prediction::rollback::DisableRollback, prelude::{client::{Predicted, Rollback}, *}, shared::replication::components::Controlled};
use serde::{Deserialize, Serialize};

use crate::{physics::GameLayer, player::Player, prelude::{UniqueIdentity, Moveable, MoveableHit, MoveableHitData, MoveableExtras, PlayerInput, MoveableShape}, utils::DespawnAfter};

pub type WeaponId = u32;

pub(crate) struct WeaponsPlugin;

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        
        // let default_weapons_list = WeaponsListConfig::default();
        // let default_weapons_list_ron = to_string_pretty(&default_weapons_list, PrettyConfig::default()).unwrap();
        // println!("{}", default_weapons_list_ron);

        app.add_event::<ProjectileHitEvent>();
        app.add_plugins(ConfigAssetLoaderPlugin::<WeaponsData>::new("data/weapons.ron"));
        app.add_systems(FixedUpdate, update_weapon_timers_system.run_if(resource_exists::<WeaponsData>));
    }
}

// // TODO: maybe have a separate event for ray-cast vs slow bullets?
// /// Bullet that shoots in a straight line
#[derive(Event, Clone, Debug)]
pub struct LinearProjectile {
    pub shooter: UniqueIdentity,
    pub shooter_entity: Entity,
    pub source: Vector,
    pub direction: Dir3,
    pub speed: f32,
    pub interpolation_delay_millis: u16,
}

impl Default for LinearProjectile {
    fn default() -> Self {
        Self {
            shooter: UniqueIdentity::Player(ClientId::Local(0)),
            shooter_entity: Entity::PLACEHOLDER,
            source: Vector::ZERO,
            direction: Dir3::Z,
            // the default is to shoot raycast bullets
            speed: 1000.0,
            interpolation_delay_millis: 0,
        }
    }
}

/// Event that is sent when a projectile hits an entity.
/// Can be used to spawn vfx and play sfx, apply damage, etc.
#[derive(Event)]
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

/// A weapon behavior is basically what it sounds like, 
/// it defines all the behaviors of a weapon.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct WeaponBehavior {
    /// The human readable name of the weapon.
    pub name: String,
    /// The description of the weapon.
    pub description: String,
    /// The positions of the barrels of the weapon.
    pub barrel_positions: Vec<Vec3>,
    /// The mode of the weapon.
    pub barrel_mode: BarrelMode,
    /// The mode of the weapon.
    pub fire_mode: FireMode,
    /// The crosshair of the weapon.
    pub crosshair: CrosshairConfiguration,
    /// The projectile behavior of the weapon.
    pub projectile: ProjectileBehavior,
    /// The starting ammo of the weapon.
    pub starting_ammo: u32,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ProjectileBehavior {
    pub speed: f32,
    /// The lifetime of the projectile in milliseconds before it is removed from the world. 
    /// Will attempt to apply splash damage upon removal.
    pub lifetime_millis: u64,
    pub direct_damage: u16,
    pub splash_damage_radius: f32,
    pub splash_damage_max: u16,
    pub splash_damage_min: u16,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum BarrelMode {
    /// All barrels fire at the same time.
    Simultaneous,
    /// Barrels fire one after the other.
    Sequential,
}

impl Default for BarrelMode {
    fn default() -> Self {
        Self::Simultaneous
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum FireMode {
    /// An automatic weapon just fires continuously with a delay between each shot.
    Auto {
        delay_millis: u64,
    },
    /// A burst fires a number of shots in a burst, with a delay between each shot.
    Burst {
        /// The number of shots in a burst.
        shots: u32,
        /// The delay between each shot in a burst.
        delay_millis: u64,
        /// The delay after the burst is finished before starting another burst.
        delay_after_burst_millis: u64,
    },
}

impl Default for FireMode {
    fn default() -> Self {
        Self::Auto { delay_millis: 100 }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CrosshairConfiguration {
    pub color: Color,

    /// The image to use for the crosshair. 
    /// Relative to assets/crosshairs/
    pub image: String,
}

impl Default for CrosshairConfiguration {
    fn default() -> Self {
        Self { color: Color::WHITE, image: "kenney_crosshair_pack/crosshair018.png".to_string() }
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
    transform: &Transform,
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
                    let direction = transform.forward();
                    for barrel_position in weapon_data.barrel_positions.iter() {
                        let rotated_barrel_pos = transform.rotation * *barrel_position;
                        let mut new_transform = *transform;
                        new_transform.translation += rotated_barrel_pos;

                        // we include information about the shooter to be able to
                        // use the correct lag compensation values
                        let linear_bullet_event = LinearProjectile {
                            shooter: identity.clone(),
                            shooter_entity: shooting_entity,
                            source: new_transform.translation,
                            direction,
                            speed: weapon_data.projectile.speed,
                            interpolation_delay_millis: 0,
                        };

                        // we shoot a non-networked linear bullet
                        // it's trajectory should be deterministic on the client and server
                        // TODO: actually we will need to network the initial replication
                        //  because we want to see enemy bullets fired in the interpolated timeline?
                        // TODO: maybe enemy bullets can be sped up to be in the predicted timeline so that
                        //  they can hit us, similar to what Piefayth does
                        //info!("speed: {}", weapon_data.projectile.speed);
                        commands.spawn((
                            new_transform,
                            linear_bullet_event,
                            Projectile,
                            Moveable {
                                velocity: direction.normalize_or_zero() * weapon_data.projectile.speed,
                                angular_velocity: Vec3::ZERO,
                                collision_shape: MoveableShape::Point,
                                collision_mask: [GameLayer::Player, GameLayer::Wall].into(),
                            },
                            MoveableExtras {
                                ignore_entities: Some(vec![shooting_entity]),
                                moveable_owner_id: identity.clone(),
                                moveable_type_id: current_weapon_idx,
                                on_hit: Some(Box::new(on_projectile_hit)),
                            },
                            DespawnAfter(Timer::new(Duration::from_millis(weapon_data.projectile.lifetime_millis), TimerMode::Once))
                        ));
                    }
                }
                BarrelMode::Sequential => {
                    // @todo-brian: implement sequential mode
                }
            }
        }
    }
}

fn on_projectile_hit(
    hit: MoveableHit,
    commands: &mut Commands,
    _spatial_query: &mut SpatialQuery,
) -> bool {
    let entity_hit = match hit.hit_data {
        MoveableHitData::ShapeCast(hit) => {
            Some(hit.entity)
        }
        MoveableHitData::RayCast(hit) => {
            Some(hit.entity)
        }
    };

    // send an event so that we can spawn vfx/sfx
    // and so that the server can subscribe to it for applying damage
    commands.send_event(ProjectileHitEvent {
        shooter_id: hit.moveable_owner_id,
        weapon_index: hit.moveable_type_id,
        projectile_entity: hit.moveable_entity,
        entity_hit
    });
    commands.entity(hit.moveable_entity).despawn_recursive();
    true
}
