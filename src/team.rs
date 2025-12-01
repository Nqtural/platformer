use ggez::graphics::{
    Rect,
};
use serde::{
    Deserialize,
    Serialize,
};
use crate::{
    attack::AttackKind,
    network::InitTeamData,
    player::Player,
    trail::TrailSquare,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Team {
    pub players: Vec<Player>,
    pub trail_interval: f32,
    pub trail_squares: Vec<TrailSquare>,
}

impl Team {
    #[must_use]
    pub fn new(players: Vec<Player>) -> Team {
        Team {
            players,
            trail_interval: 0.01,
            trail_squares: Vec::new(),
        }
    }

    #[must_use]
    pub fn from_init(init: InitTeamData) -> Team {
        let mut players = Vec::new();
        let names = init.player_names;
        let positions = init.start_positions;

        for (i, name) in names.iter().enumerate() {
            let pos = positions
                .get(i)
                .copied()
                .unwrap_or_else(|| positions.first().copied().unwrap_or([0.0, 0.0]));
            players.push(Player::new(pos, name.clone(), init.color));
        }

        Team::new(players)
    }

    pub fn update_players(
        &mut self,
        enemy_team: &mut Team,
        team_idx: usize,
        map: &Rect,
        winner: usize,
        mut dt: f32,
    ) {
        if winner > 0 {
            dt /= 2.0;
        }

        self.trail_squares.iter_mut().for_each(|s| s.update(dt));
        self.trail_squares.retain(|s| s.lifetime > 0.0);

        for player_idx in 0..self.players.len() {
            let (head, player_and_tail) = self.players.split_at_mut(player_idx);
            let (player_slice, tail) = player_and_tail.split_at_mut(1);
            let player = &mut player_slice[0];

            for atk_index in 0..player.attacks.len() {
                let kind = player.attacks[atk_index].kind().clone();
                let atk_rect = player.attacks[atk_index].get_rect(player.pos);

                for enemy in head.iter_mut().chain(tail.iter_mut()) {
                    if atk_rect.overlaps(&enemy.get_rect()) {
                        enemy.attack(&kind, player);
                    }
                }
            }
        }

        for player_idx in 0..self.players.len() {
            let player = &mut self.players[player_idx];
            if player.lives == 0 { continue; }

            player.update(map, enemy_team, dt);

            player.apply_input(map, team_idx, player_idx, dt);

            while player.trail_timer >= self.trail_interval
            && (
                player.is_doing_attack(&AttackKind::Slam)
                || player.is_doing_attack(&AttackKind::Dash)
                )
            {
                player.trail_timer -= self.trail_interval;
                self.trail_squares.push(
                    TrailSquare::new(
                        player.pos,
                        player.color,
                    )
                )
            }
        }
    }
}
