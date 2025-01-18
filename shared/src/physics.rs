use bevy::prelude::*;
use avian3d::prelude::*;

pub struct PhysicsPlugin;


impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        // PLUGINS
        app.add_plugins(PhysicsPlugins::default()
                            .build()
                            .disable::<PhysicsInterpolationPlugin>());
        // // as an optimization, we run the sync plugin in RunFixedMainLoop (outside FixedMainLoop)
        // // so that in the case of a rollback we don't do the sync again
        // app.add_plugins(SyncPlugin::new(RunFixedMainLoop));

        // RESOURCES

        // disable sleeping
        app.insert_resource(SleepingThreshold {
            linear: -0.01,
            angular: -0.01,
        });
        app.insert_resource(Gravity::ZERO);
    }
}