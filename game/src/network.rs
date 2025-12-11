use serde::{
    Serialize,
    Deserialize,
};
use crate::{
    attack::AttackKind,
    input::PlayerInput,
    lobby::LobbyPlayer,
};
use foundation::color::Color;

#[derive(Serialize, Deserialize, Clone)]
pub struct InitTeamData {
    pub color: Color,
    pub player_names: Vec<String>,
    pub start_position: [f32; 2],
    pub index: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NetSnapshot {
    pub tick: u64,
    pub winner: usize,
    pub players: Vec<NetPlayer>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NetPlayer {
    pub team_idx: usize,
    pub player_idx: usize,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub combo: u32,
    pub knockback_multiplier: f32,
    pub attacks: Vec<NetAttack>,
    pub stunned: f32,
    pub invulnerable: f32,
    pub parry: f32,
    pub lives: u8,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NetAttack {
    pub timer: f32,
    pub owner_team: usize,
    pub owner_player: usize,
    pub kind: AttackKind,
    pub facing: [f32; 2],
    pub frame: usize,
}

#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    Hello { name: String },
    Input {
        tick: u64,
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
