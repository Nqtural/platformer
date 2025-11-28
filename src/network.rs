use ggez::graphics::Color;
use serde::{
    Serialize,
    Deserialize,
};
use crate::{
    attack::AttackKind,
    input::PlayerInput,
    lobby::LobbyPlayer,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct InitTeamData {
    pub name: String,
    pub color: Color,
    pub player_names: Vec<String>,
    pub start_positions: Vec<[f32; 2]>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NetSnapshot {
    pub winner: usize,
    pub players: Vec<NetPlayer>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NetPlayer {
    pub team_id: usize,
    pub player_id: usize,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub attacks: Vec<NetAttack>,
    pub stunned: f32,
    pub invulnerable: f32,
    pub lives: u8,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NetAttack {
    pub owner_team: usize,
    pub owner_player: usize,
    pub kind: AttackKind,
    pub facing: [f32; 2],
}

#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    Hello { name: String },
    Input {
        team_id: usize,
        player_id: usize,
        input: PlayerInput,
    },
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ServerMessage {
    Welcome {
        team_id: usize,
        player_id: usize,
        name: String,
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
