use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::Mutex;
use ggez::input::keyboard::KeyCode;

use protocol::net_team::InitTeamData;
use protocol::net_game_state::new_from_initial;
use simulation::game_state::GameState;
use crate::interpolation::SnapshotHistory;
use crate::render_clock::RenderClock;

pub struct ClientState {
    pub team_id: usize,
    pub player_id: usize,
    pub current_input: Arc<Mutex<HashSet<KeyCode>>>,
    pub snapshot_history: Arc<Mutex<SnapshotHistory>>,
    pub render_clock: RenderClock,
    pub render_tick: Arc<Mutex<f32>>,
    pub game_state: Option<Arc<Mutex<GameState>>>,
    pub tick: Arc<AtomicU64>,
}

impl ClientState {
    pub fn apply_initial_data(
        &mut self,
        teams: Vec<InitTeamData>,
    ) -> Result<()> {
        let gs = new_from_initial(self.team_id, self.player_id, teams)?;
        self.game_state = Some(Arc::new(Mutex::new(gs)));
        Ok(())
    }
}

