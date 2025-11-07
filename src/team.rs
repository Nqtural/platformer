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
    trail_interval: f32,
    pub trail_squares: Vec<TrailSquare>,
    pub start_pos: [f32; 2],
}

impl Team {
    pub fn new(players: Vec<Player>, color: Color, start_pos: [f32; 2]) -> Team {
        Team {
            players,
            color_default: color,
            color_stunned: Color::new(color.r * 2.0, color.g * 2.0, color.b * 2.0, 1.0),
            trail_interval: 0.01,
            trail_squares: Vec::new(),
            start_pos
        }
    }

    pub fn update_players(
            &mut self,
            left: &mut [Team],
            others: &mut [Team],
            team_idx: usize,
            map: &Rect,
            winner: usize,
            active_attacks: &mut Vec<Attack>,
            mut normal_dt: f32,
        ) {
            if winner > 0 {
                normal_dt = normal_dt / 2.0;
            }

            let slow_dt = normal_dt / 2.0;

            self.trail_squares.iter_mut().for_each(|s| s.update(normal_dt));
            self.trail_squares.retain(|s| s.lifetime > 0.0);

            for (player_idx, player) in self.players.iter_mut().enumerate() {
                player.update_cooldowns(normal_dt);

                if player.respawn_timer > 0.0 { continue; }

                let dt = if player.slow > normal_dt {
                    slow_dt
                } else {
                    normal_dt
                };

                active_attacks.extend(player.apply_input(map, team_idx, player_idx, dt));

            if player.dashing > 0.0 || player.input.slam() {
                handle_collisions(player, left.iter_mut().chain(others.iter_mut()));
                player.trail_timer += dt;
                while player.trail_timer >= self.trail_interval {
                    player.trail_timer -= self.trail_interval;
                    self.trail_squares.push(
                        TrailSquare::new(
                            player.pos[0],
                            player.pos[1],
                            self.color_default
                        )
                    )
                }
            }

            player.update_position(dt);
            player.check_platform_collision(&map, dt);
            player.check_for_death(self.start_pos);
        }
    }
}
