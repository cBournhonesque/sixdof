use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::client::InterpolationDelay;
use lightyear_avian::prelude::LagCompensationSpatialQuery;
use shared::{prelude::{Damageable, GameLayer, Projectile}, weapons::{ProjectileHitEvent, WeaponsData}};

/// Handles projectiles colliding with walls and enemies
pub(crate) struct WeaponsPlugin;
impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ProjectileHitEvent>();
        app.add_systems(FixedPostUpdate, projectile_hit_system.run_if(resource_exists::<WeaponsData>));
    }
}

fn projectile_hit_system(
    mut commands: Commands,
    mut events: EventReader<ProjectileHitEvent>,
    mut weapons_data: ResMut<WeaponsData>,
    mut damageables: Query<&mut Damageable>,
) {
    for event in events.read() {
        // by this point the projectile itself has already been queued for despawn so we dont need to worry about the projectile itself
        if let Some(weapon_data) = weapons_data.weapons.get_mut(&event.weapon_index) {
            // @todo-brian: apply splash damage
            if let Some(entity_hit) = event.entity_hit {
                if let Ok(mut damageable) = damageables.get_mut(entity_hit) {
                    damageable.health = damageable.health.saturating_sub(weapon_data.projectile.direct_damage);
                    if damageable.health == 0 {
                        commands.entity(entity_hit).despawn_recursive();
                    }
                }
            }
        }
    }
}
