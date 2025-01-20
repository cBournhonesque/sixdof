use avian3d::prelude::PhysicsDebugPlugin;
use bevy::prelude::*;


pub(crate) struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        // PLUGINS
        // draw debug shapes in Last to make sure that TransformPropagate has run?
        app.add_plugins(PhysicsDebugPlugin::new(Last));
    }
}