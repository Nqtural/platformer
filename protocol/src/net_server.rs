use serde::{
    Serialize,
    Deserialize,
};
use crate::{
    lobby::LobbyPlayer,
    net_player::NetPlayer,
    net_team::InitTeamData,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct NetSnapshot {
    pub tick: u64,
    pub winner: usize,
    pub players: Vec<NetPlayer>,
}

#[derive(Serialize, Deserialize, Clone)]
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
        teams: Vec<InitTeamData>,
    },
    Snapshot(NetSnapshot),
}
