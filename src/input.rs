use ggez::input::keyboard::KeyCode;
use serde::{
    Deserialize,
    Serialize,
};

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
    pary: bool,
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
            pary: false,
        }
    }

    pub fn update(&mut self, keycode: KeyCode, value: bool) {
        match keycode {
            KeyCode::Space => self.jump = value,
            KeyCode::W => self.up = value,
            KeyCode::A => self.left = value,
            KeyCode::D => self.right = value,
            KeyCode::S => self.slam = value,
            KeyCode::H => self.dash = value,
            KeyCode::J => self.normal = value,
            KeyCode::K => self.light = value,
            KeyCode::L | KeyCode::LShift => self.pary = value,
            _ => {}
        }
    }

    // GETTERS
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
    pub fn pary(&self) -> bool { self.pary }

    // SETTERS
    pub fn set_jump(&mut self, value: bool) { self.up = value }
    pub fn set_slam(&mut self, value: bool) { self.slam = value }
    pub fn set_light(&mut self, value: bool) { self.light = value }
    pub fn set_normal(&mut self, value: bool) { self.normal = value }
}
