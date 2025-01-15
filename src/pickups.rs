use crate::components::Health;
use crate::player::*;
use crate::spawn::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

const PICKUP_DETECTION_DISTANCE: f32 = 3.0;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PickupKind {
    Health,
    RedKey,
    BlueKey,
    YellowKey,
    OrangeKey,
}

impl From<u8> for PickupKind {
    fn from(value: u8) -> Self {
        match value {
            0 => PickupKind::Health,
            1 => PickupKind::RedKey,
            2 => PickupKind::BlueKey,
            3 => PickupKind::YellowKey,
            4 => PickupKind::OrangeKey,
            _ => PickupKind::Health,
        }
    }
}

#[derive(Component)]
pub struct Pickup {
    pub id: u8,
    pub kind: PickupKind,
}

#[derive(Component)]
pub struct HealthPickup {
    pub amount: i16,
}

#[derive(Component)]
pub struct RedKey;

#[derive(Component)]
pub struct BlueKey;

#[derive(Component)]
pub struct YellowKey;

#[derive(Component)]
pub struct OrangeKey;

pub fn pickup_system(
    local_player: Res<LocalPlayer>,
    pickups: Query<(Entity, &Pickup, &Transform)>,
    health_pickups: Query<&HealthPickup>,
    mut player_query: Query<(&Transform, &mut Health), With<Player>>,
    mut spawn_events: EventWriter<SpawnEvent>,
) {
    if !local_player.has_authority() {
        return;
    }

    for (pickup_entity, pickup, pickup_transform) in &mut pickups.iter() {
        for (player_transform, mut health) in &mut player_query.iter_mut() {
            if pickup_transform
                .translation
                .distance(player_transform.translation)
                < PICKUP_DETECTION_DISTANCE
            {
                match pickup.kind {
                    PickupKind::Health => {
                        let health_pickup = health_pickups.get(pickup_entity);
                        match health_pickup {
                            Ok(health_pickup) => {
                                health.increment(health_pickup.amount);
                                spawn_events.send(SpawnEvent::DespawnPickup(pickup.id));
                            }
                            Err(_) => {}
                        }
                    }
                    PickupKind::RedKey => {
                        spawn_events.send(SpawnEvent::DespawnPickup(pickup.id));
                    }
                    PickupKind::BlueKey => {
                        spawn_events.send(SpawnEvent::DespawnPickup(pickup.id));
                    }
                    PickupKind::YellowKey => {
                        spawn_events.send(SpawnEvent::DespawnPickup(pickup.id));
                    }
                    PickupKind::OrangeKey => {
                        spawn_events.send(SpawnEvent::DespawnPickup(pickup.id));
                    }
                }
            }
        }
    }
}
