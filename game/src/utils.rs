use ggez::graphics::{
    Color as GgezColor,
    Rect as GgezRect,
};
use foundation::color::Color;
use foundation::rect::Rect;


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
