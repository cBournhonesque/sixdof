use avian3d::math::AsF32;
use bevy::prelude::*;
use avian3d::prelude::*;
use avian3d::sync::SyncSet;

pub(crate) struct PhysicsPlugin;


/// Collision layers
#[derive(PhysicsLayer, Default)]
pub enum GameLayer {
    #[default]
    Default,
    Wall,
    Projectile,
    Player,
    /// Used for lag compensation: we will check the collision between the bullet and the AABB bounding box
    /// of the collider + it's history
    LagCompensatedBroadPhase,
}

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        // PLUGINS
        app.add_plugins(PhysicsPlugins::default()
                            .build()
                            .disable::<SyncPlugin>()
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

        // SYSTEMS
        app.add_systems(
            FixedPostUpdate,
            position_to_transform
                .in_set(SyncSet::PositionToTransform)
        );
        app.add_systems(
            PostUpdate,
            position_to_transform
                .in_set(SyncSet::PositionToTransform)
        );
        app.configure_sets(
            PostUpdate, SyncSet::PositionToTransform.in_set(PhysicsSet::Sync)
        );
        app.add_systems(Update, log_collisions);

        // RESOURCES
        // disable sleeping
        app.insert_resource(SleepingThreshold {
            linear: -0.01,
            angular: -0.01,
        });
        app.insert_resource(Gravity::ZERO);
    }
}

type PosToTransformComponents = (
    &'static mut Transform,
    &'static Position,
    &'static Rotation,
    Option<&'static Parent>,
);

type ParentComponents = (
    &'static GlobalTransform,
    Option<&'static Position>,
    Option<&'static Rotation>,
);

pub fn position_to_transform(
    mut query: Query<PosToTransformComponents, Or<(Changed<Position>, Changed<Rotation>)>>,
    parents: Query<ParentComponents, With<Children>>,
) {
    for (mut transform, pos, rot, parent) in &mut query {
        if let Some(parent) = parent {
            if let Ok((parent_transform, parent_pos, parent_rot)) = parents.get(**parent) {
                // Compute the global transform of the parent using its Position and Rotation
                let parent_transform = parent_transform.compute_transform();
                let parent_pos = parent_pos.map_or(parent_transform.translation, |pos| pos.f32());
                let parent_rot = parent_rot.map_or(parent_transform.rotation, |rot| rot.f32());
                let parent_scale = parent_transform.scale;
                let parent_transform = Transform::from_translation(parent_pos)
                    .with_rotation(parent_rot)
                    .with_scale(parent_scale);

                // The new local transform of the child body,
                // computed from the its global transform and its parents global transform
                let new_transform = GlobalTransform::from(
                    Transform::from_translation(pos.f32()).with_rotation(rot.f32()),
                )
                    .reparented_to(&GlobalTransform::from(parent_transform));

                transform.translation = new_transform.translation;
                transform.rotation = new_transform.rotation;
            }
        } else {
            transform.translation = pos.f32();
            transform.rotation = rot.f32();
            // info!(?transform, ?pos, ?rot, "PosToTransform");
        }
    }
}

pub fn log_collisions(
    mut event_reader: EventReader<Collision>,
) {
        for Collision(contacts) in event_reader.read() {
        println!(
            "Entities {} and {} are colliding",
            contacts.entity1,
            contacts.entity2,
        );
    }
}