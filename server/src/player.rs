use bevy::prelude::*;
use lightyear::prelude::server::*;
use shared::player::Player;
use avian3d::prelude::*;
use lightyear::prelude::NetworkTarget;

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
                Transform::default(),
                // Health::new(even/**/t.health, 200),
                // HealthRegen {
                //     delay_before_heal: Timer::from_seconds(5.0, TimerMode::Once),
                //     heal_tick_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
                //     amount: 1.0,
                // },
                // MovementState {
                //     max_speed: 20.0,
                //     acceleration: 1.5,
                //     rotation_speed: 12.0,
                //     drag: 1.5,
                //     ..default()
                // },
                // event.team.clone(),
                // TriggerInstigator::default(),
                // WeaponContainer::new(WeaponContainerConfig {
                //     owner_id: event.id,
                //     weapons: vec![
                //         Weapon::new(1, None),
                //         Weapon::new(2, None),
                //         Weapon::new(3, None),
                //         Weapon::new(4, None),
                //         Weapon::new(5, None),
                //     ],
                // }),
                RigidBody::Kinematic,
                Collider::sphere(0.5),
            )
        );
    }
}

