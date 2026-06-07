use anyhow::Result;
use ggez::input::keyboard::KeyCode;
use protocol::init::InitData;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64};
use tokio::sync::{Mutex, watch};
use uuid::Uuid;

use crate::interpolation::SnapshotHistory;
use crate::render_clock::RenderClock;
use simulation::simulation::SimulationCore;

#[derive(Clone)]
pub enum ClientEvent {
    EndGame,
}

pub struct ClientState {
    pub player_id: Uuid,
    pub event_tx: watch::Sender<Option<ClientEvent>>,
    pub event_rx: watch::Receiver<Option<ClientEvent>>,
    pub current_input: Arc<Mutex<HashSet<KeyCode>>>,
    pub snapshot_history: Arc<Mutex<SnapshotHistory>>,
    pub render_clock: Arc<Mutex<RenderClock>>,
    pub render_tick: Arc<Mutex<f32>>,
    pub core: Arc<Mutex<SimulationCore>>,
    pub tick: Arc<AtomicU64>,
    pub shutdown: Arc<AtomicBool>,
}

impl ClientState {
    pub fn new(player_id: Uuid, init_data: InitData) -> Result<Self> {
        let gs = init_data.to_game_state();
        let (event_tx, event_rx) = watch::channel(None);

        Ok(Self {
            player_id,
            event_tx,
            event_rx,
            current_input: Arc::new(Mutex::new(HashSet::new())),
            snapshot_history: Arc::new(Mutex::new(SnapshotHistory::default())),
            render_clock: Arc::new(Mutex::new(RenderClock::default())),
            render_tick: Arc::new(Mutex::new(0.0)),
            core: Arc::new(Mutex::new(SimulationCore::new(gs))),
            tick: Arc::new(AtomicU64::new(0)),
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }
}
