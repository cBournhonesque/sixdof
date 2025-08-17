use bevy::prelude::*;
use leafwing_input_manager::Actionlike;
use lightyear::prelude::*;
use avian3d::prelude::*;
use lightyear::prelude::input::{leafwing, InputConfig};
use crate::player::{PlayerRespawnTimer, PlayerShip};
use crate::prelude::{Damageable, Projectile, UniqueIdentity, WeaponFiredEvent};
use crate::ships::Ship;
use crate::weapons::{CurrentWeaponIndex, WeaponInventory};
use serde::{Deserialize, Serialize};
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
        }).add_direction(NetworkDirection::ServerToClient);

        // Inputs
        app.add_plugins(leafwing::InputPlugin::<PlayerInput> {
            config: InputConfig::<PlayerInput> {
                // enable lag compensation for player inputs
                lag_compensation: true,
                ..default()
            }
        });

        // Messages
        app.add_message::<WeaponFiredEvent>()
            .add_direction(NetworkDirection::ServerToClient)
            .add_map_entities();

        // Components
        app.register_component::<Name>()
            .add_prediction(PredictionMode::Once)
            .add_interpolation(InterpolationMode::Once);

        app.register_component::<Projectile>()
            .add_interpolation(InterpolationMode::Once);
        
        app.register_component::<PlayerShip>()
            .add_prediction(PredictionMode::Simple)
            .add_interpolation(InterpolationMode::Simple);
        
        app.register_component::<Ship>()
            .add_prediction(PredictionMode::Once);

        app.register_component::<BotShip>()
            .add_interpolation(InterpolationMode::Once);

        app.register_component::<PlayerRespawnTimer>();
        
        // Fully replicated, but not visual, so no need for lerp/corrections:
        app.register_component::<LinearVelocity>()
            .add_prediction(PredictionMode::Full);

        app.register_component::<AngularVelocity>()
            .add_prediction(PredictionMode::Full);

        app.register_component::<ExternalForce>()
            .add_prediction(PredictionMode::Full);

        app.register_component::<ExternalImpulse>()
            .add_prediction(PredictionMode::Full);

        app.register_component::<WeaponInventory>()
            .add_prediction(PredictionMode::Once);

        app.register_component::<Position>()
            .add_prediction(PredictionMode::Full)
            .add_linear_interpolation_fn()
            .add_interpolation(InterpolationMode::Full)
            .add_linear_correction_fn();

        app.register_component::<Rotation>()
            .add_prediction(PredictionMode::Full)
            .add_linear_interpolation_fn()
            .add_interpolation(InterpolationMode::Full)
            .add_linear_correction_fn();

        app.register_component::<UniqueIdentity>();
        app.register_component::<Damageable>();
        app.register_component::<CurrentWeaponIndex>()
            .add_prediction(PredictionMode::Full);
    }
}
