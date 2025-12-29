use serde::{
    Serialize,
    Deserialize,
};
use std::net::SocketAddr;
use foundation::color::Color;
use crate::net_team::InitTeamData;
use crate::constants::{
    TEAM_ONE_START_POS,
    TEAM_TWO_START_POS,
};
use crate::utils::condense_name;

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
    team_size: usize,
}

impl Lobby {
    #[must_use]
    pub fn new(team_size: usize) -> Self {
        Self {
            players: Vec::new(),
            next_team: 0,
            next_player: 0,
            team_size,
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
        if self.team_size == 1 {
            self.next_team = (self.next_team + 1) % 2;
        } else if self.team_size == 2 {
            #[allow(clippy::modulo_one)]
            {
                self.next_player = (self.next_player + 1) % self.team_size;
            }
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
            InitTeamData::new(team_one_color, TEAM_ONE_START_POS, 0, self.team_size),
            InitTeamData::new(team_two_color, TEAM_TWO_START_POS, 1, self.team_size),
        ];

        // Fill players into correct team + slot
        for p in &self.players {
            if p.team_id < 2 && p.player_id < self.team_size {
                teams[p.team_id].player_names[p.player_id] = condense_name(&p.name);
            }
        }

        teams
    }

    #[must_use]
    pub fn players_list(&self) -> Vec<(usize, usize, String)> {
        self.players
            .iter()
            .map(|p| (p.team_id, p.player_id, p.name.clone()))
            .collect()
    }

    #[must_use]
    pub fn connected_count(&self) -> usize { self.players.len() }

    #[must_use]
    pub fn required(&self) -> usize { self.team_size * 2 }


    #[must_use]
    pub fn is_full(&self) -> bool { self.connected_count() >= self.required() }
}
