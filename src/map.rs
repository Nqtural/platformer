use ggez::graphics::{
    Color,
};
use crate::{
    constants::MAP_COLOR,
    rect::Rect,
};

#[derive(Clone)]
pub struct Map {
    rect: Rect,
    color: Color,
}

impl Default for Map {
    fn default() -> Self {
        Self::new()
    }
}

impl Map {
    #[must_use]
    pub fn new() -> Map {
        Map {
            rect: Rect::new(200.0, 350.0, 400.0, 30.0),
            color: MAP_COLOR,
        }
    }

    // GETTERS
    #[must_use]
    pub fn get_rect(&self) -> &Rect { &self.rect }

    #[must_use]
    pub fn get_color(&self) -> Color { self.color }
}
