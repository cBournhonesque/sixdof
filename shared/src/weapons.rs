use avian3d::prelude::Position;
use bevy::prelude::*;
use leafwing_input_manager::action_state::ActionState;
use lightyear::client::prediction::Predicted;
use lightyear::prelude::*;
use lightyear::prelude::server::{Replicate, SyncTarget};
use crate::player::Player;
use crate::prelude::{PlayerInput, PREDICTION_REPLICATION_GROUP_ID};

pub(crate) struct WeaponsPlugin;

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        // SYSTEMS
        app.add_systems(FixedUpdate, shoot_projectiles);
    }
}

#[derive(Component, Debug, Clone)]
pub struct Projectile;

/// Shoot projectiles from the current weapon when the shoot action is pressed
pub(crate) fn shoot_projectiles(
    mut commands: Commands,
    identity: NetworkIdentity,
    query: Query<
        (
            &Player,
            &Position,
            &ActionState<PlayerInput>,
        ),
        Or<(With<Predicted>, With<Replicating>)>,
    >,
) {
    for (player, position, action) in query.iter() {

        // NOTE: pressed lets you shoot many bullets, which can be cool
        if action.just_pressed(&PlayerInput::ShootPrimary) {
            let projectile = (
                Transform::from_translation(position.0),
                Projectile,
                // the projectile will be spawned on both client (in the predicted timeline) and the server
                PreSpawnedPlayerObject::default(),
            );

            // on the server, spawn and replicate the projectile
            if identity.is_server() {
                commands.spawn((
                    projectile,
                    Replicate {
                        sync: SyncTarget {
                            // the bullet is predicted for the client who shot it
                            prediction: NetworkTarget::Single(player.id),
                            // the bullet is interpolated for other clients
                            interpolation: NetworkTarget::AllExceptSingle(player.id),
                        },
                        // NOTE: all predicted entities need to have the same replication group
                        group: ReplicationGroup::new_id(PREDICTION_REPLICATION_GROUP_ID),
                        ..default()
                    },
                ));
            } else {
                commands.spawn(projectile);
            }
        }
    }
}

/// The resource that contains all the weapon configurations.
#[derive(Resource)]
pub enum WeaponConfigurations {
    DualLasers(WeaponConfiguration),
    RocketLauncher(WeaponConfiguration),
}

/// A weapon configuration is basically what it sounds like, 
/// it defines all the behaviors of a weapon.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct WeaponConfiguration {
    pub name: String,
    pub description: String,
    pub barrel_positions: Vec<Vec3>,
    pub barrel_mode: BarrelMode,
    pub fire_mode: FireMode,
    pub crosshair: CrosshairConfiguration,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ProjectileConfiguration {
    speed: f32,
    /// The lifetime of the projectile in seconds before it is removed from the world. 
    /// Will attempt to apply splash damage upon removal.
    lifetime: f32,
    direct_damage: f32,
    splash_damage_radius: f32,
    splash_damage_max: f32,
    splash_damage_min: f32,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub enum BarrelMode {
    /// All barrels fire at the same time.
    Simultaneous,
    /// Barrels fire one after the other.
    Sequential,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub enum FireMode {
    /// An automatic weapon just fires continuously with a delay between each shot.
    Auto {
        delay: f32,
    },
    /// A burst fires a number of shots in a burst, with a delay between each shot.
    Burst {
        /// The number of shots in a burst.
        shots: u32,
        /// The delay between each shot in a burst.
        delay: f32,
        /// The delay after the burst is finished before starting another burst.
        delay_after_burst: f32,
    },
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct CrosshairConfiguration {
    pub color: Color,
    
    /// The image to use for the crosshair. 
    /// Relative to assets/crosshairs/
    pub image: String,
}
