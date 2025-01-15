use crate::components::*;
use crate::in_playing_state;
use crate::net::*;
use crate::player::*;
use crate::weapons::WeaponContainer;
use bevy::prelude::*;
use bevy::text::TextLayoutInfo;
use bevy::ui::widget::TextFlags;
use bevy::utils::HashMap;
use bevy_egui::{egui, EguiContexts};

pub const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
pub const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
pub const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

#[derive(Event)]
pub struct SpawnHud;

#[derive(Default, Component)]
pub struct Hud {
    pub health: i32,
    pub red_key: bool,
    pub blue_key: bool,
    pub yellow_key: bool,
}

#[derive(Default, Component)]
pub struct HudFps {
    pub frame_count: u64,
    pub time_elapsed: f64,
}

#[derive(Component)]
pub struct HudHealth;

#[derive(Component)]
pub struct HudRedKey;

#[derive(Component)]
pub struct HudBlueKey;

#[derive(Component)]
pub struct HudYellowKey;

#[derive(Component)]
pub struct HudRespawnCountdown;

#[derive(Component, Debug)]
pub struct Crosshair {
    pub textures: HashMap<u8, Handle<Image>>,
}

pub struct HudPlugin;
impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (hud_creation_system, chat_send_system, hud_fps_system).run_if(in_playing_state),
        )
        .add_systems(
            FixedUpdate,
            (
                hud_health_system,
                hud_door_keys_system,
                button_system,
                crosshair_system,
                respawn_counter_system,
                chat_receive_system,
            )
                .run_if(in_playing_state),
        )
        .add_event::<SpawnHud>()
        .add_event::<ChatMessage>()
        .insert_resource(Chat::default());
    }
}

fn hud_creation_system(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    added_camera: Query<Entity, Added<Camera3d>>,
) {
    // hud is only created if a camera is added
    if added_camera.iter().next().is_some() {
        let default_crosshair =
            asset_server.load("crosshairs/kenney_crosshair_pack/crosshair019.png");

        commands
            .spawn((
                Hud {
                    health: 100,
                    red_key: false,
                    blue_key: false,
                    yellow_key: false,
                },
                NodeBundle {
                    style: Style {
                        // tkae up entire screen
                        position_type: PositionType::Relative,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Crosshair {
                        textures: HashMap::from_iter(
                            vec![
                                (1, default_crosshair.clone()),
                                (
                                    2,
                                    asset_server
                                        .load("crosshairs/kenney_crosshair_pack/crosshair188.png"),
                                ),
                                (
                                    3,
                                    asset_server
                                        .load("crosshairs/kenney_crosshair_pack/crosshair030.png"),
                                ),
                                (
                                    4,
                                    asset_server
                                        .load("crosshairs/kenney_crosshair_pack/crosshair043.png"),
                                ),
                                (
                                    5,
                                    asset_server
                                        .load("crosshairs/kenney_crosshair_pack/crosshair018.png"),
                                ),
                            ]
                            .into_iter(),
                        ),
                    },
                    ImageBundle {
                        image: UiImage {
                            texture: default_crosshair,
                            ..Default::default()
                        },
                        style: Style {
                            position_type: PositionType::Absolute,
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            left: Val::Percent(50.0),
                            bottom: Val::Percent(50.0),
                            margin: UiRect {
                                left: Val::Px(-32.0),
                                bottom: Val::Px(-32.0),
                                ..default()
                            },
                            ..default()
                        },
                        ..default()
                    },
                ));

                parent.spawn((
                    HudRespawnCountdown,
                    TextBundle {
                        style: Style {
                            left: Val::Percent(50.0),
                            top: Val::Percent(50.0),
                            ..default()
                        },
                        text: Text {
                            sections: vec![TextSection {
                                value: "Spawn in: 0".to_string(),
                                style: TextStyle {
                                    font: Handle::default(),
                                    font_size: 20.0,
                                    color: Color::WHITE,
                                },
                            }],
                            ..default()
                        },
                        ..default()
                    },
                ));

                parent.spawn((
                    HudFps::default(),
                    TextBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            left: Val::Px(20.0),
                            top: Val::Px(20.0),
                            ..default()
                        },
                        text: Text {
                            sections: vec![TextSection {
                                value: "FPS: 0".to_string(),
                                style: TextStyle {
                                    font: Handle::default(),
                                    font_size: 20.0,
                                    color: Color::ORANGE_RED,
                                },
                            }],
                            ..default()
                        },
                        ..default()
                    },
                ));

                parent.spawn((
                    HudHealth,
                    TextBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            right: Val::Px(20.0),
                            bottom: Val::Px(20.0),
                            ..default()
                        },
                        text: Text {
                            sections: vec![TextSection {
                                value: "Health: 0".to_string(),
                                style: TextStyle {
                                    font: Handle::default(),
                                    font_size: 20.0,
                                    color: Color::WHITE,
                                },
                            }],
                            ..default()
                        },
                        ..default()
                    },
                ));

                parent.spawn((
                    HudRedKey,
                    TextBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            right: Val::Px(20.0),
                            bottom: Val::Px(50.0),
                            ..default()
                        },
                        text: Text {
                            sections: vec![TextSection {
                                value: "".to_string(),
                                style: TextStyle {
                                    font: Handle::default(),
                                    font_size: 20.0,
                                    color: Color::RED,
                                },
                            }],
                            ..default()
                        },
                        ..default()
                    },
                ));

                parent.spawn((
                    HudBlueKey,
                    TextBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            right: Val::Px(20.0),
                            bottom: Val::Px(80.0),
                            ..default()
                        },
                        text: Text {
                            sections: vec![TextSection {
                                value: "".to_string(),
                                style: TextStyle {
                                    font: Handle::default(),
                                    font_size: 20.0,
                                    color: Color::BLUE,
                                },
                            }],
                            ..default()
                        },
                        ..default()
                    },
                ));

                parent.spawn((
                    HudYellowKey,
                    TextBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            right: Val::Px(20.0),
                            bottom: Val::Px(110.0),
                            ..default()
                        },
                        text: Text {
                            sections: vec![TextSection {
                                value: "".to_string(),
                                style: TextStyle {
                                    font: Handle::default(),
                                    font_size: 20.0,
                                    color: Color::YELLOW,
                                },
                            }],
                            ..default()
                        },
                        ..default()
                    },
                ));
            });
    }
}

