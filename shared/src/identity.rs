use lightyear::prelude::ClientId;

pub enum Identity {
    Player(ClientId),
    Monster(u32),
}
