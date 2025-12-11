use glam::Vec2;
use mint::{Point2, Vector2};

pub trait IntoMint {
    fn to_mint_point(self) -> Point2<f32>;
    fn to_mint_vec(self) -> Vector2<f32>;
}

impl IntoMint for Vec2 {
    fn to_mint_point(self) -> Point2<f32> {
        Point2 { x: self.x, y: self.y }
    }

    fn to_mint_vec(self) -> Vector2<f32> {
        Vector2 { x: self.x, y: self.y }
    }
}
