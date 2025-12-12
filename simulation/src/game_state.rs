use ggez::input::keyboard::KeyCode;
use crate::{
    map::Map,
    team::Team,
    utils::current_and_enemy,
};
use std::collections::HashSet;

#[derive(Clone)]
pub struct GameState {
    pub c_team: usize,
    pub c_player: usize,
    pub teams: [Team; 2],
    pub map: Map,
    pub winner: usize,
}

impl GameState {
    pub fn new(
        c_team: usize,
        c_player: usize,
        teams: [Team; 2],
    ) -> Self {
        Self {
            c_team,
            c_player,
            teams,
            map: Map::new(),
            winner: 0,
        }
    }

    pub fn fixed_update(&mut self, dt: f32) {
        self.check_for_win();

        for i in 0..2 {
            let (current, enemy) = current_and_enemy(&mut self.teams, i);
            current.update_players(
                enemy,
                self.map.get_rect(),
                self.winner,
                dt,
            );
        }
    }

    pub fn check_for_win(&mut self) {
        if self.winner > 0 {
            return;
        }

        for (team_idx, team) in self.teams.iter_mut().enumerate() {
            if team.players.iter().all(|p| p.is_dead()) {
                self.winner = if team_idx == 0 { 2 } else { 1 };
                break;
            }
        }
    }

    pub fn update_input(&mut self, pressed: &HashSet<KeyCode>) {
        self.teams[self.c_team].players[self.c_player].update_input(pressed);
    }
}
