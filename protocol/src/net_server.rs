use crate::{init::InitData, net_player::NetPlayer};
use serde::{Deserialize, Serialize};
use wincode::{SchemaRead, SchemaWrite};

#[derive(Serialize, Deserialize, Clone, SchemaWrite, SchemaRead)]
pub struct NetSnapshot {
    pub tick: u64,
    pub winner: usize,
    pub players: Vec<NetPlayer>,
}

#[derive(SchemaWrite, SchemaRead)]
pub enum ServerMessage {
    Welcome {
        team_id: usize,
        player_id: usize,
    },
    StartGame {
        c_player: String,
        init_data: InitData,
    },
    EndGame,
    Snapshot {
        server_tick: u64,
        server_state: NetSnapshot,
    },
}
