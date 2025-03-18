use std::time::Duration;

use bevy::prelude::*;
use lightyear::prelude::{server::*, *};
use shared::{player::{PlayerRespawnTimer, PlayerShip}, prelude::{Damageable, UniqueIdentity, PREDICTION_REPLICATION_GROUP_ID}, ships::{get_shared_ship_components, Ship, ShipId, ShipsData}, weapons::{CurrentWeaponIndex, WeaponInventory, WeaponsData}};
use avian3d::prelude::*;
use lightyear::prelude::{NetworkTarget, ReplicateHierarchy, ReplicationGroup};

pub(crate) struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnPlayerShipEvent>();
        app.add_systems(Update, 
            (
                player_connect_system,
                spawn_player_ship_system
                    .run_if(resource_exists::<WeaponsData>)
                    .run_if(resource_exists::<ShipsData>)
            )
        );
    }
}

#[derive(Event)]
pub struct SpawnPlayerShipEvent {
    pub client_id: ClientId,
    pub ship_id: ShipId,
    pub position: Vec3,
    pub rotation: Quat,
}

fn player_connect_system(
    mut commands: Commands,
    mut connect_events: EventReader<ConnectEvent>,
    mut disconnect_events: EventReader<DisconnectEvent>,
    mut spawn_player_ship_events: EventWriter<SpawnPlayerShipEvent>,
) {
    for event in connect_events.read() {
        info!("Received ConnectEvent: {:?}", event);
        commands.entity(event.entity).insert((
            Name::from(format!("Player ({})", event.client_id)),
            UniqueIdentity::Player(event.client_id),
            PlayerRespawnTimer(Timer::new(Duration::from_secs(3), TimerMode::Once)),
        ));
        
        spawn_player_ship_events.send(SpawnPlayerShipEvent {
            client_id: event.client_id,
            ship_id: 0,
            position: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_arc(Vec3::Y, Vec3::Z),
        });
    }

    for event in disconnect_events.read() {
        info!("Received DisconnectEvent: {:?}", event);
    }
}

fn spawn_player_ship_system(
    mut commands: Commands,
    mut events: EventReader<SpawnPlayerShipEvent>,
    weapons_data: Res<WeaponsData>,
    ships_data: Res<ShipsData>,
    existing_player_ships: Query<(Entity, &UniqueIdentity), With<PlayerShip>>,
) {
    for event in events.read() {
        if let Some(ship_data) = ships_data.ships.get(&event.ship_id) {
            let existing_player_ship = existing_player_ships.iter().find(|(_, identity)| match identity {
                UniqueIdentity::Player(client_id) => *client_id == event.client_id,
                _ => false,
            });

            if let Some((entity, _)) = existing_player_ship {
                warn!("Player ship already exists for client id {}, despawning old ship: {}", event.client_id, entity);
                commands.entity(entity).despawn_recursive();
            }

            commands.spawn(
                (
                    Name::from(format!("Player Ship ({})", event.client_id)),
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
                    PlayerShip,
                    Ship(0),
                    Damageable {
                        health: ship_data.starting_health,
                    },
                    CurrentWeaponIndex(0),
                    WeaponInventory::from_data(&weapons_data, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
                    Position::from(event.position),
                    Rotation::from(event.rotation),
                    get_shared_ship_components(Collider::sphere(0.5))
                )
            );
        } else {
            error!("Ship data not found for ship id: {}", event.ship_id);
        }
    }
}

