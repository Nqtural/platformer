use crate::{
    network::InitTeamData,
    player::Player,
    rect::Rect,
};

#[derive(Clone)]
pub struct Team {
    pub players: Vec<Player>,
}

impl Team {
    #[must_use]
    fn new(players: Vec<Player>) -> Team {
        Team {
            players,
        }
    }

    #[must_use]
    pub fn from_init(init: InitTeamData) -> Team {
        let mut players = Vec::new();

        for name in init.player_names.iter() {
            players.push(Player::new(init.start_position, name.clone(), init.color.clone(), init.index));
        }

        Team::new(players)
    }

    pub fn update_players(
        &mut self,
        enemy_team: &mut Team,
        map: &Rect,
        winner: usize,
        mut dt: f32,
    ) {
        if winner > 0 {
            dt /= 2.0;
        }

        for player_idx in 0..self.players.len() {
            let player = &mut self.players[player_idx];
            if player.lives() == 0 { continue; }

            for atk in player.attacks().clone() {
                let atk_rect = atk.get_rect(player.position());

                for enemy in &mut enemy_team.players {
                    if atk_rect.overlaps(&enemy.get_rect()) {
                        enemy.attack(&atk, player);
                    }
                }
            }

            player.update(map, enemy_team, dt);

            player.apply_input(map, player_idx, dt);
        }
    }
}
