use serde::{Deserialize, Serialize};
use simulation::PlayerInput;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReplayMetadata {
    team_size: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Replay {
    version: u32,
    metadata: ReplayMetadata,
    inputs: [Vec<PlayerInput>; 2],
}

impl Replay {
    pub fn new(team_size: usize) -> Self {
        Self {
            version: 1,
            metadata: ReplayMetadata { team_size },
            inputs: [Vec::new(), Vec::new()],
        }
    }

    pub fn store(&mut self, team: usize, input: PlayerInput) {
        self.inputs[team].push(input);
    }
}
