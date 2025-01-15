use crate::components::Health;
use crate::components::Team;
use crate::monsters::*;
use crate::net::serialize::{QuantizedRotation, QuantizedVec3U16};
use crate::physics::MovementState;
use crate::pickups::{Pickup, PickupKind};
use crate::player::LocalPlayer;
use crate::player::Player;
use crate::spawn::SpawnEvent;
use bevy::prelude::*;
use byteorder::BigEndian;
use byteorder::ByteOrder;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;
use history::BubbledUpDeletions;
use history::SnapshotHistory;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::io::Cursor;
use std::io::Read;

pub mod history;

pub const TRANSFORM_QUANTIZE_RANGE: u32 = 1000;
pub const VELOCITY_QUANTIZE_RANGE: u32 = 500;
pub const TRANSLATION_EQ_TOLERANCE: f32 = 0.001;

pub fn snapshot_system(
    local_player: Res<LocalPlayer>,
    monsters_query: Query<(Entity, &Transform, &Monster)>,
    players_query: Query<
        (Entity, &Transform, &Team, &MovementState, &Player, &Health),
        Without<Monster>,
    >,
    pickups_query: Query<(&Transform, &Pickup)>,
    mut spawn_events: EventReader<SpawnEvent>,
    mut snapshot_history: ResMut<SnapshotHistory>,
) {
    if !local_player.has_authority() {
        return;
    }

    // let time_now = SystemTime::now();
    let mut player_deletions = Vec::new();
    let mut monster_deletions = Vec::new();
    let mut pickup_deletions = Vec::new();

    for event in spawn_events.read() {
        match event {
            SpawnEvent::DespawnPlayer(player_id) => {
                player_deletions.push(*player_id);
            }
            SpawnEvent::DespawnMonster(monster_id) => {
                monster_deletions.push(*monster_id);
            }
            SpawnEvent::DespawnPickup(pickup_id) => {
                pickup_deletions.push(*pickup_id);
            }
            _ => {}
        }
    }

    let snapshot = Snapshot {
        id: crate::utils::timestamp_millis_since_epoch(),
        absolute: true,
        players: {
            let mut player_snapshots = Vec::new();
            for (entity, transform, team, movement_state, player, health) in players_query.iter() {
                player_snapshots.push(PlayerSnapshot::from_player(
                    entity,
                    player,
                    team,
                    health,
                    transform,
                    movement_state,
                ));
            }
            player_snapshots
        },
        monsters: {
            let mut monster_snapshots = Vec::new();
            for (entity, transform, monster) in monsters_query.iter() {
                monster_snapshots.push(MonsterSnapshot::from_monster(entity, monster, transform));
            }
            monster_snapshots
        },
        pickups: {
            let mut pickup_snapshots = Vec::new();
            for (transform, pickup) in pickups_query.iter() {
                pickup_snapshots.push(PickupSnapshot::from_pickup(pickup, transform));
            }
            pickup_snapshots
        },
        local_player_id: None,
        local_last_processed_input_id: None,
        player_deletions,
        monster_deletions,
        pickup_deletions,
    };

    // delete snapshots older than a second ago
    snapshot_history.clean_old_snapshots(&snapshot);

    // log the snapshot in the history
    snapshot_history
        .server_snapshots
        .insert(snapshot.id, snapshot.clone());
}

