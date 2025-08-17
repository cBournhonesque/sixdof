use bevy::prelude::*;
use avian3d::prelude::*;

pub(crate) struct PhysicsPlugin;


/// Collision layers
#[derive(PhysicsLayer, Default)]
pub enum GameLayer {
    #[default]
    Default,
    Wall,
    Projectile,
    Ship,
    /// Used for lag compensation: we will check the collision between the bullet and the AABB bounding box
    /// of the collider + it's history
    LagCompensatedBroadPhase,
}

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        // PLUGINS
        app.add_plugins(PhysicsPlugins::default()
                            .build()
                            .disable::<SleepingPlugin>()
                            .disable::<PhysicsInterpolationPlugin>());

        // SyncPlugin
        // 1. The SyncPlugin transform_to_position system causes issues (see https://github.com/Jondolf/avian/issues/634)
        // 2. The SyncPlugin only works on RigidBodies but we want to run it on all entities (for example for interpolated entities we do not add
        //    a RigidBody) so instead we roll out our own sync system
        // 3. We still need to run the SyncPlugin in FixedPostUpdate; if we run it in RunFixedMainLoop, the visual interpolation will have
        //    empty Transform values when updating VisualInterpolation. VisualInterpolationUpdate runs in FixedLast.
        // 4. We also need to run the SyncPlugin in FixedUpdate, for example if you have multiple Updates in a row without FixedUpdates,
        //    and the first one triggers a rollback with Correction. Then on the first frame we reset the Position to the original_prediction
        //    and we need a sync to make sure that visually we also use this value!

        // RESOURCES
        // disable sleeping
        app.insert_resource(SleepingThreshold {
            linear: -0.01,
            angular: -0.01,
        });
        app.insert_resource(Gravity::ZERO);
    }
}
