use bevy::prelude::*;
use leafwing_input_manager::{Actionlike, InputControlKind};
use lightyear::prelude::*;
use avian3d::prelude::*;
use lightyear::prelude::client::ComponentSyncMode;
use lightyear::utils::avian3d::{position, rotation};
use crate::player::Player;

pub struct ProtocolPlugin;


#[derive(Channel)]
pub struct Channel1;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, Hash, Reflect, Actionlike)]
pub enum PlayerInput {
    #[actionlike(DualAxis)]
    Look,
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveRollLeft,
    MoveRollRight,
    ShootPrimary,
    ShootSecondary,
    Weapon1,
    Weapon2,
    Weapon3,
    Weapon4,
    Weapon5
}

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // Channels
        app.add_channel::<Channel1>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        });

        // Inputs
        app.add_plugins(LeafwingInputPlugin::<PlayerInput>::default());

        // Messages
        // TODO: MapLoad, RespawnCounter,

        // Components
        app.register_component::<Name>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Once)
            .add_interpolation(ComponentSyncMode::Once);
        app.register_component::<Player>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Simple)
            .add_interpolation(ComponentSyncMode::Simple);

        // Fully replicated, but not visual, so no need for lerp/corrections:
        app.register_component::<LinearVelocity>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        app.register_component::<AngularVelocity>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        app.register_component::<ExternalForce>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        app.register_component::<ExternalImpulse>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        app.register_component::<Transform>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        app.register_component::<ComputedMass>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        // Position and Rotation have a `correction_fn` set, which is used to smear rollback errors
        // over a few frames, just for the rendering part in postudpate.
        //
        // They also set `interpolation_fn` which is used by the VisualInterpolationPlugin to smooth
        // out rendering between fixedupdate ticks.
        app.register_component::<Position>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full)
            .add_interpolation_fn(position::lerp)
            .add_correction_fn(position::lerp);

        app.register_component::<Rotation>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full)
            .add_interpolation_fn(rotation::lerp)
            .add_correction_fn(rotation::lerp);
    }
}