pub trait SnapshotTrait {
    type Value;
    fn id_u64(&self) -> u64;
    fn diff(&self, old: &Self) -> Self;
    fn is_empty(&self) -> bool;
    fn read_from_stream<E>(cursor: &mut Cursor<&[u8]>) -> Result<Self::Value, E>
    where
        E: serde::de::Error;
    fn write_to_stream(&self, bytes: &mut Vec<u8>);
    fn diff_nested<T>(new: &Vec<T>, old: &Vec<T>) -> Vec<T>
    where
        T: SnapshotTrait + Clone,
    {
        let mut out = Vec::new();
        for new_item in new {
            if let Some(old_item) = old
                .iter()
                .find(|old_item| old_item.id_u64() == new_item.id_u64())
            {
                let diff = new_item.diff(old_item);
                if !diff.is_empty() {
                    out.push(diff);
                }
            } else {
                out.push(new_item.clone());
            }
        }
        out
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Snapshot {
    pub id: u64, // id of the snapshot is a unix timestamp in ms
    pub absolute: bool,
    pub players: Vec<PlayerSnapshot>,
    pub monsters: Vec<MonsterSnapshot>,
    pub pickups: Vec<PickupSnapshot>,
    pub local_player_id: Option<u8>,
    pub local_last_processed_input_id: Option<u64>,
    pub player_deletions: Vec<u8>,
    pub monster_deletions: Vec<u8>,
    pub pickup_deletions: Vec<u8>,
}

impl Snapshot {
    pub fn diff_snapshots(
        new: &Snapshot,
        old: &Snapshot,
        deletions: &BubbledUpDeletions,
    ) -> Snapshot {
        let mut out = new.diff(old);
        out.player_deletions = deletions.player_deletions.clone();
        out.monster_deletions = deletions.monster_deletions.clone();
        out.pickup_deletions = deletions.pickup_deletions.clone();
        out
    }

    pub fn interpolate_players(
        ignore_player_id: u8,
        from: &Snapshot,
        to: &Snapshot,
        alpha: f32,
    ) -> Snapshot {
        let mut interpolated_snapshot = from.clone();
        for player_snapshot in interpolated_snapshot.players.iter_mut() {
            if player_snapshot.id == ignore_player_id {
                continue;
            }

            if let Some(to_player_snapshot) = to
                .players
                .iter()
                .find(|to_player_snapshot| to_player_snapshot.id == player_snapshot.id)
            {
                if let Some(from_translation) = player_snapshot.translation {
                    if let Some(to_translation) = to_player_snapshot.translation {
                        player_snapshot.translation =
                            Some(from_translation.lerp(to_translation, alpha));
                    }
                }

                if let Some(mut from_rotation) = player_snapshot.rotation {
                    if let Some(to_rotation) = to_player_snapshot.rotation {
                        player_snapshot.rotation = Some(from_rotation.slerp(&to_rotation, alpha));
                    }
                }
            }
        }
        interpolated_snapshot
    }

    pub fn interpolate_pickups(from: &Snapshot, to: &Snapshot, alpha: f32) -> Snapshot {
        let mut interpolated_snapshot = from.clone();
        for pickup_snapshot in interpolated_snapshot.pickups.iter_mut() {
            if let Some(to_pickup_snapshot) = to
                .pickups
                .iter()
                .find(|to_pickup_snapshot| to_pickup_snapshot.id == pickup_snapshot.id)
            {
                if let Some(mut from_translation) = pickup_snapshot.translation {
                    if let Some(to_translation) = to_pickup_snapshot.translation {
                        pickup_snapshot.translation =
                            Some(from_translation.lerp(&to_translation, alpha));
                    }
                }
            }
        }
        interpolated_snapshot
    }
}

impl SnapshotTrait for Snapshot {
    type Value = Snapshot;

    fn id_u64(&self) -> u64 {
        self.id
    }

    fn diff(&self, old: &Self) -> Self {
        Self {
            id: self.id,
            absolute: false,
            players: Self::diff_nested(&self.players, &old.players),
            monsters: Self::diff_nested(&self.monsters, &old.monsters),
            pickups: Self::diff_nested(&self.pickups, &old.pickups),
            local_player_id: if old.local_player_id != self.local_player_id {
                self.local_player_id
            } else {
                None
            },
            local_last_processed_input_id: if old.local_last_processed_input_id
                != self.local_last_processed_input_id
            {
                self.local_last_processed_input_id
            } else {
                None
            },
            player_deletions: Vec::new(),
            monster_deletions: Vec::new(),
            pickup_deletions: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        false
    }

    fn write_to_stream(&self, bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(&self.id.to_be_bytes());

        let mut head: u16 = 0;

        if self.absolute {
            head |= 0b00000000_00000001;
        }

        if self.players.len() > 0 {
            head |= 0b00000000_00000010;
        }

        if self.monsters.len() > 0 {
            head |= 0b00000000_00000100;
        }

        if self.pickups.len() > 0 {
            head |= 0b00000000_00001000;
        }

        if self.local_player_id.is_some() {
            head |= 0b00000000_00010000;
        }

        if self.local_last_processed_input_id.is_some() {
            head |= 0b00000000_00100000;
        }

        if self.player_deletions.len() > 0 {
            head |= 0b00000000_01000000;
        }

        if self.monster_deletions.len() > 0 {
            head |= 0b00000000_10000000;
        }

        if self.pickup_deletions.len() > 0 {
            head |= 0b00000001_00000000;
        }

        bytes.extend_from_slice(&head.to_be_bytes());

        if self.players.len() > 0 {
            bytes.push(self.players.len() as u8);
            for player in &self.players {
                player.write_to_stream(bytes);
            }
        }

        if self.monsters.len() > 0 {
            bytes.push(self.monsters.len() as u8);
            for monster in &self.monsters {
                monster.write_to_stream(bytes);
            }
        }

        if self.pickups.len() > 0 {
            bytes.push(self.pickups.len() as u8);
            for pickup in &self.pickups {
                pickup.write_to_stream(bytes);
            }
        }

        if let Some(local_player_id) = self.local_player_id {
            bytes.push(local_player_id);
        }

        if let Some(local_last_processed_input_id) = self.local_last_processed_input_id {
            bytes.extend_from_slice(&local_last_processed_input_id.to_be_bytes());
        }

        if self.player_deletions.len() > 0 {
            bytes.push(self.player_deletions.len() as u8);
            for id in &self.player_deletions {
                bytes.push(*id);
            }
        }

        if self.monster_deletions.len() > 0 {
            bytes.push(self.monster_deletions.len() as u8);
            for id in &self.monster_deletions {
                bytes.push(*id);
            }
        }

        if self.pickup_deletions.len() > 0 {
            bytes.push(self.pickup_deletions.len() as u8);
            for id in &self.pickup_deletions {
                bytes.push(*id);
            }
        }
    }

    fn read_from_stream<E>(cursor: &mut Cursor<&[u8]>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let id = cursor
            .read_u64::<BigEndian>()
            .map_err(|_| E::custom("Error reading snapshot_id"))?;
        let head = cursor
            .read_u16::<BigEndian>()
            .map_err(|_| E::custom("Error reading snapshot_head"))?;
        Ok(Self {
            id: id,
            absolute: head & 0b00000000_00000001 != 0,
            players: {
                let mut players = Vec::<PlayerSnapshot>::new();
                if head & 0b00000000_00000010 != 0 {
                    let count = cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading player_count"))?;
                    for _ in 0..count {
                        if let Ok(player) = PlayerSnapshot::read_from_stream::<E>(cursor) {
                            players.push(player);
                        }
                    }
                }
                players
            },
            monsters: {
                let mut monsters = Vec::<MonsterSnapshot>::new();
                if head & 0b00000000_00000100 != 0 {
                    let count = cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading monster_count"))?;
                    for _ in 0..count {
                        if let Ok(monster) = MonsterSnapshot::read_from_stream::<E>(cursor) {
                            monsters.push(monster);
                        }
                    }
                }
                monsters
            },
            pickups: {
                let mut pickups = Vec::<PickupSnapshot>::new();
                if head & 0b00000000_00001000 != 0 {
                    let count = cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading pickup_count"))?;
                    for _ in 0..count {
                        if let Ok(pickup) = PickupSnapshot::read_from_stream::<E>(cursor) {
                            pickups.push(pickup);
                        }
                    }
                }
                pickups
            },
            local_player_id: if head & 0b00000000_00010000 != 0 {
                Some(
                    cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading local_player_id"))?,
                )
            } else {
                None
            },
            local_last_processed_input_id: if head & 0b00000000_00100000 != 0 {
                Some(
                    cursor
                        .read_u64::<BigEndian>()
                        .map_err(|_| E::custom("Error reading local_last_processed_input_id"))?,
                )
            } else {
                None
            },
            player_deletions: {
                let mut player_deletions = Vec::<u8>::new();
                if head & 0b00000000_01000000 != 0 {
                    let count = cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading player_deletions_count"))?;
                    for _ in 0..count {
                        player_deletions.push(
                            cursor
                                .read_u8()
                                .map_err(|_| E::custom("Error reading player_deletion"))?,
                        );
                    }
                }
                player_deletions
            },
            monster_deletions: {
                let mut monster_deletions = Vec::<u8>::new();
                if head & 0b00000000_10000000 != 0 {
                    let count = cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading monster_deletions_count"))?;
                    for _ in 0..count {
                        monster_deletions.push(
                            cursor
                                .read_u8()
                                .map_err(|_| E::custom("Error reading monster_deletion"))?,
                        );
                    }
                }
                monster_deletions
            },
            pickup_deletions: {
                let mut pickup_deletions = Vec::<u8>::new();
                if head & 0b00000001_00000000 != 0 {
                    let count = cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading pickup_deletions_count"))?;
                    for _ in 0..count {
                        pickup_deletions.push(
                            cursor
                                .read_u8()
                                .map_err(|_| E::custom("Error reading pickup_deletion"))?,
                        );
                    }
                }
                pickup_deletions
            },
        })
    }
}

impl Serialize for Snapshot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut bytes = Vec::new();
        self.write_to_stream(&mut bytes);
        serializer.serialize_bytes(&bytes)
    }
}

impl<'de> Deserialize<'de> for Snapshot {
    fn deserialize<D>(deserializer: D) -> Result<Snapshot, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SnapshotVisitor;

