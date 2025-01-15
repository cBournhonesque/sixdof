use bevy::asset::AsyncReadExt;
use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext, LoadedFolder},
    prelude::*,
    utils::{thiserror::Error, BoxedFuture},
};
use mlua::prelude::*;

pub mod conversions;
use conversions::*;

use crate::components::Health;
use crate::ids::IdPooler;
use crate::monsters::{Monster, MonsterKind};
use crate::pickups::PickupKind;
use crate::player::{LocalPlayer, Player};
use crate::spawn::{SpawnEvent, SpawnMonster, SpawnPickup};

pub struct ScriptPlugin;

pub struct ScriptContainer {
    engine: Lua,
    script_folder_handle: Option<Handle<LoadedFolder>>,
    receiver: std::sync::mpsc::Receiver<ScriptEvent>,
}

impl ScriptContainer {
    pub fn on_enter_playing_state(&mut self, map: String, is_server: bool) {
        if let Err(e) = self
            .engine
            .globals()
            .call_function::<_, ()>("OnEnterPlayingState", (map, is_server))
        {
            error! {"Error calling function: {}", e};
        }
    }

    pub fn on_player_fragged_player(&mut self, instigator: &ScriptPlayer, victim: &ScriptPlayer) {
        if let Err(e) = self.engine.globals().call_function::<_, ()>(
            "OnPlayerFraggedPlayer",
            (instigator.clone(), victim.clone()),
        ) {
            error! {"Error calling function: {}", e};
        }
    }

    pub fn on_player_fragged_monster(
        &mut self,
        instigator: &ScriptPlayer,
        monster: &ScriptMonster,
    ) {
        if let Err(e) = self.engine.globals().call_function::<_, ()>(
            "OnPlayerFraggedMonster",
            (instigator.clone(), monster.clone()),
        ) {
            error! {"Error calling function: {}", e};
        }
    }

    pub fn on_monster_fragged_player(&mut self, instigator: &ScriptMonster, victim: &ScriptPlayer) {
        if let Err(e) = self.engine.globals().call_function::<_, ()>(
            "OnMonsterFraggedPlayer",
            (instigator.clone(), victim.clone()),
        ) {
            error! {"Error calling function: {}", e};
        }
    }

    pub fn on_monster_fragged_monster(
        &mut self,
        instigator: &ScriptMonster,
        victim: &ScriptMonster,
    ) {
        if let Err(e) = self.engine.globals().call_function::<_, ()>(
            "OnMonsterFraggedMonster",
            (instigator.clone(), victim.clone()),
        ) {
            error! {"Error calling function: {}", e};
        }
    }
}

#[derive(Asset, TypePath)]
struct ScriptAsset {
    script: String,
}

#[derive(Debug, Error)]
pub enum ScriptAssetLoaderError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Default)]
struct ScriptAssetLoader;

impl AssetLoader for ScriptAssetLoader {
    type Asset = ScriptAsset;
    type Settings = ();
    type Error = ScriptAssetLoaderError;
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::<u8>::new();
            reader.read_to_end(&mut bytes).await?;
            if let Ok(script) = String::from_utf8(bytes) {
                Ok(ScriptAsset { script })
            } else {
                Err(ScriptAssetLoaderError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid UTF-8",
                )))
            }
        })
    }

    fn extensions(&self) -> &[&str] {
        &["lua"]
    }
}

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut App) {
        let (sender, receiver) = std::sync::mpsc::channel();
        let engine = Lua::new();

        let sender_clone = sender.clone();
        match engine.create_function(move |_, (amount, translation): (i16, ScriptVec3)| {
            let _ = sender_clone.send(ScriptEvent::SpawnHealth(amount, translation.into()));
            Ok(())
        }) {
            Ok(func) => {
                engine.globals().set("SpawnHealth", func).unwrap();
            }
            Err(e) => {
                error! {"Error creating function: {}", e};
            }
        }

        let sender_clone = sender.clone();
        match engine.create_function(move |_, player: ScriptPlayer| {
            let _ = sender_clone.send(ScriptEvent::UpdatePlayer(player));
            Ok(())
        }) {
            Ok(func) => {
                engine.globals().set("UpdatePlayer", func).unwrap();
            }
            Err(e) => {
                error! {"Error creating function: {}", e};
            }
        }

        let sender_clone = sender.clone();
        match engine.create_function(move |_, monster: ScriptMonster| {
            let _ = sender_clone.send(ScriptEvent::UpdateMonster(monster));
            Ok(())
        }) {
            Ok(func) => {
                engine.globals().set("UpdateMonster", func).unwrap();
            }
            Err(e) => {
                error! {"Error creating function: {}", e};
            }
        }

        let sender_clone = sender.clone();
        match engine.create_function(
            move |_, (spawn_monster, translation): (ScriptSpawnMonster, ScriptVec3)| {
                let _ =
                    sender_clone.send(ScriptEvent::SpawnMonster(spawn_monster, translation.into()));
                Ok(())
            },
        ) {
            Ok(func) => {
                engine.globals().set("SpawnMonster", func).unwrap();
            }
            Err(e) => {
                error! {"Error creating function: {}", e};
            }
        }

        app.insert_non_send_resource(ScriptContainer {
            engine,
            script_folder_handle: None,
            receiver,
        });

        app.init_asset::<ScriptAsset>();
        app.init_asset_loader::<ScriptAssetLoader>();
        app.add_systems(Startup, setup_system);
        app.add_systems(FixedUpdate, asset_event_system);
        app.add_systems(Update, receive_system);
        app.add_event::<ScriptEvent>();
    }
}

