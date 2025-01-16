// use std::net::{IpAddr, Ipv4Addr};
//
// use shared::prelude::*;
// use bevy::{prelude::*, window::CursorGrabMode};
// use bevy_egui::{
//     egui::{self, Align2},
//     EguiContexts,
// };
//
// #[derive(Debug, Clone, Eq, PartialEq)]
// enum ActiveModal {
//     ExitGame,
//     QuitToTitleScreen,
// }
//
// #[derive(Debug, Clone, Eq, PartialEq)]
// enum ActiveMultiplayerMenu {
//     Host,
//     Join,
// }
//
// #[derive(Debug, Clone, Eq, PartialEq)]
// enum ActiveMenu {
//     Main,
//     Multiplayer(ActiveMultiplayerMenu),
//     Options,
//     Controls,
// }
//
// impl Default for ActiveMenu {
//     fn default() -> Self {
//         ActiveMenu::Main
//     }
// }
//
// #[derive(Resource)]
// struct Menu {
//     active_modal: Option<ActiveModal>,
//     active_menu: ActiveMenu,
//     server_port: String,
//     server_game_mode: GameMode,
//     server_map: String,
//     join_port: String,
//     join_ip: String,
// }
//
// pub struct MenuPlugin;
// impl Plugin for MenuPlugin {
//     fn build(&self, app: &mut App) {
//         app.add_systems(
//             Update,
//             (menu_system)
//                 .chain()
//                 .run_if(in_state(AppState::TitleScreen).or_else(in_playing_state)),
//         )
//             .insert_resource(Menu {
//                 active_modal: None,
//                 active_menu: ActiveMenu::Main,
//                 server_port: DEFAULT_PORT.to_string(),
//                 server_game_mode: GameMode::Deathmatch,
//                 server_map: "m4".to_string(),
//                 join_port: DEFAULT_PORT.to_string(),
//                 join_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)).to_string(),
//             });
//     }
// }
//
// fn menu_system(
//     world_state: Res<State<WorldState>>,
//     current_app_state: Res<State<AppState>>,
//     mut next_app_state: ResMut<NextState<AppState>>,
//     keyboard_input: Res<ButtonInput<KeyCode>>,
//     mut ctx: EguiContexts,
//     mut window: Query<&mut Window>,
//     mut menu: ResMut<Menu>,
//     mut app_events: EventWriter<AppEvent>,
// ) {
//     let display_menu = match current_app_state.get() {
//         AppState::TitleScreen => true,
//         AppState::Playing(PlayingSubState::Menu) => true,
//         _ => false,
//     };
//
//     let is_title_screen = *current_app_state.get() == AppState::TitleScreen;
//
//     if display_menu {
//         if let Ok(mut window) = window.get_single_mut() {
//             window.cursor.visible = true;
//             window.cursor.grab_mode = CursorGrabMode::Confined;
//         }
//
//         if !is_title_screen {
//             if keyboard_input.just_pressed(KeyCode::Escape) {
//                 close_menu(
//                     world_state.get() == &WorldState::SinglePlayer,
//                     &mut next_app_state,
//                 );
//             }
//         }
//     } else {
//         if let Ok(mut window) = window.get_single_mut() {
//             window.cursor.visible = false;
//             window.cursor.grab_mode = CursorGrabMode::Locked;
//         }
//
//         if !is_title_screen {
//             if keyboard_input.just_pressed(KeyCode::Escape) {
//                 open_menu(
//                     world_state.get() == &WorldState::SinglePlayer,
//                     &mut next_app_state,
//                     &mut menu,
//                 );
//             }
//         }
//     }
//
//     if display_menu {
//         let window_size = {
//             let window = ctx.ctx().screen_rect();
//             (window.width(), window.height())
//         };
//
//         let title = if is_title_screen { "Quantsum" } else { "Menu" };
//
//         egui::Window::new(title)
//             .collapsible(false)
//             .resizable(false)
//             .movable(false)
//             .pivot(Align2::CENTER_CENTER)
//             .default_pos((window_size.0 / 2.0, window_size.1 / 2.0))
//             .show(&ctx.ctx_mut(), |ui| {
//                 ui.horizontal(|ui| {
//                     if *current_app_state == AppState::TitleScreen {
//                         if ui.button("Main").clicked() {
//                             menu.active_menu = ActiveMenu::Main;
//                         }
//                     }
//                     if ui.button("Options").clicked() {
//                         menu.active_menu = ActiveMenu::Options;
//                     }
//                     if ui.button("Controls").clicked() {
//                         menu.active_menu = ActiveMenu::Controls;
//                     }
//                 });
//
//                 ui.separator();
//
//                 match &menu.active_menu {
//                     ActiveMenu::Main => {
//                         if *current_app_state == AppState::TitleScreen {
//                             ui.heading("Main");
//                             ui.separator();
//                             if ui.button("Single Player").clicked() {
//                                 app_events.send(AppEvent::StartSinglePlayer);
//                                 close_menu(
//                                     world_state.get() == &WorldState::SinglePlayer,
//                                     &mut next_app_state,
//                                 );
//                             }
//                             if ui.button("Multiplayer").clicked() {
//                                 menu.active_menu =
//                                     ActiveMenu::Multiplayer(ActiveMultiplayerMenu::Join);
//                             }
//                         }
//                     }
//                     ActiveMenu::Multiplayer(active_multiplayer_menu) => {
//                         match active_multiplayer_menu {
//                             ActiveMultiplayerMenu::Host => {
//                                 ui.heading("Multiplayer: Host");
//
//                                 ui.label("Port");
//                                 ui.text_edit_singleline(&mut menu.server_port);
//                                 menu.server_port.retain(|c| c.is_numeric());
//
//                                 ui.label("Game Mode");
//                                 ui.radio_value(
//                                     &mut menu.server_game_mode,
//                                     GameMode::Deathmatch,
//                                     "Deathmatch",
//                                 );
//                                 ui.radio_value(&mut menu.server_game_mode, GameMode::Coop, "Coop");
//
//                                 ui.label("Map");
//                                 ui.radio_value(&mut menu.server_map, "m4".to_string(), "Map 4");
//                                 ui.radio_value(&mut menu.server_map, "dm".to_string(), "Construct");
//
//                                 if ui.button("Start Server").clicked() {
//                                     app_events.send(AppEvent::HostServer(ServerSettings {
//                                         port: menu.server_port.parse().unwrap_or(DEFAULT_PORT),
//                                         game_mode: menu.server_game_mode.clone(),
//                                         map: menu.server_map.clone(),
//                                     }));
//                                     close_menu(
//                                         world_state.get() == &WorldState::SinglePlayer,
//                                         &mut next_app_state,
//                                     );
//                                 }
//                                 ui.separator();
//                                 ui.horizontal(|ui| {
//                                     if ui.button("Join").clicked() {
//                                         menu.active_menu =
//                                             ActiveMenu::Multiplayer(ActiveMultiplayerMenu::Join);
//                                     }
//                                     if ui.button("Back").clicked() {
//                                         menu.active_menu = ActiveMenu::Main;
//                                     }
//                                 });
//                             }
//                             ActiveMultiplayerMenu::Join => {
//                                 ui.heading("Multiplayer: Join");
//
//                                 ui.text_edit_singleline(&mut menu.join_port);
//                                 menu.join_port.retain(|c| c.is_numeric());
//
//                                 ui.text_edit_singleline(&mut menu.join_ip);
//                                 menu.join_ip.retain(|c| c.is_numeric() || c == '.');
//
//                                 if ui.button("Join Server").clicked() {
//                                     app_events.send(AppEvent::JoinServer(ClientSettings {
//                                         port: menu.join_port.parse().unwrap_or(DEFAULT_PORT),
//                                         ip: menu
//                                             .join_ip
//                                             .parse()
//                                             .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)),
//                                         player_name: "Player!!!".to_string(),
//                                     }));
//                                     close_menu(
//                                         world_state.get() == &WorldState::SinglePlayer,
//                                         &mut next_app_state,
//                                     );
//                                 }
//
//                                 ui.separator();
//                                 ui.horizontal(|ui| {
//                                     if ui.button("Host").clicked() {
//                                         menu.active_menu =
//                                             ActiveMenu::Multiplayer(ActiveMultiplayerMenu::Host);
//                                     }
//                                     if ui.button("Back").clicked() {
//                                         menu.active_menu = ActiveMenu::Main;
//                                     }
//                                 });
//                             }
//                         }
//                     }
//                     ActiveMenu::Options => {
//                         ui.heading("Options");
//                     }
//                     ActiveMenu::Controls => {
//                         ui.heading("Controls");
//                     }
//                 }
//
//                 ui.separator();
//
//                 ui.horizontal(|ui| {
//                     if ui.button("Exit Game").clicked() {
//                         menu.active_modal = Some(ActiveModal::ExitGame);
//                     }
//                     if !is_title_screen {
//                         if ui.button("Quit").clicked() {
//                             menu.active_modal = Some(ActiveModal::QuitToTitleScreen);
//                         }
//                         if ui.button("Resume").clicked() {
//                             close_menu(
//                                 world_state.get() == &WorldState::SinglePlayer,
//                                 &mut next_app_state,
//                             );
//                         }
//                     }
//                 });
//             });
//
//         if let Some(active_modal) = &menu.active_modal {
//             match active_modal {
//                 ActiveModal::QuitToTitleScreen => {
//                     egui::Window::new("Quit to Title Screen")
//                         .collapsible(false)
//                         .resizable(false)
//                         .movable(false)
//                         .pivot(Align2::CENTER_CENTER)
//                         .default_pos((window_size.0 / 2.0, window_size.1 / 2.0))
//                         .default_size((200.0, 100.0))
//                         .show(&ctx.ctx_mut(), |ui| {
//                             ui.label("Are you sure you want to quit to the title screen?");
//                             ui.separator();
//                             ui.horizontal(|ui| {
//                                 if ui.button("Yes").clicked() {
//                                     app_events.send(AppEvent::GotoTitleScreen);
//                                     menu.active_modal = None;
//                                 }
//                                 if ui.button("No").clicked() {
//                                     menu.active_modal = None;
//                                 }
//                             });
//                         });
//                 }
//                 ActiveModal::ExitGame => {
//                     egui::Window::new("Exit Game")
//                         .collapsible(false)
//                         .resizable(false)
//                         .movable(false)
//                         .pivot(Align2::CENTER_CENTER)
//                         .default_pos((window_size.0 / 2.0, window_size.1 / 2.0))
//                         .default_size((200.0, 100.0))
//                         .show(&ctx.ctx_mut(), |ui| {
//                             ui.label("Are you sure you want to exit the game?");
//                             ui.separator();
//                             ui.horizontal(|ui| {
//                                 if ui.button("Yes").clicked() {
//                                     std::process::exit(0);
//                                 }
//                                 if ui.button("No").clicked() {
//                                     menu.active_modal = None;
//                                 }
//                             });
//                         });
//                 }
//             }
//         }
//     }
// }
//
// fn open_menu(
//     single_player: bool,
//     next_app_state: &mut ResMut<NextState<AppState>>,
//     menu: &mut ResMut<Menu>,
// ) {
//     next_app_state.set(AppState::Playing(PlayingSubState::Menu));
//     menu.active_modal = None;
//     menu.active_menu = ActiveMenu::Main;
// }
//
// fn close_menu(single_player: bool, next_app_state: &mut ResMut<NextState<AppState>>) {
//     next_app_state.set(AppState::Playing(PlayingSubState::Playing));
// }