        impl<'de> Visitor<'de> for SnapshotVisitor {
            type Value = Snapshot;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a byte array")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let mut cursor = Cursor::new(v);
                Snapshot::read_from_stream(&mut cursor)
            }
        }

        deserializer.deserialize_bytes(SnapshotVisitor)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct PlayerSnapshot {
    pub id: u8,
    pub translation: Option<Vec3>,
    pub rotation: Option<QuantizedRotation>,
    pub velocity: Option<QuantizedVec3U16>,
    pub frozen_amount: Option<u8>,
    pub health: Option<i16>,
    pub frags: Option<u16>,
    pub deaths: Option<u16>,
    pub score: Option<u16>,
    pub name: Option<[u8; 32]>,
    pub team: Option<Team>,
    pub ping: Option<u8>,

    // authority only
    pub entity: Option<Entity>,
}

impl PlayerSnapshot {
    pub fn rotation(&self) -> Option<Quat> {
        self.rotation.map(|r| r.to_quat())
    }

    pub fn from_player(
        entity: Entity,
        player: &Player,
        team: &Team,
        health: &Health,
        transform: &Transform,
        movement_state: &MovementState,
    ) -> PlayerSnapshot {
        PlayerSnapshot {
            id: player.id,
            frozen_amount: Some(player.frozen_amount),
            translation: Some(transform.translation),
            rotation: Some(QuantizedRotation::from_quat(&transform.rotation)),
            velocity: Some(QuantizedVec3U16::from_vec3(
                &movement_state.velocity,
                VELOCITY_QUANTIZE_RANGE,
            )),
            health: Some(health.current()),
            name: Some(Self::encode_name(&player.name)),
            score: Some(player.score),
            frags: Some(player.frags),
            deaths: Some(player.deaths),
            ping: Some(player.ping),
            team: Some(*team),
            entity: Some(entity),
        }
    }

