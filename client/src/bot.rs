use avian3d::prelude::Position;
use bevy::prelude::*;
use lightyear::client::interpolation::Interpolated;
use lightyear::prelude::client::InterpolateStatus;
use lightyear::prelude::TickManager;
use shared::bot::Bot;

pub(crate) struct BotPlugin;

impl Plugin for BotPlugin {
    fn build(&self, app: &mut App) {
        // Debug bot position (we run this in Last to be after Interpolation)
        app.add_systems(Last, debug_bot_position);
    }
}


/// Debug system to log the interpolated bot position
fn debug_bot_position(
    tick: Res<TickManager>,
    query: Query<(&Position, &InterpolateStatus<Position>), (With<Interpolated>, With<Bot>)>,
) {
    let tick = tick.tick();
    query.iter().for_each(|(pos, interpolate_status)| {
        info!(?tick, ?pos, ?interpolate_status, "Bot position");
    });
}