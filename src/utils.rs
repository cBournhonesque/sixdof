use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use std::time::SystemTime;

pub fn timestamp_millis_since_epoch() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn turn_off(entity: Entity, commands: &mut Commands) {
    if let Some(mut entity) = commands.get_entity(entity) {
        entity.insert(Visibility::Hidden);
        entity.insert(CollisionGroups::new(
            crate::physics::COLLISION_GROUP_NO_COLLISION,
            crate::physics::COLLISION_GROUP_NO_COLLISION,
        ));
    }
}

pub fn turn_on(entity: Entity, commands: &mut Commands) {
    if let Some(mut entity) = commands.get_entity(entity) {
        entity.insert(Visibility::Visible);
        entity.insert(CollisionGroups::new(
            crate::physics::COLLISION_GROUP_DYNAMIC,
            crate::physics::COLLISION_GROUP_MAX,
        ));
    }
}
