use bevy::{diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}, prelude::*};
use lightyear::client::prediction::diagnostics::PredictionMetrics;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_hud);
        app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        app.add_systems(Update, prediction_metrics_system);
    }
}

#[derive(Default, Component)]
struct Hud {
    pub health: i32,
    pub red_key: bool,
    pub blue_key: bool,
    pub yellow_key: bool,
}

#[derive(Component)]
struct PredictionMetricsText;

#[derive(Component, Debug)]
struct Crosshair {
    /// List of available crosshairs
    textures: Vec<Handle<Image>>,
}

fn prediction_metrics_system(
    diagnostics: Res<DiagnosticsStore>,
    prediction_metrics: Option<Res<PredictionMetrics>>,
    mut text_query: Query<&mut Text, With<PredictionMetricsText>>,
) {
    if let Some(prediction_metrics) = prediction_metrics {
        if let Ok(mut text) = text_query.get_single_mut() {
            if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
                text.0 = format!("FPS: {}\nRollbacks: {}\nRollback Ticks: {}",
                    fps.smoothed().unwrap_or(0.0).round(),
                    prediction_metrics.rollbacks,
                    prediction_metrics.rollback_ticks
                );
            } else {
                text.0 = format!("Rollbacks: {}\nRollback Ticks: {}",
                    prediction_metrics.rollbacks,
                    prediction_metrics.rollback_ticks
                );
            }
        }
    }
}

fn spawn_hud(
    trigger: Trigger<OnAdd, Camera3d>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let default_crosshair = asset_server.load("crosshairs/kenney_crosshair_pack/crosshair019.png");
    commands
        .spawn((
            Hud {
                health: 100,
                red_key: false,
                blue_key: false,
                yellow_key: false,
            },
            Node {
                position_type: PositionType::Relative,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            }
        )).with_children(|parent| {
            // Prediction metrics text
            parent.spawn((
                PredictionMetricsText,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(10.0),
                    right: Val::Px(10.0),
                    ..default()
                },
                Text::new("Prediction metrics..."),
            ));

            // Crosshair
            parent.spawn((
                Crosshair {
                    textures: vec![
                        default_crosshair.clone(),
                        asset_server.load("crosshairs/kenney_crosshair_pack/crosshair188.png"),
                        asset_server.load("crosshairs/kenney_crosshair_pack/crosshair030.png"),
                        asset_server.load("crosshairs/kenney_crosshair_pack/crosshair043.png"),
                        asset_server.load("crosshairs/kenney_crosshair_pack/crosshair018.png"),
                    ],
                },
                ImageNode {
                    image: default_crosshair,
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(64.0),
                    height: Val::Px(64.0),
                    right: Val::Percent(50.0),
                    bottom: Val::Percent(50.0),
                    margin: UiRect {
                        right: Val::Px(-32.0),
                        bottom: Val::Px(-32.0),
                        ..default()
                    },
                    ..default()
                }
            ));
        });
}