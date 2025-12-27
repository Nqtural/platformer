use ggez::{
    ContextBuilder,
    GameResult,
    input::keyboard::KeyCode,
};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use client_logic::interpolation::SnapshotHistory;
use simulation::constants::{
    VIRTUAL_HEIGHT,
    VIRTUAL_WIDTH,
};
use crate::render::Renderer;

pub fn run(
    input_tx: UnboundedSender<HashSet<KeyCode>>,
    snapshot_history: Arc<Mutex<SnapshotHistory>>,
    render_tick_clone: Arc<Mutex<f32>>,
    context_name: &str,
    vsync: bool,
) -> GameResult {
    let (ctx, event_loop) = ContextBuilder::new(context_name, "platform")
        .window_setup(
            ggez::conf::WindowSetup::default()
                .vsync(vsync)
                .title("Game")
        )
        .window_mode(
            ggez::conf::WindowMode::default()
                .dimensions(VIRTUAL_WIDTH, VIRTUAL_HEIGHT)
                .resizable(true)
        )
        .build()?;

    let renderer = Renderer::new(&ctx, snapshot_history, render_tick_clone, input_tx);
    ggez::event::run(
        ctx,
        event_loop,
        renderer,
    )
}
