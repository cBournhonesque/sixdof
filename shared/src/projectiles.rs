use avian3d::prelude::{Collider, LinearVelocity, Position};
use bevy::prelude::*;
use leafwing_input_manager::action_state::ActionState;
use lightyear::client::prediction::Predicted;
use lightyear::prelude::*;
use lightyear::prelude::server::{Replicate, SyncTarget};
use crate::player::Player;
use crate::prelude::{PlayerInput, PREDICTION_REPLICATION_GROUP_ID};

pub(crate) struct ProjectilesPlugin;

impl Plugin for ProjectilesPlugin {
    fn build(&self, app: &mut App) {
        // SYSTEMS
        app.add_systems(FixedUpdate, shoot_projectiles);
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
            let projectile = (
                *transform,
                Projectile,
                // TODO: change projectile speed
                LinearVelocity(transform.translation.normalize() * 10.0),
                // TODO: change projectile shape
                Collider::sphere(0.5),
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