use serde::{
    Serialize,
    Deserialize,
};
use crate::{
    game_state::GameState,
    input::PlayerInput,
};

#[derive(Serialize, Deserialize, Clone)]
pub enum ClientMessage {
    Input {
        team_id: usize,
        player_id: usize,
        input: PlayerInput,
    },
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ServerMessage {
    Snapshot(GameState),
}
