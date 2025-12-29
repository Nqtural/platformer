use foundation::{
    color::Color,
    rect::Rect,
};
use crate::trail::TrailSquare;

#[derive(Clone)]
pub struct PlayerVisuals {
    pub trail_squares: Vec<TrailSquare>,
    pub trail_timer: f32,
}

impl Default for PlayerVisuals {
    fn default() -> Self {
        Self {
            trail_squares: Vec::new(),
            trail_timer: 0.0,
        }
    }
}

impl PlayerVisuals {
    pub fn tick(
        &mut self,
        dt: f32,
        rect: Rect,
        color: Color,
        trail_active: bool,
    ) {
        self.update_trail(dt);
        if trail_active {
            self.spawn_trail_squares(rect, color);
        }
    }

    fn update_trail(&mut self, dt: f32) {
        self.trail_squares.iter_mut().for_each(|s| s.update(dt));
        self.trail_squares.retain(|s| s.lifetime > 0.0);

        self.trail_timer += dt;
    }

    fn spawn_trail_squares(&mut self, rect: Rect, color: Color) {
        if self.trail_timer >= 0.01 {
            self.trail_timer = 0.0;
            self.trail_squares.push(TrailSquare::new(rect, color));
        }
    }
}
