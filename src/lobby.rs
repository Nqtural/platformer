use ggez::graphics::Color;
use serde::{
    Serialize,
    Deserialize,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use crate::network::InitTeamData;

#[derive(Serialize, Deserialize, Clone)]
pub struct LobbyPlayer {
    pub addr: SocketAddr,
    pub team_id: usize,
    pub player_id: usize,
    pub name: String,
    pub connected: bool,
}

pub struct Lobby {
    pub players: Vec<LobbyPlayer>,
    pub team_colors: HashMap<usize, Color>,
    pub team_start_positions: HashMap<usize, Vec<[f32; 2]>>,

    next_team: usize,
    next_player: usize,
}

impl Default for Lobby {
    fn default() -> Self {
        Self::new()
    }
}

impl Lobby {
    #[must_use]
    pub fn new() -> Self {
        Self {
            players: Vec::new(),
            team_colors: HashMap::new(),
            team_start_positions: HashMap::new(),
            next_team: 0,
            next_player: 0,
        }
    }

    pub fn assign_slot(
        &mut self,
        addr: SocketAddr,
        name: String
    ) -> (usize, usize) {
        // assign team + player slots
        let team_id = self.next_team;
        let player_id = self.next_player;

        self.players.push(LobbyPlayer {
            addr,
            team_id,
            player_id,
            name,
            connected: true,
        });

        // rotate slots
        self.next_player += 1;
        if self.next_player >= 2 {
            self.next_player = 0;
            self.next_team += 1;
        }

        (team_id, player_id)
    }

    #[must_use]
    pub fn initial_teams(&self) -> Vec<InitTeamData> {
        let mut map: HashMap<usize, InitTeamData> = HashMap::new();

        for p in &self.players {
            map.entry(p.team_id)
                .or_insert_with(|| InitTeamData {
                    name: format!("Team {}", p.team_id),
                    color: self.team_colors.get(&p.team_id)
                        .cloned()
                        .unwrap_or(Color::WHITE),
                    player_names: Vec::new(),
                    start_positions: self.team_start_positions
                        .get(&p.team_id)
                        .cloned()
                        .unwrap_or_default(),
                })
                .player_names.push(p.name.clone());
        }

        map.into_values().collect()
    }

    // GETTERS
    #[must_use]
    pub fn players_list(&self) -> Vec<(usize, usize, String)> {
        self.players
            .iter()
            .map(|p| (p.team_id, p.player_id, p.name.clone()))
            .collect()
    }

    #[must_use]
    pub fn connected_count(&self) -> usize { self.players.len() }
}
