use core::time::Duration;

use bevy::prelude::*;
use lightyear::prelude::{server::*, *};
use shared::{player::{PlayerRespawnTimer, PlayerShip}, prelude::{Damageable, UniqueIdentity}, ships::{get_shared_ship_components, Ship, ShipId, ShipsData}, weapons::{CurrentWeaponIndex, WeaponInventory, WeaponsData}};
use avian3d::prelude::*;

pub(crate) struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnPlayerShipEvent>();
        app.add_observer(player_connect_system);
        app.add_systems(Update, 
                spawn_player_ship_system
                    .run_if(resource_exists::<WeaponsData>)
                    .run_if(resource_exists::<ShipsData>)
        );
    }
}

#[derive(Event)]
pub struct SpawnPlayerShipEvent {
    pub client_entity: Entity,
    pub client_id: PeerId,
    pub ship_id: ShipId,
    pub position: Vec3,
    pub rotation: Quat,
}

fn player_connect_system(
    trigger: Trigger<OnAdd, Connected>,
    peer_id: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands,
    mut spawn_player_ship_events: EventWriter<SpawnPlayerShipEvent>,
) {
    if let Ok(peer_id) = peer_id.get(trigger.target()) {
        let client_id = peer_id.0;
        info!("Connection from new client: {client_id:?}");
         commands.entity(trigger.target()).insert((
            Name::from(format!("Player ({})", client_id)),
            UniqueIdentity::Player(client_id),
            PlayerRespawnTimer(Timer::new(Duration::from_secs(3), TimerMode::Once)),
        ));

        spawn_player_ship_events.write(SpawnPlayerShipEvent {
            client_entity: trigger.target(),
            client_id,
            ship_id: 0,
            position: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_arc(Vec3::Y, Vec3::Z),
        });
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
                commands.entity(entity).despawn();
            }

            commands.spawn(
                (
                    Name::from(format!("Player Ship ({})", event.client_id)),
                    Replicate::to_clients(NetworkTarget::All),
                    PredictionTarget::to_clients(NetworkTarget::Single(event.client_id)),
                    InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(event.client_id)),
                    ControlledBy {
                        owner: event.client_entity,
                        lifetime: Default::default()
                    },
                    // in case the renderer is enabled on the server, we don't want the visuals to be replicated!
                    DisableReplicateHierarchy,
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

