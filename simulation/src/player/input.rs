use ggez::input::keyboard::KeyCode;
use serde::{
    Deserialize,
    Serialize,
};
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Clone, Default, PartialEq, Debug)]
pub struct PlayerInput {
    jump: bool,
    up: bool,
    left: bool,
    right: bool,
    slam: bool,
    dash: bool,
    light: bool,
    normal: bool,
    parry: bool,
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
        self.jump   = pressed.contains(&KeyCode::Space);
        self.up     = pressed.contains(&KeyCode::W);
        self.left   = pressed.contains(&KeyCode::A);
        self.right  = pressed.contains(&KeyCode::D);
        self.slam   = pressed.contains(&KeyCode::S);
        self.dash   = pressed.contains(&KeyCode::H);
        self.normal = pressed.contains(&KeyCode::J);
        self.light  = pressed.contains(&KeyCode::K);
        self.parry  = pressed.contains(&KeyCode::L)
            || pressed.contains(&KeyCode::LShift);
    }

    #[must_use]
    pub fn jump(&self) -> bool { self.jump }

    #[must_use]
    pub fn up(&self) -> bool { self.up }

    #[must_use]
    pub fn left(&self) -> bool { self.left }

    #[must_use]
    pub fn right(&self) -> bool { self.right }

    #[must_use]
    pub fn slam(&self) -> bool { self.slam }

    #[must_use]
    pub fn dash(&self) -> bool { self.dash }

    #[must_use]
    pub fn light(&self) -> bool { self.light }

    #[must_use]
    pub fn normal(&self) -> bool { self.normal }

    #[must_use]
    pub fn parry(&self) -> bool { self.parry }

    // SETTERS
    pub fn set_jump(&mut self, value: bool) { self.up = value }
    pub fn set_slam(&mut self, value: bool) { self.slam = value }
    pub fn set_light(&mut self, value: bool) { self.light = value }
    pub fn set_normal(&mut self, value: bool) { self.normal = value }
}
