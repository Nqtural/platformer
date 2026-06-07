use simulation::{Player, game_state::GameState};
use std::collections::HashMap;
use uuid::Uuid;
use wincode::{SchemaRead, SchemaWrite};

use crate::constants::{DUO_OFFSET, TEAM_ONE_START_POS, TEAM_TWO_START_POS};

#[derive(SchemaWrite, SchemaRead, Clone)]
pub struct InitPlayerData {
    pub name: String,
}

#[derive(SchemaWrite, SchemaRead, Clone)]
pub struct InitData {
    pub players: HashMap<String, InitPlayerData>,
    pub teams: [Vec<String>; 2],
}

impl InitData {
    pub fn to_game_state(&self) -> GameState {
        let mut players = HashMap::new();
        for (team_index, team) in self.teams.iter().enumerate() {
            for (player_index, player_id) in team.iter().enumerate() {
                players.insert(
                    Uuid::parse_str(player_id).expect("Invalid UUID string"),
                    Player::new(spawn_position(team_index, player_index), team_index),
                );
            }
        }

        let teams: Vec<Vec<Uuid>> = self
            .teams
            .iter()
            .map(|team| {
                team.iter()
                    .map(|id| Uuid::parse_str(id).expect("Invalid UUID string"))
                    .collect()
            })
            .collect();

        GameState::new(players, teams.try_into().expect("Expected exactly 2 teams"))
    }
}

fn spawn_position(team_id: usize, player_id: usize) -> [f32; 2] {
    match (team_id, player_id) {
        (0, 0) => TEAM_ONE_START_POS,
        (0, 1) => [TEAM_ONE_START_POS[0] + DUO_OFFSET, TEAM_ONE_START_POS[1]],
        (1, 0) => TEAM_TWO_START_POS,
        (1, 1) => [TEAM_TWO_START_POS[0] - DUO_OFFSET, TEAM_TWO_START_POS[1]],
        _ => unreachable!(),
    }
}
