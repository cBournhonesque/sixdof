use std::ops::DerefMut;
use bevy::utils::Duration;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::time::Stopwatch;
use lightyear::prelude::*;
use lightyear::prelude::server::*;
use lightyear_avian::prelude::LagCompensationHistory;
use shared::bot::Bot;
use shared::prelude::{Damageable, GameLayer, UniqueIdentity};
use shared::ships::{get_shared_ship_components, move_ship, ShipIndex, ShipsData};
// TODO: should bots be handled similarly to players? i.e. they share most of the same code (visuals, collisions)
//  but they are simply controlled by the server. The server could be sending fake inputs to the bots so that their movement
//  is the same as players
//  For now i'm just using them to debug lag compensation

pub(crate) struct BotPlugin;
impl Plugin for BotPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BotManager { next_bot_id: 0 });
        app.add_systems(Startup, spawn_bot);
        app.add_systems(FixedUpdate, move_bot.run_if(resource_exists::<ShipsData>));
    }
}   

#[derive(Resource)]
struct BotManager {
    next_bot_id: u32,
}

fn spawn_bot(mut commands: Commands, mut bot_manager: ResMut<BotManager>) {
    // TODO: use spawn-events so we can control spawn position, etc.
    let position = Position(Vec3::new(1.0, 4.0, -1.0));
    let rotation = Rotation(Quat::from_rotation_arc(Vec3::Y, Vec3::NEG_Z));
    commands.spawn(
        (
            Name::from("Bot"),
            Replicate {
                sync: SyncTarget {
                    interpolation: NetworkTarget::All,
                    ..default()
                },
                // in case the renderer is enabled on the server, we don't want the visuals to be replicated!
                hierarchy: ReplicateHierarchy {
                    enabled: false,
                    recursive: false,
                },
                // TODO: all predicted entities must be part of the same replication group
                ..default()
            },
            UniqueIdentity::Bot(bot_manager.next_bot_id),
            Bot,
            Damageable {
                health: 50,
            },
            ShipIndex(1),
            // TODO: UNDERSTAND WHY IT IS NECESSARY TO MANUALLY INSERT THE CORRECT POSITION/ROTATION
            //  ON THE ENTITY! I THOUGHT THE PREPARE_SET WOULD DO THIS AUTOMATICALLY
            position,
            rotation,
            get_shared_ship_components(Collider::sphere(0.5))
            //LagCompensationHistory::default(),
        )
    );
    bot_manager.next_bot_id += 1;
}

/// Move bots up and down
/// For some reason we cannot use the TimeManager.delta() here, maybe because we're running in FixedUpdate?
fn move_bot(
    tick_manager: Res<TickManager>,
    fixed_time: Res<Time<Fixed>>, 
    mut query: Query<(
        &mut LinearVelocity, 
        &mut AngularVelocity, 
        &ShipIndex,
    ), With<Bot>>, 
    ships_data: Res<ShipsData>,
    mut timer: Local<(Stopwatch, bool)>)
{
    let tick = tick_manager.tick();
    let (stopwatch, go_up) = timer.deref_mut();
    for (mut linear_velocity, mut angular_velocity, ship_index) in query.iter_mut() {
        stopwatch.tick(fixed_time.delta());
        if stopwatch.elapsed() > Duration::from_secs_f32(4.0) {
            stopwatch.reset();
            *go_up = !*go_up;
        }
        let wish_dir = if *go_up {
            Vec3::Y
        } else {
            Vec3::NEG_Y
        };
        if let Some(data) = ships_data.ships.get(&ship_index.0) {
            move_ship(&fixed_time, &data, &mut linear_velocity, &mut angular_velocity, wish_dir, None);
        }
        trace!(?tick, ?linear_velocity, ?angular_velocity, "Bot velocity");
    }
}
