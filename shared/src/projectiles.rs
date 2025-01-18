use avian3d::prelude::{Collider, LinearVelocity, Position, RigidBody};
use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use leafwing_input_manager::action_state::ActionState;
use lightyear::client::prediction::Predicted;
use lightyear::prelude::*;
use lightyear::prelude::server::{Replicate, SyncTarget};
use crate::player::Player;
use crate::prelude::{PlayerInput, PREDICTION_REPLICATION_GROUP_ID};

pub(crate) struct ProjectilesPlugin;

impl Plugin for ProjectilesPlugin {
    fn build(&self, app: &mut App) {
        // PLUGINS
        app.add_plugins(RonAssetPlugin::<WeaponConfiguration>::new(&["weapon.ron"]));
        // SYSTEMS
        app.add_systems(Startup, setup_configuration);
        app.add_systems(Update, configuration_change_watcher);
        app.add_systems(FixedUpdate, shoot_projectiles);
    }
}

/// Loads the weapon configurations and stores them in a resource.
fn setup_configuration(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // @todo-brian: We might want it so that the server loads these 
    // and then the client just receives the configurations from the server.
    // That way, the server can configure on the fly and it all syncs up.
    // This would be ideal during heavy development phase, and it lends itself to modding.
    commands.insert_resource(WeaponConfigurations {
        dual_lasers: asset_server.load("data/weapons/dual_lasers.weapon.ron"),
        rocket_launcher: asset_server.load("data/weapons/rocket_launcher.weapon.ron"),
    });
}

/// Watch for changes to weapon configurations and log them.
fn configuration_change_watcher(
    mut events: EventReader<AssetEvent<WeaponConfiguration>>,
) {
    for event in events.read() {
        match event {
            AssetEvent::LoadedWithDependencies { id: _ } => {
                //info!("Weapon config loaded: {:?}", id);
            }
            AssetEvent::Modified { id: _ } => {
                //info!("Weapon config modified: {:?}", id);
            }
            AssetEvent::Removed { id: _ } => {
                //info!("Weapon config removed: {:?}", id);
            }
            _ => {}
        }
    }
}

// TODO: maybe make this an enum with the type of projectile?
#[derive(Component, Debug, Clone)]
pub struct Projectile;


/// Shoot projectiles from the current weapon when the shoot action is pressed
pub(crate) fn shoot_projectiles(
    mut commands: Commands,
    identity: NetworkIdentity,
    query: Query<
        (
            &Player,
            &Transform,
            &ActionState<PlayerInput>,
        ),
        Or<(With<Predicted>, With<Replicating>)>,
    >,
) {
    for (player, transform, action) in query.iter() {

        // NOTE: pressed lets you shoot many bullets, which can be cool
        if action.just_pressed(&PlayerInput::ShootPrimary) {
            let direction = transform.forward().as_vec3();

            // offset a little bit from the player
            let mut new_transform = *transform;
            new_transform.translation += 0.5 * direction;
            let projectile = (
                new_transform,
                Projectile,
                // TODO: change projectile speed
                LinearVelocity(direction * 5.0),
                // TODO: change projectile shape
                Collider::sphere(0.1),
                // the projectile will be spawned on both client (in the predicted timeline) and the server
                PreSpawnedPlayerObject::default(),
                RigidBody::Dynamic,
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
pub struct WeaponConfigurations {
    dual_lasers: Handle<WeaponConfiguration>,
    rocket_launcher: Handle<WeaponConfiguration>,
}

/// A weapon configuration is basically what it sounds like, 
/// it defines all the behaviors of a weapon.
#[derive(Asset, TypePath, serde::Deserialize, serde::Serialize)]
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