fn cleanup_hud(mut commands: Commands, query: Query<Entity, With<Hud>>) {
    for e in query.iter() {
        if let Some(e) = commands.get_entity(e) {
            e.despawn_recursive();
        }
    }
}

fn hud_health_system(
    mut q_hud_health: Query<(&mut Text, &mut HudHealth)>,

    q_health: Query<&Health, With<LocallyOwned>>,
) {
    for (mut text, _) in q_hud_health.iter_mut() {
        for health in q_health.iter() {
            text.sections[0].value = format!("Health: {}", health.current());
        }
    }
}

fn hud_door_keys_system(
    mut q_hud_red_key: Query<
        &mut Text,
        (With<HudRedKey>, Without<HudBlueKey>, Without<HudYellowKey>),
    >,
    mut q_hud_blue_key: Query<
        &mut Text,
        (Without<HudRedKey>, With<HudBlueKey>, Without<HudYellowKey>),
    >,
    mut q_hud_yellow_key: Query<
        &mut Text,
        (Without<HudRedKey>, Without<HudBlueKey>, With<HudYellowKey>),
    >,
    mut ev_reader: EventReader<DoorKeyPickupEvent>,
) {
    for event in ev_reader.read() {
        match event.key {
            DoorKey::Red => {
                for mut text in q_hud_red_key.iter_mut() {
                    text.sections[0].value = "Red Key".to_string();
                }
            }
            DoorKey::Blue => {
                for mut text in q_hud_blue_key.iter_mut() {
                    text.sections[0].value = "Blue Key".to_string();
                }
            }
            DoorKey::Yellow => {
                for mut text in q_hud_yellow_key.iter_mut() {
                    text.sections[0].value = "Yellow Key".to_string();
                }
            }
        }
    }
}

fn hud_fps_system(time: Res<Time>, mut query: Query<(&mut Text, &mut HudFps)>) {
    for (mut text, mut counter) in query.iter_mut() {
        counter.frame_count += 1;
        counter.time_elapsed += time.delta_seconds_f64();

        // Update FPS every second
        if counter.time_elapsed >= 1.0 {
            let fps = counter.frame_count as f64 / counter.time_elapsed;
            text.sections[0].value = format!("FPS: {:.2}", fps);

            // Reset the counter
            counter.frame_count = 0;
            counter.time_elapsed = 0.0;
        }
    }
}

fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
            }
        }
    }
}

fn crosshair_system(
    health: Query<&Health, With<LocallyOwned>>,
    weapon: Query<&WeaponContainer, With<LocallyOwned>>,
    mut crosshair: Query<(Entity, &Crosshair, &mut UiImage)>,
    mut commands: Commands,
) {
    if let Ok(health) = health.get_single() {
        // if dead render nothing
        if health.dead() {
            for (entity, _, _) in crosshair.iter_mut() {
                if let Some(mut entity) = commands.get_entity(entity) {
                    entity.insert(Visibility::Hidden);
                }
            }
        }
        // if alive render crosshair
        else if let Ok(weapon_container) = weapon.get_single() {
            for (entity, crosshair, mut image) in crosshair.iter_mut() {
                if let Some(mut entity) = commands.get_entity(entity) {
                    entity.insert(Visibility::Visible);
                    if let Some(crosshair) =
                        crosshair.textures.get(&weapon_container.weapons[0].key)
                    {
                        if image.texture != *crosshair {
                            image.texture = crosshair.clone();
                        }
                    }
                }
            }
        }
    // player hasn't been created yet
    } else {
        for (entity, _, _) in crosshair.iter_mut() {
            if let Some(mut entity) = commands.get_entity(entity) {
                entity.insert(Visibility::Hidden);
            }
        }
    }
}

fn respawn_counter_system(
    time: Res<Time>,
    local_player: Res<LocalPlayer>,
    mut players: Query<(&Health, &mut Player), With<LocallyOwned>>,
    mut respawn_counter: Query<&mut Text, With<HudRespawnCountdown>>,
) {
    if let Ok((health, mut player)) = players.get_single_mut() {
        for mut text in respawn_counter.iter_mut() {
            if health.dead() {
                // we tick here if we're a client, otherwise the game mode ticks the timer
                if !local_player.has_authority() {
                    player.respawn_timer.tick(time.delta());
                }

                text.sections[0].value = format!(
                    "Respawn in {}",
                    player.respawn_timer.remaining_secs().ceil() as u32
                );
            } else {
                text.sections[0].value = "".to_string();
            }
        }
    // player hasn't been created yet
    } else {
        for mut text in respawn_counter.iter_mut() {
            text.sections[0].value = "".to_string();
        }
    }
}

#[derive(Resource, Default)]
pub struct Chat {
    pub show: bool,
    pub message: String,
    pub messages: Vec<String>,
}

#[derive(Component)]
pub struct ChatComponent;

#[derive(Event)]
pub struct ChatMessage {
    pub message: String,
}

fn chat_receive_system(
    mut chat: ResMut<Chat>,
    mut chat_event_reader: EventReader<ChatMessage>,
    mut components: Query<(&mut Text, &ChatComponent)>,
) {
    let mut needs_update = false;
    for message in chat_event_reader.read() {
        chat.messages.push(message.message.clone());
        needs_update = true;
    }

    if needs_update {
        while chat.messages.len() > 10 {
            chat.messages.remove(0);
        }

        for (mut text, _) in components.iter_mut() {
            text.sections[0].value = {
                let mut value = String::new();
                for message in chat.messages.iter() {
                    value.push_str(&message);
                    value.push_str("\n");
                }
                value
            };
        }
    }
}

fn chat_send_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut contexts: EguiContexts,
    mut chat: ResMut<Chat>,
    mut chat_event_writer: EventWriter<ChatMessage>,
) {
    if !chat.show {
        if chat.message.is_empty() && keyboard_input.just_pressed(KeyCode::KeyT) {
            chat.show = true;
        }
    } else {
        if keyboard_input.just_pressed(KeyCode::Escape) {
            chat.show = false;
            chat.message = String::new();
        }

        if keyboard_input.just_pressed(KeyCode::Enter) {
            chat.show = false;
            chat_event_writer.send(ChatMessage {
                message: chat.message.clone(),
            });
            chat.message = String::new();
        }
    }

    // chat input
    if chat.show {
        egui::Window::new("Chat")
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::new(0.0, -100.0))
            .show(&contexts.ctx_mut(), |ui| {
                ui.vertical(|ui| {
                    ui.text_edit_singleline(&mut chat.message).request_focus();
                });
            });
    }
}
