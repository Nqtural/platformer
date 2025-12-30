use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::Mutex;
use ggez::input::keyboard::KeyCode;

use protocol::net_team::InitTeamData;
use protocol::net_game_state::new_from_initial;
use simulation::simulation::SimulationCore;
use crate::interpolation::SnapshotHistory;
use crate::render_clock::RenderClock;

pub struct ClientState {
    pub team_id: usize,
    pub player_id: usize,
    pub current_input: Arc<Mutex<HashSet<KeyCode>>>,
    pub snapshot_history: Arc<Mutex<SnapshotHistory>>,
    pub render_clock: Arc<Mutex<RenderClock>>,
    pub render_tick: Arc<Mutex<f32>>,
    pub core: Arc<Mutex<SimulationCore>>,
    pub tick: Arc<AtomicU64>,
    pub tick_accumulator: Mutex<f32>,
}

impl ClientState {
    pub fn new(
        team_id: usize,
        player_id: usize,
        teams: Vec<InitTeamData>
    ) -> Result<Self> {
        let gs = new_from_initial(team_id, player_id, teams)?;

        Ok(Self {
            team_id,
            player_id,
            current_input: Arc::new(Mutex::new(HashSet::new())),
            snapshot_history: Arc::new(Mutex::new(SnapshotHistory::default())),
            render_clock: Arc::new(Mutex::new(RenderClock::default())),
            render_tick: Arc::new(Mutex::new(0.0)),
            core: Arc::new(Mutex::new(SimulationCore::new(gs))),
            tick: Arc::new(AtomicU64::new(0)),
            tick_accumulator: Mutex::new(0.0),
        })
    }
}
