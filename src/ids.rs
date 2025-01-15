use bevy::prelude::*;
use bevy::utils::HashMap;

use crate::net::AUTHORITY_ID;

#[derive(Resource, Default)]
pub struct IdPooler {
    next_projectile_id: u16,
    pickup_id_pool: Vec<u8>,
    monster_id_pool: Vec<u8>,
    player_id_index_map: HashMap<u64, u8>,
}

impl IdPooler {
    pub fn new(server_has_player: bool) -> Self {
        if server_has_player {
            Self {
                next_projectile_id: 0,
                pickup_id_pool: Vec::new(),
                monster_id_pool: Vec::new(),

                // zero is always reserved for the server/single-player
                player_id_index_map: HashMap::from([(AUTHORITY_ID, 0)]),
            }
        } else {
            Self {
                next_projectile_id: 0,
                pickup_id_pool: Vec::new(),
                monster_id_pool: Vec::new(),
                player_id_index_map: HashMap::new(),
            }
        }
    }

    fn assign_and_reserve_id(pool: &mut Vec<u8>) -> Result<u8, ()> {
        for i in 0..u8::MAX {
            if !pool.contains(&i) {
                pool.push(i);
                return Ok(i);
            }
        }

        Err(())
    }

    fn release_id(pool: &mut Vec<u8>, id: u8) {
        pool.retain(|&x| x != id);
    }

    pub fn assign_and_reserve_pickup_id(&mut self) -> Result<u8, ()> {
        Self::assign_and_reserve_id(&mut self.pickup_id_pool)
    }

    pub fn release_pickup_id(&mut self, id: u8) {
        Self::release_id(&mut self.pickup_id_pool, id)
    }

    pub fn assign_and_reserve_monster_id(&mut self) -> Result<u8, ()> {
        Self::assign_and_reserve_id(&mut self.monster_id_pool)
    }

    pub fn release_bot_id(&mut self, id: u8) {
        Self::release_id(&mut self.monster_id_pool, id)
    }

    pub fn next_projectile_id(&mut self) -> u16 {
        let id = self.next_projectile_id;
        self.next_projectile_id = self.next_projectile_id.wrapping_add(1);
        id
    }

    pub fn assign_and_reserve_player_id(&mut self, client_id: u64) -> Result<u8, ()> {
        // zero is always reserved for the server/single-player
        for i in 1..u8::MAX {
            if !self.player_id_index_map.values().any(|&id| id == i) {
                self.player_id_index_map.insert(client_id, i);
                return Ok(i);
            }
        }

        Err(())
    }

    pub fn release_player_id(&mut self, client_id: u64) {
        self.player_id_index_map.remove(&client_id);
    }

    pub fn get_player_id(&self, client_id: u64) -> Option<&u8> {
        self.player_id_index_map.get(&client_id)
    }

    pub fn get_all_player_ids(&self) -> Vec<&u8> {
        self.player_id_index_map.values().collect()
    }

    pub fn get_client_id_from_player_id(&self, player_id: u8) -> Option<&u64> {
        self.player_id_index_map
            .iter()
            .find_map(|(k, &v)| if v == player_id { Some(k) } else { None })
    }
}
