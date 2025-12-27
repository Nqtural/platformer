use crate::game_state::GameState;

pub struct SimulationCore {
    game_state: GameState,
}

impl SimulationCore {
    pub fn new(game_state: GameState) -> Self {
        Self { game_state }
    }

    pub fn step(&mut self, dt: f32) {
        self.game_state.fixed_update(dt);
    }

    pub fn game_state(&self) -> &GameState {
        &self.game_state
    }

    pub fn game_state_mut(&mut self) -> &mut GameState {
        &mut self.game_state
    }
}
