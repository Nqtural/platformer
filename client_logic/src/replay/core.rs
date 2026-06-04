use foundation::color::Color;
use protocol::constants::{TEAM_ONE_START_POS, TEAM_TWO_START_POS};
use serde::{Deserialize, Serialize};
use simulation::{Player, PlayerInput, game_state::GameState, team::Team};
use wincode::{SchemaRead, SchemaWrite};

#[derive(SchemaRead, SchemaWrite, Serialize, Deserialize)]
pub struct Replay {
    version: u32,
    player_names: [Vec<String>; 2],
    inputs: [Vec<PlayerInput>; 2],
}

impl Replay {
    pub fn new(player_names: [Vec<String>; 2]) -> Self {
        Self {
            version: 1,
            player_names,
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
                Team::new(
                    self.player_names[0]
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
                    self.player_names[1]
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
        )
    }
}
