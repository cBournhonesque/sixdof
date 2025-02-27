use bevy::prelude::{Component};
use lightyear::prelude::{Deserialize, Serialize};

#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bot;


