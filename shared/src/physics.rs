use bevy::prelude::*;
use avian3d::prelude::*;

pub struct PhysicsPlugin;


impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        // PLUGINS
        app.add_plugins(PhysicsPlugins::default()
                            .build()
                            // disable sync to add the sync plugin in a different schedule
                            .disable::<SyncPlugin>()
                            .disable::<PhysicsInterpolationPlugin>());
        // as an optimization, we run the sync plugin in RunFixedMainLoop (outside FixedMainLoop)
        // so that in the case of a rollback we don't do the sync again
        app.add_plugins(SyncPlugin::new(RunFixedMainLoop));

        // RESOURCES

        // Position and Rotation are the primary source of truth so no need to
        // sync changes from Transform to Position.
        // NOTE: this is CRUCIAL to avoid rollbacks! presumably because on the client
        //  we modify Transform in PostUpdate, which triggers the Sync from transform->position systems in avian
        //  Maybe those systems cause some numerical instability?
        app.insert_resource(avian3d::sync::SyncConfig {
            transform_to_position: false,
            position_to_transform: true,
            ..default()
        });
        // disable sleeping
        app.insert_resource(SleepingThreshold {
            linear: -0.01,
            angular: -0.01,
        });
        app.insert_resource(Gravity::ZERO);
    }
}