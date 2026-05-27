use anyhow::Result;
use foundation::color::Color;
use ggez::input::keyboard::KeyCode;
use protocol::constants::{TEAM_ONE_START_POS, TEAM_TWO_START_POS};
use simulation::Player;
use simulation::game_state::GameState;
use simulation::team::Team;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::Mutex;

use crate::interpolation::SnapshotHistory;
use crate::render_clock::RenderClock;
use simulation::simulation::SimulationCore;

pub struct ClientState {
    pub team_id: usize,
    pub player_id: usize,
    pub current_input: Arc<Mutex<HashSet<KeyCode>>>,
    pub snapshot_history: Arc<Mutex<SnapshotHistory>>,
    pub render_clock: Arc<Mutex<RenderClock>>,
    pub render_tick: Arc<Mutex<f32>>,
    pub core: Arc<Mutex<SimulationCore>>,
    pub tick: Arc<AtomicU64>,
}

impl ClientState {
    pub fn new(
        team_id: usize,
        player_id: usize,
        player_names: [Vec<String>; 2],
        trail_delay: f32,
        trail_opacity: f32,
        trail_lifetime: f32,
    ) -> Result<Self> {
        let gs = GameState::new(
            0,
            0,
            [
                Team::new(
                    player_names[0]
                        .iter()
                        .map(|n| {
                            Player::new(
                                TEAM_ONE_START_POS,
                                n.clone(),
                                Color::new(0.0, 0.0, 1.0, 1.0),
                                0,
                                trail_delay,
                                trail_opacity,
                                trail_lifetime,
                            )
                        })
                        .collect(),
                ),
                Team::new(
                    player_names[1]
                        .iter()
                        .map(|n| {
                            Player::new(
                                TEAM_TWO_START_POS,
                                n.clone(),
                                Color::new(1.0, 0.0, 0.0, 1.0),
                                1,
                                trail_delay,
                                trail_opacity,
                                trail_lifetime,
                            )
                        })
                        .collect(),
                ),
            ],
        );

        Ok(Self {
            team_id,
            player_id,
            current_input: Arc::new(Mutex::new(HashSet::new())),
            snapshot_history: Arc::new(Mutex::new(SnapshotHistory::default())),
            render_clock: Arc::new(Mutex::new(RenderClock::default())),
            render_tick: Arc::new(Mutex::new(0.0)),
            core: Arc::new(Mutex::new(SimulationCore::new(gs))),
            tick: Arc::new(AtomicU64::new(0)),
        })
    }
}
