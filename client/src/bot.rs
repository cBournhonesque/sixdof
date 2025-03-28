use avian3d::prelude::{Collider, Position};
use bevy::prelude::*;
use lightyear::client::interpolation::Interpolated;
use lightyear::prelude::client::InterpolateStatus;
use lightyear::prelude::TickManager;
use shared::bot::BotShip;
use shared::ships::get_shared_ship_components;

pub(crate) struct BotPlugin;

impl Plugin for BotPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(add_bot_collider);
        // Debug bot position (we run this in Last to be after Interpolation)
        // app.add_systems(Last, debug_bot_position);
    }
}


/// Debug system to log the interpolated bot position
#[allow(dead_code)]
fn debug_bot_position(
    tick_manager: Res<TickManager>,
    query: Query<(&Position, &InterpolateStatus<Position>), (With<Interpolated>, With<BotShip>)>,
) {
    let tick = tick_manager.tick();
    query.iter().for_each(|(pos, interpolate_status)| {
        info!(?tick, ?pos, ?interpolate_status, "Bot position");
    });
}

/// When an interpolated bot is spawned, we add a collider to it so we can visually
/// find collisions between bullets and bots
fn add_bot_collider(
    trigger: Trigger<OnAdd, BotShip>,
    mut commands: Commands,
    query: Query<(), With<Interpolated>>,
) {
    if query.get(trigger.entity()).is_ok() {
        commands.entity(trigger.entity()).insert(
            get_shared_ship_components(Collider::sphere(0.5))
        );
    }
}
