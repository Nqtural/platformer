use ggez::graphics::Rect;
use serde::{
    Deserialize,
    Serialize,
};
use crate::player::Player;

#[derive(Serialize, Deserialize, Clone)]
pub struct Attack {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    pub power: f32,
    pub stun: f32,
    pub knockback: [f32; 2],
    pub slow: f32,
    pub duration: f32,
    pub timer: f32,
    pub owner_team: usize,
}

impl Attack {
    pub fn light(player: &Player, owner_team: usize) -> Attack {
        Attack {
            x: player.pos[0] - 9.0,
            y: player.pos[1] - 10.0,
            w: 40.0,
            h: 40.0,
            power: 0.01,
            stun: 0.2,
            knockback: [
                (player.vel[0] / 2.0) + (400.0 * player.facing),
                -200.0
            ],
            slow: 0.5,
            duration: 0.1,
            timer: 0.0,
            owner_team,
        }
    }
    pub fn uppercut(player: &Player, owner_team: usize) -> Attack {
        Attack {
            x: player.pos[0] - 9.0,
            y: player.pos[1] - 10.0,
            w: 40.0,
            h: 40.0,
            power: 0.02,
            stun: 0.4,
            knockback: [0.0, -500.0],
            slow: 0.5,
            duration: 0.15,
            timer: 0.0,
            owner_team,
        }
    }

    pub fn get_rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.w, self.h)
    }
}
