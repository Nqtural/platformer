use std::time::Instant;
use simulation::constants::TICK_RATE;
use crate::constants::INTERPOLATION_DELAY;

pub struct RenderClock {
    last_server_tick: u64,
    last_server_time: Instant,
}

impl Default for RenderClock {
    fn default() -> Self {
        Self {
            last_server_tick: 0,
            last_server_time: Instant::now(),
        }
    }
}

impl RenderClock {
    pub fn update(&mut self, server_tick: u64) {
        self.last_server_tick = server_tick;
        self.last_server_time = Instant::now();
    }

    pub fn render_tick(&self) -> f32 {
        let elapsed = self.last_server_time.elapsed().as_secs_f32();
        let predicted_tick =
            self.last_server_tick as f32 + elapsed * TICK_RATE as f32;

        predicted_tick - INTERPOLATION_DELAY
    }
}
