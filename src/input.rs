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
    lunge: bool,
}

impl PlayerInput {
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
            lunge: false,
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
            KeyCode::L => self.lunge = value,
            _ => {}
        }
    }

    pub fn jump(&self) -> bool { self.jump }
    pub fn set_jump(&mut self, value: bool) { self.up = value }
    pub fn up(&self) -> bool { self.up }
    pub fn left(&self) -> bool { self.left }
    pub fn right(&self) -> bool { self.right }
    pub fn slam(&self) -> bool { self.slam }
    pub fn set_slam(&mut self, value: bool) { self.slam = value }
    pub fn dash(&self) -> bool { self.dash }
    pub fn light(&self) -> bool { self.light }
    pub fn set_light(&mut self, value: bool) { self.light = value }
    pub fn normal(&self) -> bool { self.normal }
    pub fn set_normal(&mut self, value: bool) { self.normal = value }
    pub fn lunge(&self) -> bool { self.lunge }
    pub fn set_lunge(&mut self, value: bool) { self.lunge = value }
}
