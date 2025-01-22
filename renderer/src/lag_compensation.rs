use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use server::lag_compensation::LagCompensationHistoryBroadPhase;

pub(crate) struct LagCompensationPlugin;


impl Plugin for LagCompensationPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(add_debug_render_broad_phase_collider_aabb);
    }
}

/// Add the DebugRender to the AABB envelopes for lag compensation
fn add_debug_render_broad_phase_collider_aabb(
    trigger: Trigger<OnAdd, LagCompensationHistoryBroadPhase>,
    mut commands: Commands,
) {
    commands.entity(trigger.entity()).insert(DebugRender::collider(Color::WHITE));
}


// /// For LagCompensation we compute the AABB envelope of the collision history of each player
// /// This system will display that computed envelope
// fn show_debug_broad_phase_collider_aabb(
//     mut gizmos: Gizmos,
//     query: Query<&ColliderAabb, With<LagCompensationHistoryBroadPhase>>,
// ) {
//     for (aabb) in query.iter() {
//     }
// }

//
// /// Provide a transform to draw the AABB when starting from a cube with edge size 1
// /// centered at the origin
// fn aabb_transform(aabb: &ColliderAabb) -> GlobalTransform {
//     let transform = GlobalTransform::from(
//         Transform::from_translation(position.0)
//             .with_rotation(rotation.0)
//             .with_scale(aabb.half_extents * 2.),
//     );
//
//     transform
//         * GlobalTransform::from(
//         Transform::from_translation(aabb.center.into())
//             .with_scale((aabb.half_extents * 2.).into()),
//     )
// }