//! Maintain a history of colliders for lag compensation
//!
//! Lag Compensation implementation:
//! - for each player collider, we will maintain a history of the collider/position/rotation in past ticks
//!   This information will be stored in a LagCompensationHistory component on the player
//! - we will also spawn a special collider that is a union of the AABB of each collider in the history.
//!   These colliders will be tagged with the LagCompensatedBroadPhase layer
//! - each bullet will be tagged with the interpolation delay `D` (tick + overstep)
//! - BroadPhase:
//!   - we perform a raycast from the bullet to the LagCompensatedBroadPhase layer to identify any potential hit
//! - NarrowPhase:
//!   - we perform a raycast from the bullet to the interpolated collider generated from the history with delay D
//!   - we don't need the SpatialQueryPipeline here, we can directly use parry
//!
//!
//! QUESTIONS:
//! - is it correct to run stuff in FixedPostUpdate between Solver and SpatialQuery?
//! - is it correct to update Pos/Rot of the hierarchy manually, since avian's hierarchy update runs in PrepareSet?
//! - do I even need to store the Collider in the history? We can assume that the Collider itself of a player doesn't change, no?
//! - assuming the Collider changes, how do I interpolate the Collider?
//!    - for now, let's assume that the collider does not change
//! - Why do I need to manually add Position/Rotation to the bot on the server for it to have correct values?
//!
//! POTENTIAL ISSUES:
//! - If the replication interval is too slow, the interpolation will be incorrect on the client (unless we store the full history)
//! - The interpolation delay is not correct and is closer to 10-11 ticks. Try to send the correct amount in time-delta directly.
use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::prelude::{GameLayer, ProjectileSet};

pub struct LagCompensationPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum LagCompensationSet {
    /// Update the collider history for all colliders
    UpdateHistory,
}

impl Plugin for LagCompensationPlugin {
    fn build(&self, app: &mut App) {
        // NOTE: we want the history at tick N to contain the collider state AFTER the physics-set ran
        // (because for example in PostUpdate::Send, we replicate the server position at tick N after the PhysicsSet ran)
        // Therefore we want to run these after the Solver, but before the SpatialQueryPipeline gets updated
        app.add_systems(
            FixedPostUpdate,
                (
                    spawn_lag_compensation_broad_phase_collider,
                    update_collider_history,
                    update_lag_compensation_broad_phase_collider,
                    // we need to update the SpatialQuery pipeline so that the queries our up-to-date
                    // with the broad-phase collider
                    |mut spatial_query: SpatialQuery| spatial_query.update_pipeline(),
                ).chain()
                .in_set(LagCompensationSet::UpdateHistory)
        );

        app.configure_sets(FixedPostUpdate, LagCompensationSet::UpdateHistory
            // NOTE: we cannot order directly with PhysicsSetSteps because those are part of a
            //  different schedule!
            .after(PhysicsSet::StepSimulation)
            .before(ProjectileSet::Hits)
        );
    }
}

/// Max number of ticks that we are keeping a collider in the history
/// (20 * 16ms = 320ms)
pub const MAX_COLLIDER_HISTORY_TICKS: u16 = 20;


/// Marker component to indicate that this collider holds the history AABB manifold
/// for lag compensation
#[derive(Component)]
pub struct LagCompensationHistoryBroadPhase;


/// ColliderData that will be included in the history
type ColliderData = (
    &'static Collider,
    &'static Position,
    &'static Rotation,
    &'static ColliderAabb,
);

/// History of the collider data for the past few ticks
pub(crate) type LagCompensationHistory = HistoryBuffer<(Collider, Position, Rotation, ColliderAabb)>;

// NOTE: we cannot use an observer for this because the ColliderAabb is not valid when the collider is created
//  it only gets updated on `update_aabb` which runs in the BroadPhase
/// Spawn a child entity that will be used for broad-phase collision detection
fn spawn_lag_compensation_broad_phase_collider(
    query: Query<(Entity, &ColliderAabb), Added<LagCompensationHistory>>,
    mut commands: Commands,
) {
    query.iter().for_each(|(entity, collider_aabb)| {
        let aabb_size = collider_aabb.size();
        debug!(?aabb_size, "spawning broad-phase collider from aabb!");
        commands.entity(entity).with_child((
            Collider::cuboid(aabb_size.x, aabb_size.y, aabb_size.z),
            // the position/rotation values don't matter here because they will be updated in the
            // `update_lag_compensation_broad_phase_collider` system
            Position::default(),
            Rotation::default(),
            LagCompensationHistoryBroadPhase,
            CollisionLayers::new(GameLayer::LagCompensatedBroadPhase, LayerMask::NONE),
        ));
    });
}

/// Update the collider of the broad-phase collider to be a union of the AABB of the colliders in the history
fn update_lag_compensation_broad_phase_collider(
    tick_manager: Res<TickManager>,
    parent_query: Query<(&Position, &Rotation, &LagCompensationHistory), Without<LagCompensationHistoryBroadPhase>>,
    mut child_query: Query<(&Parent, &mut Collider, &mut ColliderAabb, &mut Position, &mut Rotation), With<LagCompensationHistoryBroadPhase>>,
) {
    let tick = tick_manager.tick();
    // the ColliderAabb is not updated automatically when the Collider component is updated
    child_query.iter_mut().for_each(|(parent, mut collider, mut collider_aabb , mut position, mut rotation)| {
        let (parent_position, parent_rotation, history) = parent_query.get(parent.get()).unwrap();
        let (min, max) = history.into_iter().fold((Vec3::MAX, Vec3::MIN), |(min, max), (_, (_, _, _, aabb))| {
            (min.min(aabb.min), max.max(aabb.max))
        });
        // update the collider as the aabb envelope of all the colliders in the history
        // (the `update_aabb` system runs in the BroadPhase, but we need it to run after the Solver phase)
        *collider_aabb = ColliderAabb::from_min_max(min, max);
        *collider = Collider::cuboid(max.x - min.x, max.y - min.y, max.z - min.z);
        // also update the position/rotation!
        // (the `update_pos_rot` system runs in the BroadPhase, but we need it to run after the Solver phase)
        *position = *parent_position;
        *rotation = *parent_rotation;
        trace!(?tick, ?history, ?position, ?rotation, ?collider_aabb, "updated broad-phase collider");
    });
}

/// For each lag-compensated collider, store every tick a copy of the collider aabb
/// that we can use to rewind collisions
///
/// We need: ColliderAabb, ColliderLayers for a collider to be able
/// to be used in spatial queries
fn update_collider_history(
    tick_manager: Res<TickManager>,
    mut colliders: Query<(ColliderData, &mut LagCompensationHistory)>,
) {
    let tick = tick_manager.tick();
    colliders.iter_mut().for_each(|((collider, position, rotation, aabb), mut history)| {
        history.add_update(tick, (
            collider.clone(),
            position.clone(),
            rotation.clone(),
            aabb.clone()
        ));
        // TODO: add a method to pop without needing the extra clone!
        history.pop_until_tick(tick - MAX_COLLIDER_HISTORY_TICKS);
    });
}