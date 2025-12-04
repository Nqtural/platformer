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
            players.push(Player::new(pos, name.clone(), init.color, init.index));
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

        self.trail_squares.iter_mut().for_each(|s| s.update(dt));
        self.trail_squares.retain(|s| s.lifetime > 0.0);

        for player_idx in 0..self.players.len() {
            let player = &mut self.players[player_idx];
            if player.lives == 0 { continue; }

            for atk in player.attacks.clone() {
                let atk_rect = atk.get_rect(player.pos);

                for enemy in &mut enemy_team.players {
                    if atk_rect.overlaps(&enemy.get_rect()) {
                        enemy.attack(&atk, player);
                    }
                }
            }

            player.update(map, enemy_team, dt);

            player.apply_input(map, player_idx, dt);

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
                );
            }
        }
    }
}
