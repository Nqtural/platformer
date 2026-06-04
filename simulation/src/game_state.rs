use crate::{
    constants::POST_GAME_TIMER, map::Map, team::Team, utils::current_and_enemy, PlayerInput,
};
use ggez::input::keyboard::KeyCode;
use std::collections::HashSet;

#[derive(Clone)]
pub struct GameState {
    pub c_team: usize,
    pub c_player: usize,
    pub teams: [Team; 2],
    pub map: Map,
    pub winner: usize,
    pub post_game_timer: f32,
}

impl GameState {
    pub fn new(c_team: usize, c_player: usize, teams: [Team; 2]) -> Self {
        Self {
            c_team,
            c_player,
            teams,
            map: Map::new(),
            winner: 0,
            post_game_timer: POST_GAME_TIMER,
        }
    }

    pub fn fixed_update(&mut self, dt: f32) {
        self.check_for_win();

        self.update_post_game_timer(dt);

        for i in 0..2 {
            let (current, enemy) = current_and_enemy(&mut self.teams, i);
            current.update_players(enemy, self.map.get_rect(), self.winner, dt);
        }
    }

    fn update_post_game_timer(&mut self, dt: f32) {
        if self.winner != 0 {
            self.post_game_timer -= dt;
        }
    }

    pub fn check_for_win(&mut self) {
        if self.winner > 0 {
            return;
        }

        for (team_idx, team) in self.teams.iter_mut().enumerate() {
            if team.all_players_dead() {
                self.winner = if team_idx == 0 { 2 } else { 1 };
                break;
            }
        }
    }

    pub fn update_input(&mut self, pressed: &HashSet<KeyCode>) {
        self.teams[self.c_team].players[self.c_player]
            .input
            .update(pressed);
    }

    pub fn apply_input(&mut self, team_index: usize, player_index: usize, input: PlayerInput) {
        self.teams[team_index].players[player_index].input = input;
    }

    pub fn is_game_over(&self) -> bool {
        self.post_game_timer <= 0.0
    }
}