    pub fn update_player_transform(
        &self,
        update_rotation: bool,
        transform_component: &mut Transform,
        movement_state_component: &mut MovementState,
    ) {
        if let Some(translation) = self.translation {
            transform_component.translation = translation;
        }

        if update_rotation {
            if let Some(rotation) = self.rotation {
                transform_component.rotation = rotation.to_quat();
            }
        }

        if let Some(velocity) = self.velocity {
            movement_state_component.velocity = velocity.to_vec3(VELOCITY_QUANTIZE_RANGE);
        }
    }

    pub fn update_player(
        &self,
        player_component: &mut Player,
        team_component: &mut Team,
        health_component: &mut Health,
    ) {
        if let Some(health) = self.health {
            health_component.set_current(health);
        }

        if let Some(frozen_amount) = self.frozen_amount {
            player_component.frozen_amount = frozen_amount;
        }

        if let Some(name) = self.name {
            player_component.name = Self::decode_name(&name);
        }

        if let Some(score) = self.score {
            player_component.score = score;
        }

        if let Some(frags) = self.frags {
            player_component.frags = frags;
        }

        if let Some(deaths) = self.deaths {
            player_component.deaths = deaths;
        }

        if let Some(ping) = self.ping {
            player_component.ping = ping;
        }

        if let Some(team) = self.team {
            *team_component = team;
        }
    }

    pub fn decode_name(name: &[u8; 32]) -> String {
        String::from_utf8_lossy(name)
            .trim_matches(char::from(0))
            .to_string()
    }

    pub fn encode_name(name: &String) -> [u8; 32] {
        let mut encoded_name = [0; 32];
        let name_bytes = name.as_bytes();
        encoded_name[..name_bytes.len()].copy_from_slice(name_bytes);
        encoded_name
    }
}

impl SnapshotTrait for PlayerSnapshot {
    type Value = PlayerSnapshot;
    fn id_u64(&self) -> u64 {
        self.id as u64
    }
    fn diff(&self, old: &PlayerSnapshot) -> PlayerSnapshot {
        Self {
            id: self.id,
            translation: {
                if self.translation.is_some() {
                    if old.translation.is_some() {
                        if self.translation.unwrap().distance(old.translation.unwrap())
                            > TRANSLATION_EQ_TOLERANCE
                        {
                            self.translation
                        } else {
                            None
                        }
                    } else {
                        self.translation
                    }
                } else {
                    None
                }
            },
            rotation: if self.rotation != old.rotation {
                self.rotation
            } else {
                None
            },
            velocity: if self.velocity != old.velocity {
                self.velocity
            } else {
                None
            },
            health: if self.health != old.health {
                self.health
            } else {
                None
            },
            frozen_amount: if self.frozen_amount != old.frozen_amount {
                self.frozen_amount
            } else {
                None
            },
            frags: if self.frags != old.frags {
                self.frags
            } else {
                None
            },
            deaths: if self.deaths != old.deaths {
                self.deaths
            } else {
                None
            },
            score: if self.score != old.score {
                self.score
            } else {
                None
            },
            name: if self.name != old.name {
                self.name
            } else {
                None
            },
            team: if self.team != old.team {
                self.team
            } else {
                None
            },
            ping: if self.ping != old.ping {
                self.ping
            } else {
                None
            },
            entity: None,
        }
    }

    fn is_empty(&self) -> bool {
        self.translation.is_none()
            && self.rotation.is_none()
            && self.velocity.is_none()
            && self.health.is_none()
            && self.frozen_amount.is_none()
            && self.frags.is_none()
            && self.deaths.is_none()
            && self.score.is_none()
            && self.name.is_none()
            && self.team.is_none()
            && self.ping.is_none()
    }

    fn write_to_stream(&self, bytes: &mut Vec<u8>) {
        bytes.push(self.id);
        let mut head: u16 = 0;

        if self.translation.is_some() {
            head |= 0b00000000_00000001;
        }

        if self.rotation.is_some() {
            head |= 0b00000000_00000010;
        }

        if self.velocity.is_some() {
            head |= 0b00000000_00000100;
        }

        if self.frozen_amount.is_some() {
            head |= 0b00000000_00001000;
        }

        if self.health.is_some() {
            head |= 0b00000000_00010000;
        }

        if self.frags.is_some() {
            head |= 0b00000000_00100000;
        }

        if self.deaths.is_some() {
            head |= 0b00000000_01000000;
        }

        if self.score.is_some() {
            head |= 0b00000000_10000000;
        }

        if self.name.is_some() {
            head |= 0b00000001_00000000;
        }

        if self.team.is_some() {
            head |= 0b00000010_00000000;
        }

        if self.ping.is_some() {
            head |= 0b00000100_00000000;
        }

        bytes.extend_from_slice(&head.to_be_bytes());

        if let Some(translation) = self.translation {
            // we do not quantize player positions
            let mut translation_bytes = [0u8; 12];
            BigEndian::write_f32_into(
                &[translation.x, translation.y, translation.z],
                &mut translation_bytes,
            );
            bytes.extend_from_slice(&translation_bytes);
        }

        if let Some(rotation) = self.rotation {
            // quantize the rotation
            let rotation_bytes = [rotation.pitch, rotation.yaw, rotation.roll];
            bytes.extend_from_slice(&rotation_bytes);
        }

        if let Some(velocity) = self.velocity {
            // quantize the velocity
            let mut velocity_bytes = [0u8; 6];
            BigEndian::write_u16_into(&[velocity.x, velocity.y, velocity.z], &mut velocity_bytes);
            bytes.extend_from_slice(&velocity_bytes);
        }

        if let Some(health) = self.health {
            bytes.extend_from_slice(&health.to_be_bytes());
        }

        if let Some(frozen_amount) = self.frozen_amount {
            bytes.push(frozen_amount);
        }

        if let Some(frags) = self.frags {
            bytes.extend_from_slice(&frags.to_be_bytes());
        }

        if let Some(deaths) = self.deaths {
            bytes.extend_from_slice(&deaths.to_be_bytes());
        }

        if let Some(score) = self.score {
            bytes.extend_from_slice(&score.to_be_bytes());
        }

        if let Some(name) = self.name {
            bytes.extend_from_slice(&name);
        }

        if let Some(team) = self.team {
            bytes.push(team as u8);
        }

        if let Some(ping) = self.ping {
            bytes.push(ping);
        }
    }

