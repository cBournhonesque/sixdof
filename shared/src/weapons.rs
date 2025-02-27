use std::time::Duration;

use avian3d::prelude::{Collider, LinearVelocity, RigidBody};
use bevy::{prelude::*, scene::ron::ser::{to_string_pretty, PrettyConfig}, utils::HashMap};
use bevy_config_stack::prelude::ConfigAssetLoaderPlugin;
use leafwing_input_manager::prelude::ActionState;
use lightyear::prelude::{client::Predicted, *};
use serde::{Deserialize, Serialize};

use crate::{player::Player, prelude::{Identity, PlayerInput}};

pub type WeaponId = u32;

pub(crate) struct WeaponsPlugin;

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        
        // let default_weapons_list = WeaponsListConfig::default();
        // let default_weapons_list_ron = to_string_pretty(&default_weapons_list, PrettyConfig::default()).unwrap();
        // println!("{}", default_weapons_list_ron);

        app.add_plugins(ConfigAssetLoaderPlugin::<WeaponsData>::new("data/weapons.ron"));
        app.add_systems(FixedUpdate, shoot_system.run_if(resource_exists::<WeaponsData>));
    }
}

#[derive(Component, Serialize, Deserialize, PartialEq)]
pub struct WeaponInventory {
    pub weapons: HashMap<WeaponId, Weapon>,
    pub current_weapon_idx: WeaponId,
}

impl Default for WeaponInventory {
    fn default() -> Self {
        let mut weapons = HashMap::new();
        weapons.insert(0, Weapon::default());
        Self { 
            weapons,
            current_weapon_idx: 0,
        }
    }
}

impl WeaponInventory {
    /// Cycle to the next weapon in the inventory. Wraps around.
    pub fn next_weapon(&mut self) {
        self.current_weapon_idx = (self.current_weapon_idx + 1) % self.weapons.len() as u32;
    }

    /// Cycle to the previous weapon in the inventory. Wraps around.
    pub fn previous_weapon(&mut self) {
        self.current_weapon_idx = (self.current_weapon_idx - 1 + self.weapons.len() as u32) % self.weapons.len() as u32;
    }
}

/// A weapon component defines the state of a weapon.
/// 
/// @todo-brian: maybe this can eventually be a component too, 
/// so that if we die and drop the weapon, it can be picked up 
/// by someone else with the exact state it was left off with.
#[derive(Component, Default, Serialize, Deserialize, PartialEq)]
pub struct Weapon {
    pub fire_timer_auto: Timer,
    pub fire_timer_burst: Timer,
    pub fire_timer_post_burst: Timer,
    pub burst_shots_left: u32,
    pub ammo_left: u32,
}

// TODO: maybe make this an enum with the type of projectile?
#[derive(Component, Debug, Clone)]
pub struct Projectile;

/// The resource that contains all the weapon configurations.
#[derive(Resource, Asset, TypePath, Debug, Deserialize, Serialize)]
struct WeaponsData {
    weapons: HashMap<WeaponId, WeaponBehavior>,
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
struct WeaponBehavior {
    /// The human readable name of the weapon.
    name: String,
    /// The description of the weapon.
    description: String,
    /// The positions of the barrels of the weapon.
    barrel_positions: Vec<Vec3>,
    /// The mode of the weapon.
    barrel_mode: BarrelMode,
    /// The mode of the weapon.
    fire_mode: FireMode,
    /// The crosshair of the weapon.
    crosshair: CrosshairConfiguration,
    /// The projectile behavior of the weapon.
    projectile: ProjectileBehavior,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct ProjectileBehavior {
    speed: f32,
    /// The lifetime of the projectile in seconds before it is removed from the world. 
    /// Will attempt to apply splash damage upon removal.
    lifetime_secs: f32,
    direct_damage: f32,
    splash_damage_radius: f32,
    splash_damage_max: f32,
    splash_damage_min: f32,
}

#[derive(Debug, Deserialize, Serialize)]
enum BarrelMode {
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
enum FireMode {
    /// An automatic weapon just fires continuously with a delay between each shot.
    Auto {
        delay_ms: u64,
    },
    /// A burst fires a number of shots in a burst, with a delay between each shot.
    Burst {
        /// The number of shots in a burst.
        shots: u32,
        /// The delay between each shot in a burst.
        delay_ms: u64,
        /// The delay after the burst is finished before starting another burst.
        delay_after_burst_ms: u64,
    },
}

impl Default for FireMode {
    fn default() -> Self {
        Self::Auto { delay_ms: 100 }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct CrosshairConfiguration {
    color: Color,

    /// The image to use for the crosshair. 
    /// Relative to assets/crosshairs/
    image: String,
}

impl Default for CrosshairConfiguration {
    fn default() -> Self {
        Self { color: Color::WHITE, image: "kenney_crosshair_pack/crosshair018.png".to_string() }
    }
}

fn shoot_system(
    fixed_time: Res<Time<Fixed>>,
    mut commands: Commands,
    weapons_data: Res<WeaponsData>,
    mut query: Query<(
        &Player,
        &Transform,
        &mut WeaponInventory,
        &ActionState<PlayerInput>,
    ),
    Or<(With<Predicted>, With<Replicating>)>>,
) {
    for (_player, transform, mut inventory, action) in query.iter_mut() {
        let current_weapon_idx = inventory.current_weapon_idx;
        if let (Some(weapon_data), Some(weapon_state)) = (
            weapons_data.weapons.get(&current_weapon_idx), 
            inventory.weapons.get_mut(&current_weapon_idx)
        ) {
            let mut should_fire = false;
            match weapon_data.fire_mode {
                FireMode::Auto { delay_ms: delay } => {
                    weapon_state.fire_timer_auto.set_duration(Duration::from_millis(delay));
                    weapon_state.fire_timer_auto.tick(fixed_time.delta());
                    
                    if weapon_state.fire_timer_auto.finished() 
                        && action.pressed(&PlayerInput::ShootPrimary) {
                        should_fire = true;
                        weapon_state.fire_timer_auto.reset();
                    }
                }
                FireMode::Burst { shots, delay_ms: delay, delay_after_burst_ms: delay_after_burst } => {
                    // @todo-brian: implement burst mode
                }
            }

            if should_fire {
                match weapon_data.barrel_mode {
                    BarrelMode::Simultaneous => {
                        let direction = transform.forward().as_vec3();
        
                        let mut new_transform = *transform;
                        new_transform.translation += 0.5 * direction;
                        commands.spawn((
                            new_transform,
                            Projectile,
                            InheritedVisibility::default(),
                            // TODO: change projectile speed
                            LinearVelocity(direction * 5.0),
                            // TODO: change projectile shape
                            Collider::sphere(0.1),
                            RigidBody::Dynamic,
                        ));
                        
                        // for barrel_position in weapon_data.barrel_positions.iter() {
                        //     let rotated_barrel_pos = transform.rotation * *barrel_position;
                        //     let mut new_transform = *transform;
                        //     new_transform.translation += rotated_barrel_pos + (direction * 1.0);

                        //     commands.spawn((
                        //         new_transform,
                        //         Projectile,
                        //         InheritedVisibility::default(),
                        //         LinearVelocity(direction * weapon_data.projectile.speed),
                        //         Collider::sphere(0.1),
                        //         RigidBody::Dynamic,
                        //     ));
                        // }
                    }
                    BarrelMode::Sequential => {
                        // @todo-brian: implement sequential mode
                    }
                }
            }
        }
    }
}

