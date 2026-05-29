use crate::{lobby::LobbyPlayer, net_player::NetPlayer};
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
    LobbyStatus {
        players: Vec<LobbyPlayer>,
        required: usize,
    },
    StartGame {
        c_team_id: usize,
        c_player_id: usize,
        player_names: [Vec<String>; 2],
    },
    EndGame,
    Snapshot {
        server_tick: u64,
        server_state: NetSnapshot,
    },
}
