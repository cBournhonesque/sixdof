use bevy::prelude::*;
use leafwing_input_manager::Actionlike;
use lightyear::prelude::*;
use avian3d::prelude::*;
use lightyear::prelude::client::{ComponentSyncMode, LerpFn};
use lightyear::utils::avian3d::{position, rotation};
use crate::player::{PlayerRespawnTimer, PlayerShip};
use crate::prelude::{Damageable, Projectile, UniqueIdentity, WeaponFiredEvent};
use crate::ships::Ship;
use crate::weapons::{CurrentWeaponIndex, WeaponInventory};
use lightyear::utils::bevy::TransformLinearInterpolation;
use crate::bot::BotShip;

/// Networking model:
/// - client is predicted
/// - other players are interpolated
/// - we use lag compensation for hit detection of bullets
/// - bullets will be
///   - pre-spawned on the client
///   - initial-replicated on the server (predicted on the client who fired the bullet, interpolated for other clients)
///     - we stop sending any replication updates after the first one
pub struct ProtocolPlugin;


#[derive(Channel)]
pub struct WeaponFiredChannel;

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
    RollLeft,
    RollRight,
    ShootPrimary,
    AfterBurners,
    NextWeapon,
    PreviousWeapon,
    Weapon1,
    Weapon2,
    Weapon3,
    Weapon4,
    Weapon5,
    ToggleMousePointer,
}

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // Channels
        app.add_channel::<WeaponFiredChannel>(ChannelSettings {
            mode: ChannelMode::UnorderedReliable(ReliableSettings::default()),
            ..default()
        });

        // Inputs
        app.add_plugins(LeafwingInputPlugin::<PlayerInput> {
            config: InputConfig::<PlayerInput> {
                // enable lag compensation for player inputs
                lag_compensation: true,
                ..default()
            }
        });

        // Messages
        app.register_message::<WeaponFiredEvent>(ChannelDirection::ServerToClient)
            .add_map_entities();

        // Components
        app.register_component::<Name>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Once)
            .add_interpolation(ComponentSyncMode::Once);

        app.register_component::<Projectile>(ChannelDirection::ServerToClient)
            .add_interpolation(ComponentSyncMode::Once);
        
        app.register_component::<PlayerShip>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Simple)
            .add_interpolation(ComponentSyncMode::Simple);
        
        app.register_component::<Ship>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Once);

        app.register_component::<BotShip>(ChannelDirection::ServerToClient)
            .add_interpolation(ComponentSyncMode::Once);

        app.register_component::<PlayerRespawnTimer>(ChannelDirection::ServerToClient);
        
        // Fully replicated, but not visual, so no need for lerp/corrections:
        app.register_component::<LinearVelocity>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        app.register_component::<AngularVelocity>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        app.register_component::<ExternalForce>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        app.register_component::<ExternalImpulse>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);

        app.register_component::<WeaponInventory>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Once);

        app.register_component::<Position>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full)
            .add_interpolation_fn(position::lerp)
            .add_interpolation(ComponentSyncMode::Full)
            .add_correction_fn(position::lerp);

        app.register_component::<Rotation>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full)
            .add_interpolation_fn(rotation::lerp)
            .add_interpolation(ComponentSyncMode::Full)
            .add_correction_fn(rotation::lerp);

        // do not replicate Transform but make sure to register an interpolation function
        // for it so that we can do visual interpolation
        app.add_interpolation::<Transform>(ComponentSyncMode::None);
        app.add_interpolation_fn::<Transform>(TransformLinearInterpolation::lerp);

        app.register_component::<UniqueIdentity>(ChannelDirection::ServerToClient);        
        app.register_component::<Damageable>(ChannelDirection::ServerToClient);
        app.register_component::<CurrentWeaponIndex>(ChannelDirection::ServerToClient)
            .add_prediction(ComponentSyncMode::Full);
    }
}
