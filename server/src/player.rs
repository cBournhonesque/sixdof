use bevy::prelude::*;
use lightyear::prelude::server::*;
use shared::{player::Player, prelude::{Damageable, GameLayer, Moveable, MoveableShape, UniqueIdentity}, ships::shared_ship_components, weapons::{CurrentWeaponIndex, WeaponInventory, WeaponsData}};
use avian3d::prelude::*;
use lightyear::prelude::{NetworkTarget, ReplicateHierarchy, ReplicationGroup};
use lightyear::shared::replication::components::ReplicationGroupId;
use shared::prelude::{PREDICTION_REPLICATION_GROUP_ID};

pub(crate) struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_player_on_connect.run_if(resource_exists::<WeaponsData>));
    }
}

fn spawn_player_on_connect(
    weapons_data: Res<WeaponsData>,
    mut commands: Commands, 
    mut events: EventReader<ConnectEvent>,
) {
    for event in events.read() {
        info!("Received ConnectEvent: {:?}", event);

        // TODO: use spawn-events so we can control spawn position, etc.
        commands.spawn(
            (
                Name::from("Player"),
                Replicate {
                    sync: SyncTarget {
                        prediction: NetworkTarget::Single(event.client_id),
                        interpolation: NetworkTarget::AllExceptSingle(event.client_id),
                    },
                    controlled_by: ControlledBy {
                        target: NetworkTarget::Single(event.client_id),
                        ..default()
                    },
                    // in case the renderer is enabled on the server, we don't want the visuals to be replicated!
                    hierarchy: ReplicateHierarchy {
                        enabled: false,
                        recursive: false,
                    },
                    group: ReplicationGroup::new_id(PREDICTION_REPLICATION_GROUP_ID),
                    ..default()
                },
                UniqueIdentity::Player(event.client_id),
                Player {
                    name: "Player".to_string(),
                    respawn_timer: Timer::from_seconds(3.0, TimerMode::Once),
                },
                Damageable {
                    health: 200,
                },
                // Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)),
                CurrentWeaponIndex(0),
                WeaponInventory::from_data(&weapons_data, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
                Position::from_xyz(0.0, 2.0, 0.0),
                shared_ship_components(Collider::sphere(0.5))
            )
        );
    }
}

