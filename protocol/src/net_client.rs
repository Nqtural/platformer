use serde::{
    Serialize,
    Deserialize,
};
use simulation::PlayerInput;

#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    Hello { name: String },
    Input {
        client_tick: u64,
        team_id: usize,
        player_id: usize,
        input: PlayerInput,
    },
}
