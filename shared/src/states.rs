use bevy::prelude::States;

#[derive(Debug, Clone, Eq, PartialEq, Hash, States)]
pub enum AppState {
    None,
    SplashScreen,
    TitleScreen,
    LoadingMap,
    Playing(PlayingSubState),
    ConnectingToServer,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States)]
pub enum PlayingSubState {
    Playing,
    Menu,
}