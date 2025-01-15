use bevy::prelude::*;
use shared::player::Player;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        // SYSTEMS
    }
}

// TODO: what is LocalPlayerVisuals?
// pub fn player_camera_system(
//     mut camera: Query<&mut Transform, With<Camera>>,
//     player: Query<&Transform, (With<Player>, Without<Camera>)>,
//     visuals: Query<
//         (&GlobalTransform, &Parent),
//         (With<LocalPlayerVisuals>, Without<Player>, Without<Camera>),
//     >,
// ) {
//     if let Ok(mut camera_transform) = camera.get_single_mut() {
//         if let Ok((visuals_transform, parent)) = visuals.get_single() {
//             let entity = parent.get();
//             if let Ok(player_transform) = player.get(entity) {
//                 camera_transform.translation = visuals_transform.translation();
//                 camera_transform.rotation = player_transform.rotation;
//             }
//         }
//     }
// }