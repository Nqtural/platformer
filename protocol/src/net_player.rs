use crate::net_attack;
use serde::{Deserialize, Serialize};
use simulation::Player;
use wincode::{SchemaRead, SchemaWrite};

#[derive(Serialize, Deserialize, Clone, SchemaWrite, SchemaRead)]
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
        team_idx: player.physics.team_idx,
        player_idx,
        pos: player.physics.pos.into(),
        vel: player.physics.vel.into(),
        combo: player.combat.combo,
        knockback_multiplier: player.combat.knockback_multiplier,
        attacks: player
            .combat
            .attacks()
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
    player.physics.pos = net_player.pos.into();
    player.physics.vel = net_player.vel.into();
    player.combat.lives = net_player.lives;
    player.combat.combo = net_player.combo;
    player.combat.knockback_multiplier = net_player.knockback_multiplier;
    player.combat.attacks = net_player
        .attacks
        .iter()
        .map(|na| net_attack::from_net(na.clone()))
        .collect();
    player.status.stunned = net_player.stunned;
    player.status.invulnerable_timer = net_player.invulnerable;
    player.status.parry = net_player.parry;
}
