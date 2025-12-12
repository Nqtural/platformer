use ggez::input::keyboard::KeyCode;
use std::collections::HashSet;

#[derive(Default)]
pub struct InputState {
    pub pressed: HashSet<KeyCode>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            pressed: HashSet::new(),
        }
    }

    pub fn press(&mut self, key: KeyCode) {
        self.pressed.insert(key);
    }

    pub fn release(&mut self, key: KeyCode) {
        self.pressed.remove(&key);
    }
}
