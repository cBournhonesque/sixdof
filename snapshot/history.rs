use super::Snapshot;
use bevy::prelude::*;
use bevy_renet::renet::ClientId;
use std::collections::BTreeMap;

pub struct BubbledUpDeletions {
    pub player_deletions: Vec<u8>,
    pub monster_deletions: Vec<u8>,
    pub pickup_deletions: Vec<u8>,
}

#[derive(Resource, Default)]
pub struct SnapshotHistory {
    pub server_snapshots: BTreeMap<u64, Snapshot>,
    pub server_last_acked_snapshots: BTreeMap<ClientId, u64>,
}

impl SnapshotHistory {
    pub fn latest(&self) -> Option<&Snapshot> {
        self.server_snapshots.values().last()
    }

    pub fn second_latest(&self) -> Option<&Snapshot> {
        self.server_snapshots.values().nth_back(1)
    }

    pub fn clear(&mut self) {
        self.server_snapshots.clear();
        self.server_last_acked_snapshots.clear();
    }

    pub fn clean_old_snapshots(&mut self, latest_snapshot: &Snapshot) {
        self.server_snapshots
            .retain(|old_snap_id, _| latest_snapshot.id - old_snap_id < 1000);
    }

    ///
    /// Collects all deletions that have occurred since the last acknowledged snapshot for the given client.
    ///
    pub fn bubble_up_deletions(&self, client_id: &ClientId) -> BubbledUpDeletions {
        let mut player_deletions = Vec::new();
        let mut bot_deletions = Vec::new();
        let mut pickup_deletions = Vec::new();

        if let Some(last_acked_snapshot_id) = self.server_last_acked_snapshots.get(client_id) {
            for (snapshot_id, next_snapshot) in
                self.server_snapshots.range(last_acked_snapshot_id..)
            {
                if snapshot_id == last_acked_snapshot_id {
                    continue;
                }

                player_deletions.extend(next_snapshot.player_deletions.iter());
                bot_deletions.extend(next_snapshot.monster_deletions.iter());
                pickup_deletions.extend(next_snapshot.pickup_deletions.iter());
            }
        }

        BubbledUpDeletions {
            player_deletions,
            monster_deletions: bot_deletions,
            pickup_deletions,
        }
    }
}

#[derive(Resource, Default)]
pub struct SnapshotInterpolation {
    pub latest_reconciled_snapshot_id: Option<u64>,
    pub latest_reconciled_snapshot_time: Option<f64>,
    pub lerp_from: Option<Snapshot>,
    pub lerp_to: Option<Snapshot>,
}
