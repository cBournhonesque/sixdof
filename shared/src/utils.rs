use bevy::prelude::*;


/// Can be added to an entity so that it's despawned after the timer has finished
#[derive(Component)]
pub struct DespawnAfter(pub Timer);

pub struct UtilsPlugin;

impl Plugin for UtilsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, despawn_after);
    }
}

/// Despawn entities after their timer has finished
fn despawn_after(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut DespawnAfter)>,
) {
    for (entity, mut despawn_after) in query.iter_mut() {
        despawn_after.0.tick(time.delta());
        if despawn_after.0.finished() {
            commands.entity(entity).despawn();
        }
    }
}