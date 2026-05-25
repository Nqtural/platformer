use protocol::net_team::{InitTeamData, from_init};
use serde::{Deserialize, Serialize};
use simulation::{PlayerInput, game_state::GameState};

#[derive(Serialize, Deserialize)]
pub struct Replay {
    version: u32,
    teams: [InitTeamData; 2],
    inputs: [Vec<PlayerInput>; 2],
}

impl Replay {
    pub fn new(teams: [InitTeamData; 2]) -> Self {
        Self {
            version: 1,
            teams,
            inputs: [Vec::new(), Vec::new()],
        }
    }

    pub fn store(&mut self, team: usize, input: PlayerInput) {
        self.inputs[team].push(input);
    }

    pub fn load(&self, team: usize, tick: usize) -> Option<&PlayerInput> {
        self.inputs[team].get(tick)
    }

    pub fn length(&self) -> usize {
        self.inputs[0].len()
    }
}

impl Replay {
    pub fn create_game_state(
        &self,
        trail_delay: f32,
        trail_opacity: f32,
        trail_lifetime: f32,
    ) -> GameState {
        GameState::new(
            0,
            0,
            [
                from_init(
                    self.teams[0].clone(),
                    trail_delay,
                    trail_opacity,
                    trail_lifetime,
                ),
                from_init(
                    self.teams[1].clone(),
                    trail_delay,
                    trail_opacity,
                    trail_lifetime,
                ),
            ],
        )
    }
}
