use foundation::color::Color;
use foundation::rect::Rect;

#[derive(Clone)]
pub struct TrailSquare {
    pub rect: Rect,
    pub color: Color,
    pub lifetime: f32,
    pub start_lifetime: f32,
}

impl TrailSquare {
    #[must_use]
    pub fn new(rect: Rect, color: Color, start_opacity: f32, lifetime: f32) -> TrailSquare {
        TrailSquare {
            rect,
            color: Color::new(color.r, color.g, color.b, start_opacity),
            lifetime,
            start_lifetime: lifetime,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.lifetime -= dt;

        let t = (self.lifetime / self.start_lifetime).clamp(0.0, 1.0);
        self.color.a *= t.powf(2.0);
    }
}
