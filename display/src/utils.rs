use ggez::graphics::{
    Color as GgezColor,
    Rect as GgezRect,
};
use foundation::color::Color;
use foundation::rect::Rect;
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

pub fn color_to_ggez(color: &Color) -> GgezColor {
    GgezColor::new(
        color.r,
        color.g,
        color.b,
        color.a,
    )
}

pub fn rect_to_ggez(rect: &Rect) -> GgezRect {
    GgezRect::new(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
    )
}
