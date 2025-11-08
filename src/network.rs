use serde::{
    Serialize,
    Deserialize
};
use crate::{
    input::PlayerInput,
    game_state::GameState
};

#[derive(Serialize, Deserialize, Clone)]
pub enum ClientMessage {
    Input {
        player_id: usize,
        input: PlayerInput,
    },
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ServerMessage {
    Snapshot(GameState),
}
