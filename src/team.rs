use ggez::graphics::{
    Color,
    Rect,
};
use serde::{
    Deserialize,
    Serialize,
};
use crate::{
    attack::Attack,
    player::Player,
    trail::TrailSquare,
    utils::handle_collisions,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Team {
    pub players: Vec<Player>,
    pub color_default: Color,
    pub color_stunned: Color,
    pub trail_interval: f32,
    pub trail_squares: Vec<TrailSquare>,
    pub start_pos: [f32; 2],
}

impl Team {
    pub fn new(players: Vec<Player>, color: Color, start_pos: [f32; 2]) -> Team {
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
            start_pos,
        }
    }

    pub fn update_players(
            &mut self,
            enemy_team: &mut Team,
            team_idx: usize,
            map: &Rect,
            winner: usize,
            active_attacks: &mut Vec<Attack>,
            mut dt: f32,
        ) {
            if winner > 0 {
                dt = dt / 2.0;
            }

            self.trail_squares.iter_mut().for_each(|s| s.update(dt));
            self.trail_squares.retain(|s| s.lifetime > 0.0);

        for player_idx in 0..self.players.len() {
            // Split self.players into [head | current | rest]
            let (head, right) = self.players.split_at_mut(player_idx);
            let (player, rest) = right.split_first_mut().unwrap();

            player.update(&map, enemy_team, self.start_pos, dt);

            active_attacks.extend(
                player.apply_input(
                    map,
                    team_idx,
                    player_idx,
                    dt
                )
            );

            if player.dashing > 0.0 || player.slamming {
                let others = head.iter_mut()
                    .chain(rest.iter_mut())
                    .chain(enemy_team.players.iter_mut());

                handle_collisions(player, others);

                while player.trail_timer >= self.trail_interval {
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
    }

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
