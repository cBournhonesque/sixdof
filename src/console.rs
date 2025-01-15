use crate::gamemode::*;
use crate::hud::ChatMessage;
use bevy::prelude::*;

static COMMAND_HELP: [(&'static str, &'static str); 2] = [
    ("/help", "print this help"),
    (
        "/map map_name (required) gamemode_name (optional)",
        "change the map and gamemode",
    ),
];

pub struct ConsolePlugin;
impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, read_system)
            .add_event::<ConsoleCommandOutput>();
    }
}

#[derive(Event)]
pub struct ConsoleCommandOutput {
    pub message: String,
}

pub fn read_system(
    local_player: Res<crate::player::LocalPlayer>,
    mut chat_messages: EventReader<ChatMessage>,
    mut ouput_writer: EventWriter<ConsoleCommandOutput>,
    mut gamemode_controller: ResMut<GameModeController>,
) {
    // for chat_message in chat_messages.read() {
    //     if is_slash_command(&chat_message.message) {
    //         let command = chat_message
    //             .message
    //             .trim()
    //             .split_whitespace()
    //             .collect::<Vec<&str>>();
    //         match command[0].to_lowercase().as_str() {
    //             "/help" => {
    //                 for (command, description) in COMMAND_HELP.iter() {
    //                     ouput_writer.send(ConsoleCommandOutput {
    //                         message: format!("{}: {}", command, description),
    //                     });
    //                 }
    //             }
    //             "/map" => {
    //                 if local_player.has_authority() {
    //                     if command.len() > 1 {
    //                         let map = command[1].to_lowercase();
    //                         let map = map.as_str();
    //                         gamemode_controller.set_map(map);

    //                         if command.len() > 2 {
    //                             let gamemode = crate::gamemode::parse_gamemode(command[2]);
    //                             gamemode_controller.set_map_and_gamemode(map, &gamemode);
    //                         }
    //                     }
    //                 }
    //             }
    //             _ => {}
    //         }
    //     }
    // }
}

fn is_slash_command(message: &str) -> bool {
    message.trim().starts_with('/')
}
