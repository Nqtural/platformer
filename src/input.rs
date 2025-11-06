use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct PlayerInput {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub slam: bool,
    pub light: bool,
    pub uppercut: bool,
    pub dash: bool,
}

impl PlayerInput {
    pub fn new() -> PlayerInput {
        PlayerInput {
            left: false,
            right: false,
            up: false,
            slam: false,
            light: false,
            uppercut: false,
            dash: false,
        }
    }
}
