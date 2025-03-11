use std::ops::DerefMut;
use bevy::utils::Duration;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::time::Stopwatch;
use lightyear::prelude::*;
use lightyear::prelude::server::*;
use lightyear_avian::prelude::LagCompensationHistory;
use shared::bot::Bot;
use shared::prelude::{Damageable, GameLayer, MOVPosition, MOVRotation, Moveable, MoveableShape, UniqueIdentity};
// TODO: should bots be handled similarly to players? i.e. they share most of the same code (visuals, collisions)
//  but they are simply controlled by the server. The server could be sending fake inputs to the bots so that their movement
//  is the same as players
//  For now i'm just using them to debug lag compensation

pub(crate) struct BotPlugin;
impl Plugin for BotPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BotManager { next_bot_id: 0 });
        app.add_systems(Startup, spawn_bot);
        app.add_systems(FixedUpdate, (
            move_bot,
        ));
    }
}

#[derive(Resource)]
struct BotManager {
    next_bot_id: u32,
}

fn spawn_bot(mut commands: Commands, mut bot_manager: ResMut<BotManager>) {
    // TODO: use spawn-events so we can control spawn position, etc.
    let spawn_position = Vec3::new(1.0, 3.5, -1.0);
    commands.spawn(
        (
            Name::from("Bot"),
            Replicate {
                sync: SyncTarget {
                    prediction: NetworkTarget::None,
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
            MOVPosition(spawn_position),
            MOVRotation(Quat::IDENTITY),
            Moveable {
                collision_shape: MoveableShape::Sphere(0.5),
                collision_mask: [GameLayer::Player, GameLayer::Wall].into(),
            },
            Transform::from_translation(spawn_position),
            //LagCompensationHistory::default(),
        )
    );
    bot_manager.next_bot_id += 1;
}

/// Move bots up and down
/// For some reason we cannot use the TimeManager.delta() here, maybe because we're running in FixedUpdate?
fn move_bot(
    tick_manager: Res<TickManager>,
    time: Res<Time>, mut query: Query<&mut MOVPosition, With<Bot>>, mut timer: Local<(Stopwatch, bool)>)
{
    let tick = tick_manager.tick();
    let (stopwatch, go_up) = timer.deref_mut();
    query.iter_mut().for_each(|mut position| {
        stopwatch.tick(time.delta());
        if stopwatch.elapsed() > Duration::from_secs_f32(4.0) {
            stopwatch.reset();
            *go_up = !*go_up;
        }
        if *go_up {
            position.0.y += 0.02;
        } else {
            position.0.y -= 0.02;
        }
        trace!(?tick, ?position, "Bot position");
    });
}
