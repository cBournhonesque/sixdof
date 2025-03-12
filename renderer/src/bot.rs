use bevy::color::palettes::basic::RED;
use bevy::prelude::*;
use lightyear::prelude::client::{InterpolateStatus, VisualInterpolateStatus};
use shared::bot::Bot;
use crate::VisibleFilter;

pub(crate) struct BotPlugin;
impl Plugin for BotPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_visuals);
    }
}

/// Add visuals to newly spawned bots
fn spawn_visuals(
    trigger: Trigger<OnAdd, Bot>,
    query: Query<(), VisibleFilter>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    // mut atomized_materials: ResMut<Assets<AtomizedMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let entity = trigger.entity();
    if query.get(entity).is_ok() {
        commands.entity(entity).insert((
            Visibility::default(),
            Transform::from_xyz(0.0, 10.0, 0.0),
            Mesh3d(meshes.add(Mesh::from(Sphere {
                radius: 0.5,
                ..default()
            }))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: RED.into(),
                ..Default::default()
            })),
        ));
    }
}
