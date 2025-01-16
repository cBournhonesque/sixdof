use bevy::prelude::*;
use bevy::prelude::TransformSystem::TransformPropagate;
use lightyear::shared::replication::components::Controlled;
use shared::player::Player;

pub(crate) struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, player_camera_system.after(TransformPropagate));
    }
}

/// Make the camera follow the controlled player
pub fn player_camera_system(
    mut camera: Query<&mut Transform, With<Camera>>,
    player: Query<&Transform, (With<Player>, With<Controlled>, Without<Camera>)>,
) {
    if let Ok(mut camera_transform) = camera.get_single_mut() {
        if let Ok(player_transform) = player.get_single() {
            camera_transform.translation = player_transform.translation;
            camera_transform.rotation = player_transform.rotation;
        }
    }
}