    fn read_from_stream<E>(cursor: &mut Cursor<&[u8]>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let id = cursor
            .read_u8()
            .map_err(|_| E::custom("Error reading snapshot_id"))?;
        let head = cursor
            .read_u16::<BigEndian>()
            .map_err(|_| E::custom("Error reading snapshot_head"))?;
        Ok(PlayerSnapshot {
            id: id,
            translation: if head & 0b00000000_00000001 != 0 {
                Some(Vec3 {
                    x: cursor
                        .read_f32::<BigEndian>()
                        .map_err(|_| E::custom("Error reading player translation x"))?,
                    y: cursor
                        .read_f32::<BigEndian>()
                        .map_err(|_| E::custom("Error reading player translation y"))?,
                    z: cursor
                        .read_f32::<BigEndian>()
                        .map_err(|_| E::custom("Error reading player translation z"))?,
                })
            } else {
                None
            },
            rotation: if head & 0b00000000_00000010 != 0 {
                Some(QuantizedRotation {
                    pitch: cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading player pitch"))?,
                    yaw: cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading player yaw"))?,
                    roll: cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading player roll"))?,
                })
            } else {
                None
            },
            velocity: if head & 0b00000000_00000100 != 0 {
                Some(QuantizedVec3U16 {
                    x: cursor
                        .read_u16::<BigEndian>()
                        .map_err(|_| E::custom("Error reading player velocity x"))?,
                    y: cursor
                        .read_u16::<BigEndian>()
                        .map_err(|_| E::custom("Error reading player velocity y"))?,
                    z: cursor
                        .read_u16::<BigEndian>()
                        .map_err(|_| E::custom("Error reading player velocity z"))?,
                })
            } else {
                None
            },
            health: if head & 0b00000000_00010000 != 0 {
                Some(
                    cursor
                        .read_i16::<BigEndian>()
                        .map_err(|_| E::custom("Error reading player health"))?,
                )
            } else {
                None
            },
            frozen_amount: if head & 0b00000000_00001000 != 0 {
                Some(
                    cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading frozen amount"))?,
                )
            } else {
                None
            },
            frags: if head & 0b00000000_00100000 != 0 {
                Some(
                    cursor
                        .read_u16::<BigEndian>()
                        .map_err(|_| E::custom("Error reading player frags"))?,
                )
            } else {
                None
            },
            deaths: if head & 0b00000000_01000000 != 0 {
                Some(
                    cursor
                        .read_u16::<BigEndian>()
                        .map_err(|_| E::custom("Error reading player deaths"))?,
                )
            } else {
                None
            },
            score: if head & 0b00000000_10000000 != 0 {
                Some(
                    cursor
                        .read_u16::<BigEndian>()
                        .map_err(|_| E::custom("Error reading player score"))?,
                )
            } else {
                None
            },
            name: if head & 0b00000001_00000000 != 0 {
                let mut name_bytes = [00u8; 32];
                cursor
                    .read_exact(&mut name_bytes)
                    .map_err(|_| E::custom("Error reading player name"))?;
                Some(name_bytes)
            } else {
                None
            },
            team: if head & 0b00000010_00000000 != 0 {
                Some(
                    cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading player team"))?
                        .into(),
                )
            } else {
                None
            },
            ping: if head & 0b00000100_00000000 != 0 {
                Some(
                    cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading player ping"))?,
                )
            } else {
                None
            },
            entity: None,
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct PickupSnapshot {
    pub id: u8,
    pub translation: Option<QuantizedVec3U16>,
    pub kind: Option<PickupKind>,
}

impl PickupSnapshot {
    pub fn from_pickup(pickup: &Pickup, transform: &Transform) -> PickupSnapshot {
        PickupSnapshot {
            id: pickup.id,
            translation: Some(QuantizedVec3U16::from_vec3(
                &transform.translation,
                TRANSFORM_QUANTIZE_RANGE,
            )),
            kind: Some(pickup.kind.clone()),
        }
    }

    pub fn update_pickup(&self, pickup: &mut Pickup, transform: &mut Transform) {
        if let Some(translation) = &self.translation {
            transform.translation = translation.to_vec3(TRANSFORM_QUANTIZE_RANGE);
        }
        if let Some(kind) = &self.kind {
            pickup.kind = kind.clone();
        }
    }
}

impl SnapshotTrait for PickupSnapshot {
    type Value = PickupSnapshot;
    fn id_u64(&self) -> u64 {
        self.id as u64
    }

    fn diff(&self, old: &PickupSnapshot) -> PickupSnapshot {
        Self {
            id: self.id,
            translation: if self.translation != old.translation {
                self.translation.clone()
            } else {
                None
            },
            kind: if self.kind != old.kind {
                self.kind.clone()
            } else {
                None
            },
        }
    }

    fn is_empty(&self) -> bool {
        self.translation.is_none() && self.kind.is_none()
    }

    fn write_to_stream(&self, bytes: &mut Vec<u8>) {
        bytes.push(self.id);
        let mut head = 0;

        if self.translation.is_some() {
            head |= 0b00000001;
        }

        if self.kind.is_some() {
            head |= 0b00000010;
        }

        bytes.push(head);

        if let Some(translation) = self.translation {
            // quantize the position
            let mut translation_bytes = [0u8; 6];
            BigEndian::write_u16_into(
                &[translation.x, translation.y, translation.z],
                &mut translation_bytes,
            );
            bytes.extend_from_slice(&translation_bytes);
        }

        if let Some(kind) = &self.kind {
            bytes.write_u8(kind.clone() as u8).unwrap();
        }
    }

    fn read_from_stream<E>(cursor: &mut Cursor<&[u8]>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let id = cursor
            .read_u8()
            .map_err(|_| E::custom("Error reading pickup_id"))?;
        let head = cursor
            .read_u8()
            .map_err(|_| E::custom("Error reading pickup_head"))?;
        Ok(PickupSnapshot {
            id: id,
            translation: if head & 0b00000001 != 0 {
                Some(QuantizedVec3U16 {
                    x: cursor
                        .read_u16::<BigEndian>()
                        .map_err(|_| E::custom("Error reading pickup x"))?,

                    y: cursor
                        .read_u16::<BigEndian>()
                        .map_err(|_| E::custom("Error reading pickup y"))?,
                    z: cursor
                        .read_u16::<BigEndian>()
                        .map_err(|_| E::custom("Error reading pickup z"))?,
                })
            } else {
                None
            },
            kind: if head & 0b00000010 != 0 {
                Some(
                    cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading pickup kind"))?
                        .into(),
                )
            } else {
                None
            },
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct MonsterSnapshot {
    pub id: u8,
    pub kind: Option<MonsterKind>,
    pub translation: Option<QuantizedVec3U16>,
    pub rotation: Option<QuantizedRotation>,

    // authority only
    pub entity: Option<Entity>,
}

impl MonsterSnapshot {
    pub fn translation(&self) -> Option<Vec3> {
        self.translation
            .map(|t| t.to_vec3(TRANSFORM_QUANTIZE_RANGE))
    }

    pub fn rotation(&self) -> Option<Quat> {
        self.rotation.map(|r| r.to_quat())
    }

    pub fn from_monster(
        entity: Entity,
        monster: &Monster,
        transform: &Transform,
    ) -> MonsterSnapshot {
        MonsterSnapshot {
            id: monster.id,
            translation: Some(QuantizedVec3U16::from_vec3(
                &transform.translation,
                TRANSFORM_QUANTIZE_RANGE,
            )),
            rotation: Some(QuantizedRotation::from_quat(&transform.rotation)),
            kind: Some(monster.kind.clone()),
            entity: Some(entity),
        }
    }

    pub fn update_monster(&self, monster: &mut Monster, transform: &mut Transform) {
        if let Some(translation) = &self.translation {
            transform.translation = translation.to_vec3(TRANSFORM_QUANTIZE_RANGE);
        }
        if let Some(rotation) = &self.rotation {
            transform.rotation = rotation.to_quat();
        }
        if let Some(kind) = &self.kind {
            monster.kind = kind.clone();
        }
    }
}

impl SnapshotTrait for MonsterSnapshot {
    type Value = MonsterSnapshot;
    fn id_u64(&self) -> u64 {
        self.id as u64
    }

    fn diff(&self, old: &MonsterSnapshot) -> MonsterSnapshot {
        Self {
            id: self.id,
            kind: if self.kind != old.kind {
                self.kind.clone()
            } else {
                None
            },
            translation: if self.translation != old.translation {
                self.translation.clone()
            } else {
                None
            },
            rotation: if self.rotation != old.rotation {
                self.rotation.clone()
            } else {
                None
            },
            entity: None,
        }
    }

    fn is_empty(&self) -> bool {
        self.translation.is_none() && self.kind.is_none()
    }

    fn write_to_stream(&self, bytes: &mut Vec<u8>) {
        bytes.push(self.id);
        let mut head = 0;

        if self.kind.is_some() {
            head |= 0b00000001;
        }

        if self.translation.is_some() {
            head |= 0b00000010;
        }

        if self.rotation.is_some() {
            head |= 0b00000100;
        }

        bytes.push(head);

        if let Some(kind) = &self.kind {
            bytes.push(kind.clone() as u8);
        }

        if let Some(translation) = self.translation {
            // quantize the position
            let mut translation_bytes = [0u8; 6];
            BigEndian::write_u16_into(
                &[translation.x, translation.y, translation.z],
                &mut translation_bytes,
            );
            bytes.extend_from_slice(&translation_bytes);
        }

        if let Some(rotation) = self.rotation {
            // quantize the rotation
            let rotation_bytes = [rotation.pitch, rotation.yaw, rotation.roll];
            bytes.extend_from_slice(&rotation_bytes);
        }
    }

    fn read_from_stream<E>(cursor: &mut Cursor<&[u8]>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let id = cursor
            .read_u8()
            .map_err(|_| E::custom("Error reading monster_id"))?;
        let head = cursor
            .read_u8()
            .map_err(|_| E::custom("Error reading monster_head"))?;
        Ok(MonsterSnapshot {
            id: id,
            kind: if head & 0b00000001 != 0 {
                Some(
                    cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading monster kind"))?
                        .into(),
                )
            } else {
                None
            },
            translation: if head & 0b00000010 != 0 {
                Some(QuantizedVec3U16 {
                    x: cursor
                        .read_u16::<BigEndian>()
                        .map_err(|_| E::custom("Error reading monster translation x"))?,
                    y: cursor
                        .read_u16::<BigEndian>()
                        .map_err(|_| E::custom("Error reading monster translation y"))?,
                    z: cursor
                        .read_u16::<BigEndian>()
                        .map_err(|_| E::custom("Error reading monster translation z"))?,
                })
            } else {
                None
            },
            rotation: if head & 0b00000100 != 0 {
                Some(QuantizedRotation {
                    pitch: cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading monster pitch"))?,
                    yaw: cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading monster yaw"))?,
                    roll: cursor
                        .read_u8()
                        .map_err(|_| E::custom("Error reading monster roll"))?,
                })
            } else {
                None
            },
            entity: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_diff() {
        let old = Snapshot {
            id: 0,
            absolute: true,
            players: vec![
                PlayerSnapshot {
                    id: 0,
                    translation: Some(Vec3 {
                        x: 20.0,
                        y: 30.0,
                        z: 0.0,
                    }),
                    rotation: Some(QuantizedRotation {
                        pitch: 200,
                        yaw: 220,
                        roll: 100,
                    }),
                    velocity: Some(QuantizedVec3U16 { x: 0, y: 0, z: 0 }),
                    frozen_amount: Some(100),
                    health: Some(100),
                    frags: Some(24),
                    deaths: Some(20),
                    score: Some(20),
                    name: Some([0; 32]),
                    team: Some(Team::AntiVirus),
                    ping: Some(20),
                    entity: None,
                },
                PlayerSnapshot {
                    id: 0,
                    translation: Some(Vec3 {
                        x: 20.0,
                        y: 30.0,
                        z: 0.0,
                    }),
                    rotation: Some(QuantizedRotation {
                        pitch: 200,
                        yaw: 220,
                        roll: 100,
                    }),
                    velocity: Some(QuantizedVec3U16 { x: 0, y: 0, z: 0 }),
                    frozen_amount: Some(100),
                    health: Some(100),
                    frags: Some(24),
                    deaths: Some(20),
                    score: Some(20),
                    name: Some([0; 32]),
                    team: Some(Team::AntiVirus),
                    ping: Some(20),
                    entity: None,
                },
            ],
            monsters: vec![
                MonsterSnapshot {
                    id: 0,
                    kind: Some(MonsterKind::GruntLasers),
                    translation: Some(QuantizedVec3U16 { x: 0, y: 0, z: 0 }),
                    rotation: Some(QuantizedRotation {
                        pitch: 200,
                        yaw: 220,
                        roll: 100,
                    }),
                    entity: None,
                },
                MonsterSnapshot {
                    id: 0,
                    kind: Some(MonsterKind::GruntLasers),
                    translation: Some(QuantizedVec3U16 { x: 0, y: 0, z: 0 }),
                    rotation: Some(QuantizedRotation {
                        pitch: 200,
                        yaw: 220,
                        roll: 100,
                    }),
                    entity: None,
                },
                MonsterSnapshot {
                    id: 0,
                    kind: Some(MonsterKind::GruntLasers),
                    translation: Some(QuantizedVec3U16 { x: 0, y: 0, z: 0 }),
                    rotation: Some(QuantizedRotation {
                        pitch: 200,
                        yaw: 220,
                        roll: 100,
                    }),
                    entity: None,
                },
            ],
            pickups: vec![PickupSnapshot {
                id: 0,
                translation: Some(QuantizedVec3U16 {
                    x: 200,
                    y: 45,
                    z: 300,
                }),
                kind: Some(PickupKind::Health),
            }],
            local_last_processed_input_id: Some(0),
            local_player_id: Some(0),
            player_deletions: Vec::new(),
            monster_deletions: Vec::new(),
            pickup_deletions: Vec::new(),
        };

        let new = Snapshot {
            id: 0,
            absolute: true,
            players: vec![
                PlayerSnapshot {
                    id: 0,
                    translation: Some(Vec3 {
                        x: 0.0,
                        y: 50.0,
                        z: 75.0,
                    }),
                    rotation: Some(QuantizedRotation {
                        pitch: 200,
                        yaw: 220,
                        roll: 100,
                    }),
                    velocity: Some(QuantizedVec3U16 { x: 0, y: 0, z: 0 }),
                    frozen_amount: Some(15),
                    health: Some(200),
                    frags: Some(24),
                    deaths: Some(20),
                    score: Some(20),
                    name: Some([0; 32]),
                    team: Some(Team::AntiVirus),
                    ping: Some(20),
                    entity: None,
                },
                PlayerSnapshot {
                    id: 0,
                    translation: Some(Vec3 {
                        x: 20.0,
                        y: 30.0,
                        z: 0.0,
                    }),
                    rotation: Some(QuantizedRotation {
                        pitch: 200,
                        yaw: 220,
                        roll: 100,
                    }),
                    velocity: Some(QuantizedVec3U16 { x: 0, y: 0, z: 0 }),
                    frozen_amount: Some(100),
                    health: Some(100),
                    frags: Some(24),
                    deaths: Some(20),
                    score: Some(20),
                    name: Some([0; 32]),
                    team: Some(Team::AntiVirus),
                    ping: Some(20),
                    entity: None,
                },
            ],
            monsters: vec![
                MonsterSnapshot {
                    id: 0,
                    kind: Some(MonsterKind::GruntLasers),
                    translation: Some(QuantizedVec3U16 { x: 0, y: 0, z: 0 }),
                    rotation: Some(QuantizedRotation {
                        pitch: 200,
                        yaw: 220,
                        roll: 100,
                    }),
                    entity: None,
                },
                MonsterSnapshot {
                    id: 0,
                    kind: Some(MonsterKind::GruntLasers),
                    translation: Some(QuantizedVec3U16 { x: 0, y: 0, z: 0 }),
                    rotation: Some(QuantizedRotation {
                        pitch: 200,
                        yaw: 220,
                        roll: 100,
                    }),
                    entity: None,
                },
                MonsterSnapshot {
                    id: 0,
                    kind: Some(MonsterKind::GruntLasers),
                    translation: Some(QuantizedVec3U16 { x: 0, y: 0, z: 0 }),
                    rotation: Some(QuantizedRotation {
                        pitch: 200,
                        yaw: 220,
                        roll: 100,
                    }),
                    entity: None,
                },
            ],
            pickups: vec![PickupSnapshot {
                id: 0,
                translation: Some(QuantizedVec3U16 {
                    x: 200,
                    y: 45,
                    z: 300,
                }),
                kind: Some(PickupKind::Health),
            }],
            local_last_processed_input_id: Some(0),
            local_player_id: Some(0),
            player_deletions: Vec::new(),
            monster_deletions: Vec::new(),
            pickup_deletions: Vec::new(),
        };

        let expected_diff = Snapshot {
            id: 0,
            absolute: false,

            players: vec![PlayerSnapshot {
                id: 0,
                translation: Some(Vec3 {
                    x: 0.0,
                    y: 50.0,
                    z: 75.0,
                }),
                rotation: None,
                velocity: None,
                frozen_amount: Some(15),
                health: Some(200),
                frags: None,
                deaths: None,
                score: None,
                name: None,
                team: None,
                ping: None,
                entity: None,
            }],
            monsters: Vec::new(),
            pickups: Vec::new(),
            local_last_processed_input_id: None,
            local_player_id: None,
            player_deletions: Vec::new(),
            monster_deletions: Vec::new(),
            pickup_deletions: Vec::new(),
        };

        assert_eq!(new.diff(&old), expected_diff);

        // serialize it
        let serialized_diff = bincode::serialize(&expected_diff).unwrap();

        // Deserialize the diff
        let deserialized_diff: Snapshot = bincode::deserialize(&serialized_diff).unwrap();

        // Compare the original and deserialized diff
        assert_eq!(expected_diff, deserialized_diff);
    }

    #[test]
    fn test_snapshot_serialization() {
        let snapshot = Snapshot {
            id: 0,
            absolute: true,
            players: vec![PlayerSnapshot {
                id: 0,
                translation: Some(Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                }),
                rotation: Some(QuantizedRotation {
                    pitch: 200,
                    yaw: 220,
                    roll: 100,
                }),
                velocity: Some(QuantizedVec3U16 { x: 0, y: 0, z: 0 }),
                frozen_amount: Some(100),
                health: Some(-10),
                frags: Some(24),
                deaths: Some(20),
                score: Some(20),
                name: Some([0; 32]),
                team: Some(Team::AntiVirus),
                ping: Some(20),
                entity: None,
            }],
            monsters: vec![MonsterSnapshot {
                id: 0,
                kind: Some(MonsterKind::GruntLasers),
                translation: Some(QuantizedVec3U16 { x: 0, y: 0, z: 0 }),
                rotation: Some(QuantizedRotation {
                    pitch: 200,
                    yaw: 220,
                    roll: 100,
                }),
                entity: None,
            }],
            pickups: vec![PickupSnapshot {
                id: 0,
                translation: Some(QuantizedVec3U16 {
                    x: 200,
                    y: 45,
                    z: 300,
                }),
                kind: Some(PickupKind::Health),
            }],
            local_last_processed_input_id: Some(0),
            local_player_id: Some(0),
            player_deletions: Vec::new(),
            monster_deletions: Vec::new(),
            pickup_deletions: Vec::new(),
        };

        // Serialize the snapshot (bincode)
        let serialized_snapshot = bincode::serialize(&snapshot).unwrap();

        // Deserialize the snapshot
        let deserialized_snapshot: Snapshot = bincode::deserialize(&serialized_snapshot).unwrap();

        // Compare the original and deserialized snapshots
        assert_eq!(snapshot, deserialized_snapshot);
    }
}
