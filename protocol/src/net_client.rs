use foundation::GameMode;
use serde::{Deserialize, Serialize};
use simulation::PlayerInput;
use wincode::{SchemaRead, SchemaWrite};

#[derive(Serialize, Deserialize, SchemaRead, SchemaWrite)]
pub enum ClientMessage {
    Hello {
        player_name: String,
    },
    QueueJoin(GameMode),
    QueueLeave,
    Input {
        client_tick: u64,
        input: PlayerInput,
    },
}
