use crate::replay::core::Replay;
use crate::{interpolation::TimedSnapshot, replay::constants::REPLAY_SPEEDS};
use anyhow::Result;
use display::render::RenderState;
use simulation::constants::{FIXED_DT, TICK_RATE};
use simulation::game_state::GameState;
use wincode::deserialize;

pub struct ReplayViewer {
    pub render_state: RenderState,
    simulated_game: Vec<TimedSnapshot>,
    paused: bool,
    playback_speed_setting: usize,
    tick: u64,
    frame_time: f32,
}

impl ReplayViewer {
    pub fn new(
        render_state: RenderState,
        replay_path: &str,
        trail_delay: f32,
        trail_opacity: f32,
        trail_lifetime: f32,
    ) -> Result<Self> {
        Ok(Self {
            render_state,
            simulated_game: simulate_game(
                &load_replay(replay_path)?,
                trail_delay,
                trail_opacity,
                trail_lifetime,
            ),
            paused: false,
            playback_speed_setting: 3, // default is 1.0
            tick: 0,
            frame_time: 0.0,
        })
    }

    pub fn update(&mut self, dt: f32) {
        if self.paused {
            return;
        }

        self.frame_time += dt * REPLAY_SPEEDS[self.playback_speed_setting];

        while self.frame_time > FIXED_DT {
            self.tick += 1;
            self.frame_time -= FIXED_DT;
        }
    }

    pub fn get_current_state(&self) -> GameState {
        self.simulated_game
            .iter()
            .find(|s| s.server_tick == self.tick)
            .map(|s| &s.snapshot)
            .unwrap()
            .clone()
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn next_tick(&mut self) {
        self.paused = true;
        self.seek_by_ticks(1);
    }

    pub fn previous_tick(&mut self) {
        self.paused = true;
        self.seek_by_ticks(-1);
    }

    pub fn seek_forwards(&mut self) {
        self.seek_by_ticks(5 * TICK_RATE as isize)
    }

    pub fn seek_backwards(&mut self) {
        self.seek_by_ticks(-5 * TICK_RATE as isize)
    }

    pub fn speed_increase(&mut self) {
        self.playback_speed_setting =
            (self.playback_speed_setting + 1).min(REPLAY_SPEEDS.len() - 1);
    }

    pub fn speed_decrease(&mut self) {
        self.playback_speed_setting = (self.playback_speed_setting.saturating_sub(1)).max(0);
    }

    fn seek_by_ticks(&mut self, ticks: isize) {
        let len = self.simulated_game.len() as isize;
        let current = self.tick as isize;

        let new_tick = (current + ticks).max(0).min(len);

        self.tick = new_tick as u64;
    }
}

fn load_replay(path: &str) -> Result<Replay> {
    let bytes = std::fs::read(path)?;
    let replay = deserialize(&bytes)?;

    Ok(replay)
}

fn simulate_game(
    replay: &Replay,
    trail_delay: f32,
    trail_opacity: f32,
    trail_lifetime: f32,
) -> Vec<TimedSnapshot> {
    let mut simulated_game = Vec::new();
    let mut game_state = replay.create_game_state(trail_delay, trail_opacity, trail_lifetime);

    let mut i = 0;
    while i <= replay.length() {
        for (team_index, team) in game_state.teams.iter_mut().enumerate() {
            for player in &mut team.players {
                player.input = replay.load(team_index, i).cloned().unwrap_or_default();
            }
        }

        game_state.fixed_update(FIXED_DT);

        simulated_game.push(TimedSnapshot {
            server_tick: i as u64,
            snapshot: game_state.clone(),
        });

        i += 1;
    }

    simulated_game
}
