use crate::pickups::*;
use crate::player::*;
use crate::spawn::*;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use qevy::components::*;

pub const GRAVITY: f32 = 0.0;

pub fn door_system(
    time: Res<Time>,
    mut ev_reader: EventReader<TriggeredEvent>,
    mut q_doors: Query<(&mut Door, &TriggerTarget, &mut Transform, &Mover)>,
    q_red_key: Query<&RedKey, With<Player>>,
    q_blue_key: Query<&BlueKey, With<Player>>,
    q_yellow_key: Query<&YellowKey, With<Player>>,
) {
    for triggered_event in ev_reader.read() {
        for (mut door, trigger_target, _, _) in q_doors.iter_mut() {
            if trigger_target.target_name == triggered_event.target {
                if door.key.is_some() {
                    match door.key.clone().unwrap().as_str() {
                        "red" => {
                            if let Ok(_) = q_red_key.get(triggered_event.triggered_by) {
                                door.triggered_time = Some(std::time::Instant::now());
                            }
                        }
                        "blue" => {
                            if let Ok(_) = q_blue_key.get(triggered_event.triggered_by) {
                                door.triggered_time = Some(std::time::Instant::now());
                            }
                        }
                        "yellow" => {
                            if let Ok(_) = q_yellow_key.get(triggered_event.triggered_by) {
                                door.triggered_time = Some(std::time::Instant::now());
                            }
                        }
                        _ => {}
                    }
                } else {
                    door.triggered_time = Some(std::time::Instant::now());
                }
            }
        }
    }

    for (mut door, _, mut transform, mover) in q_doors.iter_mut() {
        let triggered = if door.open_once {
            door.triggered_time.is_some()
        } else {
            door.triggered_time.is_some()
                && (std::time::Instant::now() - door.triggered_time.unwrap() < door.open_time)
        };

        if triggered {
            let destination = mover.destination_translation;
            let direction = destination - transform.translation;
            let move_distance = mover.speed * time.delta_seconds();

            if direction.length() < move_distance {
                transform.translation = destination;
            } else {
                transform.translation += direction.normalize() * move_distance;
            }
        } else {
            if !door.open_once && door.triggered_time.is_some() {
                continue;
            }
            door.triggered_time = None;

            let destination = mover.start_translation;
            let direction = destination - transform.translation;
            let move_distance = mover.speed * time.delta_seconds();

            if direction.length() < move_distance {
                transform.translation = destination;
            } else {
                transform.translation += direction.normalize() * move_distance;
            }
        }
    }
}
