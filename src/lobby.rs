use ggez::graphics::Color;
use serde::{
    Serialize,
    Deserialize,
};
use std::net::SocketAddr;
use crate::{
    constants::{
        TEAM_ONE_START_POS,
        TEAM_SIZE,
        TEAM_TWO_START_POS,
    },
    network::InitTeamData,
};

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
        if TEAM_SIZE == 1 {
            self.next_team = (self.next_team + 1) % 2;
        } else if TEAM_SIZE == 2 {
            self.next_player = (self.next_player + 1) % TEAM_SIZE;
            if self.next_player == 0 {
                self.next_team = (self.next_team + 1) % 2;
            }
        }

        (team_id, player_id)
    }

    #[must_use]
    pub fn initial_teams(
        &self,
        team_one_color: Color,
        team_two_color: Color
    ) -> Vec<InitTeamData> {
        // Prepare output vec of length 2
        let mut teams = vec![
            InitTeamData {
                color: team_one_color,
                player_names: vec![String::new(); TEAM_SIZE],
                start_position: TEAM_ONE_START_POS,
                index: 0,
            },
            InitTeamData {
                color: team_two_color,
                player_names: vec![String::new(); TEAM_SIZE],
                start_position: TEAM_TWO_START_POS,
                index: 1,
            },
        ];

        // Fill players into correct team + slot
        for p in &self.players {
            if p.team_id < 2 && p.player_id < TEAM_SIZE {
                teams[p.team_id].player_names[p.player_id] = p.name.clone();
            }
        }

        teams
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
