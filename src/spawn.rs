use crate::components::*;
use crate::fx::*;
use crate::monsters::*;
use crate::net::client::*;
use crate::net::*;
use crate::physics;
use crate::physics::*;
use crate::pickups::*;
use crate::player::*;
use crate::sfx::*;
use crate::weapons::*;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::pbr::ExtendedMaterial;
use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::camera::Exposure;
use bevy::utils::HashMap;
use bevy_fmod::prelude::*;
use bevy_rapier3d::prelude::*;
use qevy::components::TriggerInstigator;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum OwnerType {
    Player,
    Bot,
}

impl Default for OwnerType {
    fn default() -> Self {
        Self::Player
    }
}

#[derive(Event)]
pub struct SpawnVisualsEvent {
    pub kind: VisualsKind,
    pub entity: Entity,
}

#[derive(Event)]
pub enum VisualsKind {
    Player(bool),
    Monster(MonsterKind),
    Pickup(PickupKind),
}

#[derive(Component)]
pub struct PlayerSpawnPoint;

#[derive(Event)]
pub enum SpawnEvent {
    Player(SpawnPlayer),
    DespawnPlayer(u8),

    Monster(SpawnMonster),
    DespawnMonster(u8),

    Pickup(SpawnPickup),
    DespawnPickup(u8),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpawnPickup {
    pub id: u8,
    pub amount: i16,
    pub kind: PickupKind,
    pub translation: Vec3,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SpawnMonster {
    pub id: u8,
    pub seed: u8,
    pub kind: MonsterKind,
    pub translation: Vec3,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SpawnPlayer {
    pub id: u8,
    pub position: Vec3,
    pub team: Team,
    pub name: String,
    pub health: i16,
}

pub fn spawn_event_system(
    local_player: Res<LocalPlayer>,
    mut commands: Commands,
    mut player_spawns: EventReader<SpawnEvent>,
    mut explosions: EventWriter<Explode>,
    players: Query<(Entity, &Player)>,
    bots: Query<(Entity, &Transform, &Monster)>,
    pickups: Query<(Entity, &Pickup)>,
    mut spawn_visuals: EventWriter<SpawnVisualsEvent>,
    weapon_configs: Res<Assets<WeaponConfig>>,
) {
    for event in player_spawns.read() {
        match event {
            SpawnEvent::Player(event) => {
                // dont spawn the player if it's already spawned
                for (_, player) in players.iter() {
                    if player.id == event.id {
                        return;
                    }
                }

                let locally_owned = event.id == local_player.player_id;
                spawn_player(
                    locally_owned,
                    &event,
                    &mut commands,
                    &mut spawn_visuals,
                    &weapon_configs,
                );
            }
            SpawnEvent::DespawnPlayer(id) => {
                for (entity, player) in players.iter() {
                    if player.id == *id {
                        if let Some(e) = commands.get_entity(entity) {
                            e.despawn_recursive();
                        }
                    }
                }
            }
            SpawnEvent::Monster(event) => {
                spawn_monster(
                    event.id,
                    event.seed,
                    &event.kind,
                    event.translation,
                    &mut commands,
                    &mut spawn_visuals,
                );
            }
            SpawnEvent::DespawnMonster(id) => {
                for (entity, transform, bot) in bots.iter() {
                    if bot.id == *id {
                        if let Some(e) = commands.get_entity(entity) {
                            e.despawn_recursive();

                            // spawn an explosion
                            explosions.send(Explode {
                                translation: transform.translation,
                                rotation: transform.rotation,
                                config: ExplosionParticlesConfig {
                                    lifetime_seconds: 2.0,
                                    light: Some(ExplosionLightConfig {
                                        color: Color::rgb(6.0, 0.0, 8.0),
                                        intensity: 8.0,
                                        lifetime_percent: 0.15,
                                    }),
                                    fire_smokes: Some(ExplosionFireSmokeConfig {
                                        amount: 2,
                                        size: 4.5,
                                        speed_max: 0.5,
                                        speed_min: 0.1,
                                        color: Color::rgb(1.0, 1.0, 1.0),
                                        lifetime_percent: 1.0,
                                    }),
                                    fragments: Some(ExplosionFragmentsConfig {
                                        amount: 30,
                                        size: 1.0,
                                        speed_max: 20.0,
                                        speed_min: 10.0,
                                        lifetime_percent: 0.5,
                                        color: Color::rgb(8.0, 0.0, 12.0),
                                    }),
                                    directional: false,
                                    electro_static_ripples: Some(
                                        ExplosionElectroStaticRippleConfig {
                                            amount: 10,
                                            size: 1.0,
                                            speed_max: 20.0,
                                            speed_min: 10.0,
                                            lifetime_percent: 0.5,
                                            color: Color::rgb(8.0, 0.0, 12.0),
                                        },
                                    ),
                                },
                            });
                        }
                    }
                }
            }
            SpawnEvent::Pickup(event) => {
                spawn_pickup(&event, &mut commands, &mut spawn_visuals);
            }
            SpawnEvent::DespawnPickup(id) => {
                for (entity, pickup) in pickups.iter() {
                    if pickup.id == *id {
                        if let Some(e) = commands.get_entity(entity) {
                            e.despawn_recursive();
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn spawn_health_system(
    mut commands: Commands,
    mut spawns: EventReader<SpawnEvent>,
    mut visuals_events: EventWriter<SpawnVisualsEvent>,
) {
    for event in spawns.read() {
        match event {
            SpawnEvent::Pickup(event) => {
                if let PickupKind::Health = event.kind {
                    let health = spawn_gameplay_entity(
                        &mut commands,
                        (
                            Pickup {
                                id: event.id,
                                kind: event.kind.clone(),
                            },
                            HealthPickup {
                                amount: event.amount,
                            },
                            TransformBundle {
                                local: Transform::from_translation(event.translation.clone()),
                                ..default()
                            },
                        ),
                    );

                    visuals_events.send(SpawnVisualsEvent {
                        kind: VisualsKind::Pickup(event.kind.clone()),
                        entity: health,
                    });
                }
            }
            _ => {}
        }
    }
}

pub fn spawn_player(
    locally_owned: bool,
    event: &SpawnPlayer,
    commands: &mut Commands,
    spawn_visuals: &mut EventWriter<SpawnVisualsEvent>,
    weapon_configs: &Res<Assets<WeaponConfig>>,
) {
    let entity = spawn_gameplay_entity(
        commands,
        (
            Player {
                id: event.id,
                visuals: None,
                latest_processed_input: None,
                score: 0,
                frags: 0,
                deaths: 0,
                ping: 0,
                frozen_amount: 0,
                name: "Player".to_string(),
                respawn_timer: Timer::from_seconds(3.0, TimerMode::Once),
            },
            SpatialBundle {
                transform: Transform::from_translation(event.position.clone()),
                ..default()
            },
            WishMove::default(),
            Health::new(event.health, 200),
            HealthRegen {
                delay_before_heal: Timer::from_seconds(5.0, TimerMode::Once),
                heal_tick_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
                amount: 1.0,
            },
            MovementState {
                max_speed: 20.0,
                acceleration: 1.5,
                rotation_speed: 12.0,
                drag: 1.5,
                ..default()
            },
            event.team.clone(),
            TriggerInstigator::default(),
            WeaponContainer::new(WeaponContainerConfig {
                owner_id: event.id,
                weapons: vec![
                    Weapon::new(1, None),
                    Weapon::new(2, None),
                    Weapon::new(3, None),
                    Weapon::new(4, None),
                    Weapon::new(5, None),
                ],
            }),
            RigidBody::KinematicPositionBased,
            Collider::ball(0.5),
        ),
    );

    spawn_visuals.send(SpawnVisualsEvent {
        kind: VisualsKind::Player(locally_owned),
        entity: entity,
    });

    if event.health > 0 {
        crate::utils::turn_on(entity, commands);
    } else {
        crate::utils::turn_off(entity, commands);
    }

    if locally_owned {
        println!("Spawning local player with id {}", event.id);
        commands.entity(entity).insert(LocallyOwned);
    } else {
        println!("Spawning remote player with id {}", event.id);
    }
}

pub fn spawn_monster(
    id: u8,
    seed: u8,
    kind: &MonsterKind,
    spawn_position: Vec3,
    commands: &mut Commands,
    spawn_visuals: &mut EventWriter<SpawnVisualsEvent>,
) {
    let monster = spawn_gameplay_entity(
        commands,
        (
            Monster {
                id,
                kind: kind.clone(),
                ..default()
            },
            Team::Virus,
            ClientInterpolate::default(),
            WishMove::default(),
            MovementState::default(),
            Health::default(),
            HealthPickupDropper { amount: 10 },
            Seed(seed),
            RigidBody::KinematicPositionBased,
            Collider::ball(0.5),
            CollisionGroups::new(
                physics::COLLISION_GROUP_DYNAMIC,
                physics::COLLISION_GROUP_MAX,
            ),
            TransformBundle {
                local: Transform::from_translation(spawn_position),
                ..default()
            },
        ),
    );

    match kind {
        MonsterKind::GruntPlasma => {
            commands
                .entity(monster)
                .insert(WeaponContainer::new(WeaponContainerConfig {
                    owner_id: id,
                    weapons: vec![Weapon::new(30, None)],
                }));
        }
        MonsterKind::GruntLasers => {
            commands
                .entity(monster)
                .insert(WeaponContainer::new(WeaponContainerConfig {
                    owner_id: id,
                    weapons: vec![Weapon::new(31, None)],
                }));
        }
        MonsterKind::GruntFusion => {
            commands
                .entity(monster)
                .insert(WeaponContainer::new(WeaponContainerConfig {
                    owner_id: id,
                    weapons: vec![Weapon::new(32, None)],
                }));
        }
    }

    spawn_visuals.send(SpawnVisualsEvent {
        kind: VisualsKind::Monster(kind.clone()),
        entity: monster,
    });
}

pub fn spawn_pickup(
    event: &SpawnPickup,
    commands: &mut Commands,
    spawn_visuals: &mut EventWriter<SpawnVisualsEvent>,
) {
    match event.kind {
        PickupKind::Health => {
            // has it's own system
        }
        PickupKind::BlueKey
        | PickupKind::RedKey
        | PickupKind::YellowKey
        | PickupKind::OrangeKey => {
            let pickup = spawn_gameplay_entity(
                commands,
                (
                    Pickup {
                        id: event.id,
                        kind: event.kind.clone(),
                    },
                    TransformBundle {
                        local: Transform::from_translation(event.translation),
                        ..default()
                    },
                ),
            );

            spawn_visuals.send(SpawnVisualsEvent {
                kind: VisualsKind::Pickup(event.kind.clone()),
                entity: pickup,
            });
        }
    }
}

pub fn spawn_gameplay_entity(commands: &mut Commands, bundle: impl Bundle) -> Entity {
    let entity = commands.spawn(bundle).id();
    commands.entity(entity).insert(GameplayEntity);
    entity
}

pub fn spawn_visuals_system(
    mut commands: Commands,
    mut events: EventReader<SpawnVisualsEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut atomized_materials: ResMut<Assets<AtomizedMaterial>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, MyExtension>>>,
    studio: Res<FmodStudio>,
) {
    for event in events.read() {
        match &event.kind {
            VisualsKind::Player(locally_owned) => {
                commands.entity(event.entity).with_children(|parent| {
                    if *locally_owned {
                        parent
                            .spawn((LocalPlayerVisuals, TransformBundle::default()))
                            .with_children(|parent| {
                                let headlamp_1_pos = Vec3::new(0.45, 0.0, 0.0);
                                let headlamp_2_pos = Vec3::new(-0.45, 0.0, 0.0);
                                for headlamp_index in 0..2 {
                                    let headlamp_pos = if headlamp_index == 0 {
                                        headlamp_1_pos
                                    } else {
                                        headlamp_2_pos
                                    };

                                    parent.spawn(SpotLightBundle {
                                        spot_light: SpotLight {
                                            color: Color::rgb(1.0, 0.95, 0.9),
                                            intensity: 4000.0,
                                            range: 800.0,
                                            outer_angle: 0.75,
                                            inner_angle: 0.1,
                                            shadows_enabled: true,
                                            ..default()
                                        },
                                        transform: Transform::from_translation(headlamp_pos)
                                            .looking_at(Vec3::new(0.0, 0.0, -1.0), Vec3::Y),
                                        ..default()
                                    });
                                }
                            });
                    } else {
                        parent
                            .spawn((
                                ClientInterpolate::default(),
                                PlayerVisuals,
                                MaterialMeshBundle {
                                    mesh: meshes.add(Mesh::from(Sphere {
                                        radius: 0.5,
                                        ..default()
                                    })),
                                    material: materials
                                        .add(MyExtension::color(Color::rgb(0.0, 0.0, 1.0))),
                                    transform: Transform::from_translation(Vec3::ZERO),
                                    ..default()
                                },
                            ))
                            .with_children(|parent| {
                                let headlamp_1_pos = Vec3::new(0.45, 0.0, 0.0);
                                let headlamp_2_pos = Vec3::new(-0.45, 0.0, 0.0);
                                for headlamp_index in 0..2 {
                                    let headlamp_pos = if headlamp_index == 0 {
                                        headlamp_1_pos
                                    } else {
                                        headlamp_2_pos
                                    };

                                    parent.spawn(SpotLightBundle {
                                        spot_light: SpotLight {
                                            color: Color::rgb(1.0, 0.95, 0.9),
                                            intensity: 4000.0,
                                            range: 800.0,
                                            outer_angle: 0.75,
                                            inner_angle: 0.1,
                                            shadows_enabled: true,
                                            ..default()
                                        },
                                        transform: Transform::from_translation(headlamp_pos)
                                            .looking_at(Vec3::new(0.0, 0.0, -1.0), Vec3::Y),
                                        ..default()
                                    });
                                }
                            });
                    }
                });

                if *locally_owned {
                    // Add camera with Field of View using a custom PerspectiveProjection
                    let entity = spawn_gameplay_entity(
                        &mut commands,
                        (
                            Camera3dBundle {
                                camera: Camera {
                                    hdr: true,
                                    ..default()
                                },
                                projection: Projection::Perspective(PerspectiveProjection {
                                    fov: 1.5708,
                                    ..default()
                                }),
                                transform: Transform::from_xyz(0.0, 0.0, 0.0)
                                    .looking_at(Vec3::ZERO, Vec3::Y),
                                exposure: Exposure {
                                    ev100: -1.0,
                                    ..default()
                                },
                                ..default()
                            },
                            DepthPrepass,
                        ),
                    );

                    commands
                        .entity(entity)
                        .insert(SpatialListenerBundle::default());
                }
            }
            VisualsKind::Monster(_kind) => {
                commands.entity(event.entity).with_children(|parent| {
                    parent.spawn((
                        MonsterVisuals,
                        MaterialMeshBundle {
                            mesh: meshes.add(
                                Mesh::try_from(Sphere {
                                    radius: 0.5,
                                    ..default()
                                })
                                .unwrap(),
                            ),
                            transform: Transform::from_translation(Vec3::ZERO),
                            material: materials
                                .add(MyExtension::color(Color::rgb(0.25, 0.25, 0.25))),
                            ..default()
                        },
                    ));

                    if let Ok(event_description) = studio.0.get_event(SFX_BOT_GRUNT_DRONE_FLIGHT) {
                        parent.spawn(SpatialAudioBundle::new(event_description));
                    }
                });
            }
            VisualsKind::Pickup(kind) => match kind {
                PickupKind::Health => {
                    commands.entity(event.entity).with_children(|parent| {
                        parent.spawn((
                            MaterialMeshBundle {
                                mesh: meshes.add(Mesh::from(Sphere {
                                    radius: 0.3,
                                    ..default()
                                })),
                                material: atomized_materials.add(AtomizedMaterial {}),
                                ..default()
                            },
                            NotShadowCaster,
                        ));
                        parent.spawn(PointLightBundle {
                            point_light: PointLight {
                                color: Color::rgb(0.1, 0.0, 1.0),
                                intensity: 8.0,
                                ..default()
                            },
                            ..default()
                        });
                    });
                }
                _ => {
                    let color = match kind {
                        PickupKind::BlueKey => Color::rgb(0.0, 0.0, 1.0),
                        PickupKind::RedKey => Color::rgb(1.0, 0.0, 0.0),
                        PickupKind::YellowKey => Color::rgb(1.0, 1.0, 0.0),
                        PickupKind::OrangeKey => Color::rgb(1.0, 0.5, 0.0),
                        _ => Color::rgb(1.0, 1.0, 1.0),
                    };

                    commands.entity(event.entity).with_children(|children| {
                        children.spawn(MaterialMeshBundle {
                            mesh: meshes.add(Mesh::from(Sphere {
                                radius: 0.25,
                                ..default()
                            })),
                            material: materials.add(MyExtension::color(color)),
                            ..default()
                        });
                        children.spawn(PointLightBundle {
                            point_light: PointLight {
                                color: color,
                                intensity: 8.0,
                                ..default()
                            },
                            ..default()
                        });
                    });
                }
            },
        }
    }
}
