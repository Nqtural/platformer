use ggez::graphics::{
    Canvas, Color as GgezColor, DrawParam, Drawable, PxScale, Text, TextFragment,
};
use ggez::{Context, GameResult};
use glam::Vec2;

fn draw_centered_text(
    game_canvas: &mut Canvas,
    ctx: &Context,
    text: &str,
    scale: f32,
    y_offset: f32,
) -> GameResult {
    let (w, h) = ctx.gfx.drawable_size();
    let center = Vec2::new(w / 2.0, h / 2.0);

    let text = Text::new(TextFragment {
        text: text.to_string(),
        font: None,
        scale: Some(PxScale::from(scale)),
        color: Some(GgezColor::WHITE),
    });

    let dims = text.dimensions(ctx).unwrap_or_default();

    let pos = Vec2::new(center.x - dims.w / 2.0, center.y - dims.h / 2.0 + y_offset);

    game_canvas.draw(&text, DrawParam::default().dest(pos));

    Ok(())
}

pub fn draw_menu(ctx: &mut Context) -> GameResult {
    let mut canvas = Canvas::from_frame(&ctx.gfx, GgezColor::BLACK);

    draw_centered_text(&mut canvas, ctx, "Main Menu", 64.0, -40.0)?;
    draw_centered_text(&mut canvas, ctx, "Press R to queue", 28.0, 40.0)?;

    canvas.finish(&mut ctx.gfx)
}

pub fn draw_queue(ctx: &mut Context) -> GameResult {
    let mut canvas = Canvas::from_frame(&ctx.gfx, GgezColor::BLACK);

    draw_centered_text(&mut canvas, ctx, "Queuing...", 48.0, -20.0)?;
    draw_centered_text(&mut canvas, ctx, "Press Esc to cancel", 24.0, 40.0)?;

    canvas.finish(&mut ctx.gfx)
}
