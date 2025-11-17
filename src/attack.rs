use ggez::graphics::Rect;
use serde::{
    Deserialize,
    Serialize,
};
use crate::{
    constants::PLAYER_SIZE,
    player::Player,
};

#[derive(Serialize, Deserialize, Clone)]
pub enum AttackKind {
    Dash,
    Light,
    Slam,
    Normal,
}

impl AttackKind {
    pub fn attack(&self, enemy: &mut Player, player: &mut Player) {
        match self {
            AttackKind::Dash => {
                if enemy.dashing > 0.0 {
                    enemy.vel[0] = player.vel[0].signum() * 100.0 * enemy.knockback_multiplier;
                    enemy.dashing = 0.0;
                    enemy.stunned = 0.5;
                    enemy.knockback_multiplier += 0.01;

                    player.stunned = 0.5;
                } else {
                    enemy.vel[0] = player.vel[0] * enemy.knockback_multiplier;
                    enemy.stunned = 0.1;
                }

                enemy.vel[1] -= 200.0;
                enemy.slow = 0.5;

                player.vel[1] -= 200.0;
                player.vel[0] = player.vel[0] * -0.5;
                player.dashing = 0.0;
                player.slow = 0.5;
            }
            AttackKind::Light => {
                player.stunned = 0.2;
                player.invulnerable_timer = 0.1;
                player.slow = 0.5;
                player.vel[0] = (enemy.vel[0] / 2.0 + 400.0 * enemy.facing) * player.knockback_multiplier;
                player.vel[1] = -200.0;
                player.knockback_multiplier += 0.01;
                player.dashing = 0.0;
            }
            AttackKind::Slam => {
                let player_bottom = player.pos[1] + PLAYER_SIZE;
                let enemy_top = enemy.pos[1];

                if player_bottom <= enemy_top + 5.0 && player.vel[1] > 0.0 {
                    enemy.vel[1] = player.vel[1] * 1.5 * enemy.knockback_multiplier;
                    enemy.stunned = 0.1;
                    enemy.slow = 0.5;
                    enemy.knockback_multiplier += 0.03;

                    player.vel[1] = -100.0;
                    player.slow = 0.5;
                    player.input.set_slam(false);
                }
            }
            AttackKind::Normal => {
                player.stunned = 0.4;
                player.invulnerable_timer = 0.1;
                player.slow = 0.5;
                player.vel[0] = 0.0;
                player.vel[1] = -500.0;
                player.knockback_multiplier += 0.02;
                player.dashing = 0.0;
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Attack {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    kind: AttackKind,
    duration: f32,
    timer: f32,
    owner_team: usize,
    owner_player: usize,
}

impl Attack {
    pub fn new(
        player: &Player,
        kind: AttackKind,
        owner_team: usize,
        owner_player: usize,
    ) -> Attack {
        Attack {
            x: player.pos[0] - 9.0,
            y: player.pos[1] - 10.0,
            w: 40.0,
            h: 40.0,
            kind,
            duration: 0.1,
            timer: 0.0,
            owner_team,
            owner_player,
        }
    }

    pub fn owner_team(&self) -> usize {
        self.owner_team
    }

    pub fn owner_player(&self) -> usize {
        self.owner_player
    }

    pub fn attack(&self, enemy: &mut Player, player: &mut Player) {
        self.kind.attack(enemy, player);
    }

    pub fn update(&mut self, dt: f32) {
        self.timer += dt;
    }

    pub fn is_expired(&self) -> bool {
        self.timer >= self.duration
    }

    pub fn get_rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.w, self.h)
    }
}
