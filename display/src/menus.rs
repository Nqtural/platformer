use ggez::graphics::{
    Canvas, Color as GgezColor, DrawParam, Drawable, PxScale, Text, TextFragment,
};
use ggez::{Context, GameResult};
use glam::Vec2;

fn draw_centered_text(
    game_canvas: &mut Canvas,
    ctx: &Context,
    text: &str,
    color: GgezColor,
    scale: f32,
    y_offset: f32,
) -> GameResult {
    let (w, h) = ctx.gfx.drawable_size();
    let center = Vec2::new(w / 2.0, h / 2.0);

    let text = Text::new(TextFragment {
        text: text.to_string(),
        font: None,
        scale: Some(PxScale::from(scale)),
        color: Some(color),
    });

    let dims = text.dimensions(ctx).unwrap_or_default();

    let pos = Vec2::new(center.x - dims.w / 2.0, center.y - dims.h / 2.0 + y_offset);

    game_canvas.draw(&text, DrawParam::default().dest(pos));

    Ok(())
}

pub fn draw_menu(ctx: &mut Context) -> GameResult {
    let mut canvas = Canvas::from_frame(&ctx.gfx, GgezColor::BLACK);

    draw_centered_text(&mut canvas, ctx, "Main Menu", GgezColor::WHITE, 64.0, -40.0)?;
    draw_centered_text(
        &mut canvas,
        ctx,
        "Press Space to queue",
        GgezColor::WHITE,
        28.0,
        40.0,
    )?;

    canvas.finish(&mut ctx.gfx)
}

pub fn draw_queue(ctx: &mut Context) -> GameResult {
    let mut canvas = Canvas::from_frame(&ctx.gfx, GgezColor::BLACK);

    draw_centered_text(
        &mut canvas,
        ctx,
        "Queuing...",
        GgezColor::WHITE,
        48.0,
        -20.0,
    )?;
    draw_centered_text(
        &mut canvas,
        ctx,
        "Press Esc to cancel",
        GgezColor::WHITE,
        24.0,
        40.0,
    )?;

    canvas.finish(&mut ctx.gfx)
}

pub fn draw_replay_picker(
    ctx: &mut Context,
    replay_files: Vec<String>,
    selected_index: usize,
    page: usize,
    page_max: usize,
) -> GameResult {
    let mut canvas = Canvas::from_frame(&ctx.gfx, GgezColor::BLACK);

    draw_centered_text(&mut canvas, ctx, "Replays", GgezColor::WHITE, 48.0, -40.0)?;
    for (i, replay_file) in replay_files.iter().enumerate() {
        draw_centered_text(
            &mut canvas,
            ctx,
            replay_file,
            if i == selected_index {
                GgezColor::BLUE
            } else {
                GgezColor::WHITE
            },
            16.0,
            20.0 * i as f32,
        )?;
    }

    draw_centered_text(
        &mut canvas,
        ctx,
        &format!("{page}/{page_max}"),
        GgezColor::WHITE,
        20.0,
        80.0,
    )?;

    canvas.finish(&mut ctx.gfx)
}
