use bevy::prelude::*;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_hud);
    }
}


#[derive(Default, Component)]
struct Hud {
    pub health: i32,
    pub red_key: bool,
    pub blue_key: bool,
    pub yellow_key: bool,
}

#[derive(Component, Debug)]
struct Crosshair {
    /// List of available crosshairs
    textures: Vec<Handle<Image>>,
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
                // take up entire screen
                position_type: PositionType::Relative,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            }
        )).with_children(|parent| {
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
                left: Val::Percent(50.0),
                bottom: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-32.0),
                    bottom: Val::Px(-32.0),
                    ..default()
                },
                ..default()
            }
         ));
        });
}