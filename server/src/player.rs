use bevy::prelude::*;
use lightyear::prelude::server::*;
use shared::{player::Player, prelude::{GameLayer, Moveable, ShapecastMoveableShape}, weapons::WeaponInventory};
use avian3d::prelude::*;
use lightyear::prelude::{NetworkTarget, ReplicateHierarchy};

pub(crate) struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_player_on_connect);
    }
}

fn spawn_player_on_connect(mut commands: Commands, mut events: EventReader<ConnectEvent>) {
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
                    // TODO: all predicted entities must be part of the same replication group
                    ..default()
                },
                Player {
                    id: event.client_id,
                    score: 0,
                    frags: 0,
                    deaths: 0,
                    ping: 0,
                    frozen_amount: 0,
                    name: "Player".to_string(),
                    respawn_timer: Timer::from_seconds(3.0, TimerMode::Once),
                },
                Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)),
                WeaponInventory::default(),
                Moveable {
                    velocity: Vec3::ZERO,
                    angular_velocity: Vec3::ZERO,
                    collision_shape: ShapecastMoveableShape::Sphere(0.5),
                },
            )
        );
    }
}

