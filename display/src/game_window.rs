use ggez::{
    ContextBuilder,
    GameResult,
    input::keyboard::KeyCode,
};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use simulation::{
    constants::{VIRTUAL_HEIGHT, VIRTUAL_WIDTH},
    game_state::GameState,
};
use crate::{
    constants::ENABLE_VSYNC,
    render::Renderer,
};

pub fn run(
    input_tx: UnboundedSender<HashSet<KeyCode>>,
    gs_clone: Arc<Mutex<GameState>>,
    context_name: &str,
) -> GameResult {
    let (ctx, event_loop) = ContextBuilder::new(context_name, "platform")
        .window_setup(
            ggez::conf::WindowSetup::default()
                .vsync(ENABLE_VSYNC)
                .title("Game")
        )
        .window_mode(
            ggez::conf::WindowMode::default()
                .dimensions(VIRTUAL_WIDTH, VIRTUAL_HEIGHT)
                .resizable(true)
        )
        .build()?;

    let renderer = Renderer::new(&ctx, gs_clone, input_tx);
    ggez::event::run(
        ctx,
        event_loop,
        renderer,
    )
}
