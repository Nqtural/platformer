use ggez::graphics::Rect;
use serde::{
    Deserialize,
    Serialize,
};
use crate::player::Player;

#[derive(Serialize, Deserialize, Clone)]
pub enum AttackKind {
    Light,
    Uppercut,
}

impl AttackKind {
    fn attack(&self, player: &mut Player, owner_vel: [f32; 2], owner_facing: f32) {
        match self {
            AttackKind::Light => {
                player.stunned = 0.2;
                player.invulnerable_timer = 0.1;
                player.slow = 0.5;
                player.vel[0] = (owner_vel[0] / 2.0 + 400.0 * owner_facing) * player.knockback_multiplier;
                player.vel[1] = -200.0;
                player.knockback_multiplier += 0.01;
                player.dashing = 0.0;
            }
            AttackKind::Uppercut => {
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
    owner_vel: [f32; 2],
    owner_facing: f32,
}

impl Attack {
    pub fn new(player: &Player, kind: AttackKind, owner_team: usize) -> Attack {
        Attack {
            x: player.pos[0] - 9.0,
            y: player.pos[1] - 10.0,
            w: 40.0,
            h: 40.0,
            kind,
            duration: 0.1,
            timer: 0.0,
            owner_team,
            owner_vel: player.vel,
            owner_facing: player.facing,
        }
    }

    pub fn owner_team(&self) -> usize {
        self.owner_team
    }

    pub fn attack(&self, player: &mut Player) {
        self.kind.attack(player, self.owner_vel, self.owner_facing);
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
