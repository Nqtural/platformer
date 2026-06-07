use serde::{Deserialize, Serialize};
use simulation::attack::{Attack, AttackKind};
use uuid::Uuid;
use wincode::{SchemaRead, SchemaWrite};

#[derive(Serialize, Deserialize, Clone, SchemaWrite, SchemaRead)]
pub struct NetAttack {
    pub timer: f32,
    pub owner: String,
    pub knockback: [f32; 2],
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
        knockback: net.knockback.into(),
        kind: net.kind,
        duration: properties.duration,
        timer: net.timer,
        owner: Uuid::parse_str(&net.owner).expect("Invalid UUID string"),
        facing: net.facing.into(),
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
        owner: attack.owner.to_string(),
        knockback: attack.knockback.into(),
        kind: attack.kind.clone(),
        facing: attack.facing.into(),
        frame: attack.frame,
    }
}
