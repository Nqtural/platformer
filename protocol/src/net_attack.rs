use serde::{
    Serialize,
    Deserialize,
};
use simulation::attack::{
    Attack,
    AttackKind,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct NetAttack {
    pub timer: f32,
    pub owner_team: usize,
    pub owner_player: usize,
    pub kind: AttackKind,
    pub facing: [f32; 2],
    pub frame: usize,
}

#[must_use]
pub fn from_net(net: NetAttack) -> Attack {
    let properties = net.kind.properties();

    Attack {
        offset: properties.offset,
        size: properties.size,
        kind: net.kind,
        duration: properties.duration,
        timer: net.timer,
        owner_team: net.owner_team,
        owner_player: net.owner_player,
        facing: net.facing,
        stun: properties.stun,
        knockback_increase: properties.knockback_increase,
        frame: net.frame,
        frame_count: properties.frame_count,
    }
}

#[must_use]
pub fn to_net(attack: &Attack) -> NetAttack {
    NetAttack {
        timer: attack.timer,
        owner_team: attack.owner_team,
        owner_player: attack.owner_player,
        kind: attack.kind.clone(),
        facing: attack.facing,
        frame: attack.frame,
    }
}
