use ggez::graphics::{
    Color,
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
    pub color_default: Color,
    pub color_stunned: Color,
    pub trail_interval: f32,
    pub trail_squares: Vec<TrailSquare>,
}

impl Team {
    pub fn new(players: Vec<Player>, color: Color) -> Team {
        Team {
            players,
            color_default: color,
            color_stunned: Color::new(
                (color.r + 0.4).min(1.0),
                (color.g + 0.4).min(1.0),
                (color.b + 0.4).min(1.0),
                1.0,
            ),
            trail_interval: 0.01,
            trail_squares: Vec::new(),
        }
    }

    pub fn from_init(init: InitTeamData) -> Team {
        let mut players = Vec::new();
        let names = init.player_names;
        let positions = init.start_positions;

        for (i, name) in names.iter().enumerate() {
            let pos = positions
                .get(i)
                .cloned()
                .unwrap_or_else(|| positions.first().cloned().unwrap_or([0.0, 0.0]));
            players.push(Player::new(pos, name.clone()));
        }

        Team::new(players, init.color)
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
            // split self.players into [head | current | rest]
            let (head, right) = self.players.split_at_mut(player_idx);
            let (player, rest) = right.split_first_mut().unwrap();

            for atk in player.attacks.clone().iter() {
                for enemy in &mut enemy_team.players {
                    enemy.handle_attack_collisions(atk, player);
                }
            }

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
                        self.color_default
                    )
                )
            }
        }
    }

    // GETTERS
    pub fn get_color(&self, invulnerable: bool, stunned: bool) -> Color {
        let color = if stunned {
            self.color_stunned
        } else {
            self.color_default
        };

        Color::new(
            color.r,
            color.g,
            color.b,
            if invulnerable { 0.5 } else { 1.0 }
        )
    }
}
