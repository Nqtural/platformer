use glam::Vec2;
use serde::{
    Serialize,
    Deserialize,
};
use simulation::Player;
use crate::net_attack;

#[derive(Serialize, Deserialize, Clone)]
pub struct NetPlayer {
    pub team_idx: usize,
    pub player_idx: usize,
    pub pos: Vec2,
    pub vel: Vec2,
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
        team_idx: player.physics.team_idx,
        player_idx,
        pos: player.physics.pos,
        vel: player.physics.vel,
        combo: player.combat.combo,
        knockback_multiplier: player.combat.knockback_multiplier,
        attacks: player.combat.attacks()
            .iter()
            .map(net_attack::to_net)
            .collect(),
        stunned: player.status.stunned,
        invulnerable: player.status.invulnerable_timer,
        parry: player.status.parry,
        lives: player.combat.lives,
    }
}

pub fn from_net(player: &mut Player, net_player: &NetPlayer) {
    player.physics.pos = net_player.pos;
    player.physics.vel = net_player.vel;
    player.combat.lives = net_player.lives;
    player.combat.combo = net_player.combo;
    player.combat.knockback_multiplier = net_player.knockback_multiplier;
    player.combat.attacks = net_player.attacks
        .iter()
        .map(|na| net_attack::from_net(na.clone()))
        .collect();
    player.status.stunned = net_player.stunned;
    player.status.invulnerable_timer = net_player.invulnerable;
    player.status.parry = net_player.parry;
}
