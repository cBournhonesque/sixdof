use bevy::prelude::*;
use avian3d::prelude::*;

pub struct PhysicsPlugin;


impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        // TODO: make adjustments similar to lightyear examples
        app.add_plugins(PhysicsPlugins::default());
    }
}