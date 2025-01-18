use std::ops::DerefMut;
use std::time::Duration;
use avian3d::prelude::{Collider, Position, RigidBody};
use bevy::prelude::*;
use bevy::time::Stopwatch;
use lightyear::prelude::*;
use lightyear::prelude::server::*;
use shared::bot::Bot;
// TODO: should bots be handled similarly to players? i.e. they share most of the same code (visuals, collisions)
//  but they are simply controlled by the server. The server could be sending fake inputs to the bots so that their movement
//  is the same as players
//  For now i'm just using them to debug lag compensation

pub(crate) struct BotPlugin;
impl Plugin for BotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_bot);
        app.add_systems(FixedUpdate, move_bot);
    }
}

fn spawn_bot(mut commands: Commands) {
    // TODO: use spawn-events so we can control spawn position, etc.
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
            Bot,
            Transform::from_xyz(1.0, 4.0, -1.0),
            RigidBody::Kinematic,
            // TODO: contacts are not fully deterministic in avian!
            Collider::sphere(0.5),
        )
    );
}

/// Move bots up and down
/// For some reason we cannot use the TimeManager.delta() here, maybe because we're running in FixedUpdate?
fn move_bot(time: Res<Time>, mut query: Query<&mut Position, With<Bot>>, mut timer: Local<(Stopwatch, bool)>) {
    let (stopwatch, go_up) = timer.deref_mut();
    query.iter_mut().for_each(|mut position| {
        stopwatch.tick(time.delta());
        if stopwatch.elapsed() > Duration::from_secs_f32(2.0) {
            stopwatch.reset();
            *go_up = !*go_up;
        }
        if *go_up {
            position.y += 0.1;
        } else {
            position.y -= 0.1;
        }
    });
}

