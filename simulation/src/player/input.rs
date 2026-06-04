use ggez::input::keyboard::KeyCode;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use wincode::{SchemaRead, SchemaWrite};

#[derive(SchemaRead, SchemaWrite, Serialize, Deserialize, Clone, Default, PartialEq, Debug)]
pub struct PlayerInput {
    pub jump: bool,
    pub up: bool,
    pub left: bool,
    pub right: bool,
    pub slam: bool,
    pub dash: bool,
    pub light: bool,
    pub normal: bool,
    pub parry: bool,
}

impl PlayerInput {
    #[must_use]
    pub fn new() -> PlayerInput {
        PlayerInput {
            jump: false,
            up: false,
            left: false,
            right: false,
            slam: false,
            dash: false,
            light: false,
            normal: false,
            parry: false,
        }
    }

    pub fn update(&mut self, pressed: &HashSet<KeyCode>) {
        self.jump = pressed.contains(&KeyCode::Space);
        self.up = pressed.contains(&KeyCode::W);
        self.left = pressed.contains(&KeyCode::A);
        self.right = pressed.contains(&KeyCode::D);
        self.slam = pressed.contains(&KeyCode::S);
        self.dash = pressed.contains(&KeyCode::H);
        self.normal = pressed.contains(&KeyCode::J);
        self.light = pressed.contains(&KeyCode::K);
        self.parry = pressed.contains(&KeyCode::L) || pressed.contains(&KeyCode::LShift);
    }
}
