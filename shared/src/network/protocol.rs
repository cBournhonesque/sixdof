use bevy::prelude::*;
use leafwing_input_manager::Actionlike;
use lightyear::prelude::*;
use avian3d::prelude::*;
use lightyear::prelude::client::{ComponentSyncMode, LeafwingInputConfig, LerpFn};
use lightyear::utils::avian3d::{position, rotation};
use crate::player::Player;
use crate::weapons::WeaponInventory;
use lightyear::utils::bevy::TransformLinearInterpolation;
use crate::bot::Bot;

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
        app.add_plugins(LeafwingInputPlugin::<PlayerInput> {
            config: LeafwingInputConfig::<PlayerInput> {
                // enable lag compensation for player inputs
                lag_compensation: true,
                ..default()
            }
        });

        // Messages
        // TODO: MapLoad, RespawnCounter,

        // Components
        app.register_component::<Name>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Once)
            .add_interpolation(ComponentSyncMode::Once);
        app.register_component::<Player>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Simple)
            .add_interpolation(ComponentSyncMode::Simple);
        app.register_component::<Bot>(ChannelDirection::ServerToClient)
            .add_interpolation(ComponentSyncMode::Once);

        // Fully replicated, but not visual, so no need for lerp/corrections:
        app.register_component::<LinearVelocity>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        app.register_component::<AngularVelocity>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        app.register_component::<ExternalForce>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        app.register_component::<ExternalImpulse>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        // app.register_component::<Transform>(ChannelDirection::ServerToClient)
        //     .add_prediction(ComponentSyncMode::Full)
        //     .add_interpolation_fn(<TransformLinearInterpolation as LerpFn<Transform>>::lerp);
        //     // .add_correction_fn(<TransformLinearInterpolation as LerpFn<Transform>>::lerp);

        // Position and Rotation have a `correction_fn` set, which is used to smear rollback errors
        // over a few frames, just for the rendering part in postudpate.
        //
        // They also set `interpolation_fn` which is used by the VisualInterpolationPlugin to smooth
        // out rendering between fixedupdate ticks.
        app.register_component::<Position>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full)
            .add_interpolation(ComponentSyncMode::Full)
            .add_interpolation_fn(position::lerp);
            // .add_correction_fn(position::lerp);

        app.register_component::<Rotation>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full)
            .add_interpolation(ComponentSyncMode::Full)
            .add_interpolation_fn(rotation::lerp);
            // .add_correction_fn(rotation::lerp);

            
        app.register_component::<WeaponInventory>(ChannelDirection::ServerToClient)
            // sync once to the Predicted entity
            .add_prediction(ComponentSyncMode::Once);
        
        // do not replicate Transform but make sure to register an interpolation function
        // for it so that we can do visual interpolation
        // (another option would be to replicate transform and not use Position/Rotation at all)
        // (we want to do visual interpolation on Transform because it's easier than doing it on Position/Rotation
        //  and remembering to apply a sync in PostUpdate)
        app.add_interpolation::<Transform>(ComponentSyncMode::None);
        app.add_interpolation_fn::<Transform>(TransformLinearInterpolation::lerp);

        // NOTE: we do not replicate Transform because the avian transform->position sync plugin causes inaccuracies
        // TODO: maybe applying a TransformPropagate system in PreUpdate after the VisualInterpolation reset
        //  would fix the problem

        // // Try replicating only Transform
        // app.register_component::<Transform>(ChannelDirection::ServerToClient)
        //     .add_prediction(ComponentSyncMode::Full)
        //     .add_interpolation(ComponentSyncMode::Full)
        //     .add_interpolation_fn(TransformLinearInterpolation::lerp);

    }
}