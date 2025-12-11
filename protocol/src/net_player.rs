use serde::{
    Serialize,
    Deserialize,
};
use simulation::player::Player;
use crate::net_attack;

#[derive(Serialize, Deserialize, Clone)]
pub struct NetPlayer {
    pub team_idx: usize,
    pub player_idx: usize,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub combo: u32,
    pub knockback_multiplier: f32,
    pub attacks: Vec<net_attack::NetAttack>,
    pub stunned: f32,
    pub invulnerable: f32,
    pub parry: f32,
    pub lives: u8,
}

#[must_use]
pub fn to_net(player: &Player, player_idx: usize) -> NetPlayer {
    NetPlayer {
        team_idx: player.team_idx,
        player_idx,
        pos: player.pos,
        vel: player.vel,
        combo: player.combo,
        knockback_multiplier: player.knockback_multiplier,
        attacks: player.attacks
            .iter()
            .map(|a| net_attack::to_net(a))
            .collect(),
        stunned: player.stunned,
        invulnerable: player.invulnerable_timer,
        parry: player.parry,
        lives: player.lives,
    }
}

pub fn from_net(player: &mut Player, net_player: NetPlayer) {
    player.pos = net_player.pos;
    player.vel = net_player.vel;
    player.lives = net_player.lives;
    player.combo = net_player.combo;
    player.knockback_multiplier = net_player.knockback_multiplier;
    player.attacks = net_player.attacks
        .iter()
        .map(|na| net_attack::from_net(na.clone()))
        .collect();
    player.stunned = net_player.stunned;
    player.invulnerable_timer = net_player.invulnerable;
    player.parry = net_player.parry;
}
