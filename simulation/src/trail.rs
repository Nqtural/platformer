use crate::constants::TRAIL_OPACITY;
use foundation::color::Color;
use foundation::rect::Rect;

#[derive(Clone)]
pub struct TrailSquare {
    pub rect: Rect,
    pub color: Color,
    pub lifetime: f32,
}

impl TrailSquare {
    #[must_use]
    pub fn new(rect: Rect, color: Color) -> TrailSquare {
        TrailSquare {
            rect,
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
            TRAIL_OPACITY * (self.lifetime / 0.15).powf(2.0),
        );
    }
}
