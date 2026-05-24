use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::Mutex;

use simulation::game_state::GameState;
use client_logic::{
    interpolation::SnapshotHistory,
    render_clock::RenderClock,
};

#[derive(Default)]
pub struct ServerState {
    pub game_state: Option<Arc<Mutex<GameState>>>,
    pub snapshot_history: Arc<Mutex<SnapshotHistory>>,
    pub render_clock: RenderClock,
    pub render_tick: Arc<Mutex<f32>>,
    pub tick: Arc<AtomicU64>,
}
