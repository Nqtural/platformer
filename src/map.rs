use ggez::graphics::{
    Color,
    Rect,
};
use serde::{
    Deserialize,
    Serialize,
};
use crate::constants::MAP_COLOR;

#[derive(Serialize, Deserialize, Clone)]
pub struct Map {
    pub rect: Rect,
    pub color: Color,
}

impl Map {
    pub fn new() -> Map {
        Map {
            rect: Rect::new(200.0, 350.0, 400.0, 30.0),
            color: MAP_COLOR,
        }
    }
}