fn setup_system(mut container: NonSendMut<ScriptContainer>, asset_server: Res<AssetServer>) {
    container.script_folder_handle = Some(asset_server.load_folder("scripts"));
}

///
/// Receives events from the script container
///
fn receive_system(
    container: NonSend<ScriptContainer>,
    local_player: Option<Res<LocalPlayer>>,
    mut idpooler: Option<ResMut<IdPooler>>,
    mut spawn_events: EventWriter<SpawnEvent>,
    mut players: Query<(&mut Transform, &mut Health, &mut Player)>,
    mut monsters: Query<(&mut Transform, &mut Health, &mut Monster), Without<Player>>,
) {
    for event in container.receiver.try_iter() {
        match event {
            ScriptEvent::SpawnHealth(amount, translation) => {
                if let Some(local_player) = &local_player {
                    if local_player.has_authority() {
                        if let Some(idpooler) = idpooler.as_mut() {
                            if let Ok(pickup_id) = idpooler.assign_and_reserve_pickup_id() {
                                spawn_events.send(SpawnEvent::Pickup(SpawnPickup {
                                    id: pickup_id,
                                    kind: PickupKind::Health,
                                    amount,
                                    translation,
                                }));
                            }
                        }
                    }
                }
            }
            ScriptEvent::UpdatePlayer(script_player) => {
                if let Some(local_player) = &local_player {
                    if local_player.has_authority() {
                        for (mut transform, mut health, mut player) in players.iter_mut() {
                            if script_player.id == player.id {
                                script_player.update_components(
                                    &mut transform,
                                    &mut health,
                                    &mut player,
                                );
                            }
                        }
                    }
                }
            }
            ScriptEvent::UpdateMonster(script_monster) => {
                if let Some(local_player) = &local_player {
                    if local_player.has_authority() {
                        for (mut transform, mut health, mut monster) in monsters.iter_mut() {
                            if monster.id == monster.id {
                                script_monster.update_components(
                                    &mut transform,
                                    &mut health,
                                    &mut monster,
                                );
                            }
                        }
                    }
                }
            }
            ScriptEvent::SpawnMonster(script_spawn_monster, translation) => {
                if let Some(local_player) = &local_player {
                    if local_player.has_authority() {
                        if let Some(idpooler) = idpooler.as_mut() {
                            if let Ok(monster_id) = idpooler.assign_and_reserve_monster_id() {
                                spawn_events.send(SpawnEvent::Monster(SpawnMonster {
                                    id: monster_id,
                                    seed: monster_id,
                                    kind: script_spawn_monster.kind,
                                    translation: translation,
                                }));
                            }
                        }
                    }
                }
            }
        }
    }

    // container.on_player_fragged_player(&ScriptPlayer::default(), &ScriptPlayer::default());
}

fn asset_event_system(
    container: NonSend<ScriptContainer>,
    assets: ResMut<Assets<ScriptAsset>>,
    mut asset_events: EventReader<AssetEvent<ScriptAsset>>,
) {
    for event in asset_events.read() {
        match event {
            AssetEvent::Added { id } => {
                if let Some(asset) = assets.get(*id) {
                    if let Err(e) = container.engine.load(&asset.script).exec() {
                        error! {"Error loading script: {}", e};
                    }

                    if let Err(e) = container
                        .engine
                        .globals()
                        .call_function::<_, ()>("OnLoad", ())
                    {
                        error! {"Error calling function: {}", e};
                    }
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone, Event, Debug)]
pub enum ScriptEvent {
    SpawnHealth(i16, Vec3),
    SpawnMonster(ScriptSpawnMonster, Vec3),
    UpdatePlayer(ScriptPlayer),
    UpdateMonster(ScriptMonster),
}
