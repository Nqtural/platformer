use ggez::graphics::{
    Color,
    Rect,
};
use crate::constants::{
    PLAYER_SIZE,
    TRAIL_OPACITY,
};

#[derive(Clone, Debug)]
pub struct TrailSquare {
    pub rect: Rect,
    pub color: Color,
    pub lifetime: f32,
}

impl TrailSquare {
    #[must_use]
    pub fn new(pos: [f32; 2], color: Color) -> TrailSquare {
        TrailSquare {
            rect: Rect::new(pos[0], pos[1], PLAYER_SIZE, PLAYER_SIZE),
            color: Color::new(color.r, color.g, color.b, TRAIL_OPACITY),
            lifetime: 0.15,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.lifetime -= dt;
        self.color = Color::new(
            self.color.r,
            self.color.g,
            self.color.b,
            TRAIL_OPACITY * (self.lifetime / 0.15).powf(2.0) 
        );
    }
}
