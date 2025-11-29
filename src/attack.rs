use ggez::graphics::Rect;
use serde::{
    Deserialize,
    Serialize,
};
use crate::{
    constants::PLAYER_SIZE,
    network::NetAttack,
};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum AttackKind {
    Dash,
    Light,
    Normal,
    Slam,
}

impl AttackKind {
    fn properties(&self) -> AttackProperties {
        match self {
            AttackKind::Dash => AttackProperties {
                offset: 0.0,
                size: PLAYER_SIZE,
                duration: 0.3,
            },
            AttackKind::Light => AttackProperties {
                offset: 15.0,
                size: PLAYER_SIZE + 30.0,
                duration: 0.1,
            },
            AttackKind::Normal => AttackProperties {
                offset: 15.0,
                size: PLAYER_SIZE + 30.0,
                duration: 0.1,
            },
            AttackKind::Slam => AttackProperties {
                offset: 0.0,
                size: PLAYER_SIZE,
                duration: 99.9,
            },
        }
    }
}

pub struct AttackProperties {
    offset: f32,
    size: f32,
    duration: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Attack {
    offset: f32,
    size: f32,
    kind: AttackKind,
    duration: f32,
    timer: f32,
    owner_team: usize,
    owner_player: usize,
    facing: [f32; 2]
}

impl Attack {
    pub fn new(
        kind: AttackKind,
        owner_team: usize,
        owner_player: usize,
        facing: [f32; 2]
    ) -> Attack {
        let properties = kind.properties();
        Attack {
            offset: properties.offset,
            size: properties.size,
            kind,
            duration: properties.duration,
            timer: 0.0,
            owner_team,
            owner_player,
            facing,
        }
    }

    pub fn from_net(net: NetAttack) -> Self {
        let properties = net.kind.properties();

        Attack {
            offset: properties.offset,
            size: properties.size,
            kind: net.kind,
            duration: properties.duration,
            timer: 0.0,
            owner_team: net.owner_team,
            owner_player: net.owner_player,
            facing: net.facing,
        }
    }

    pub fn to_net(&self) -> NetAttack {
        NetAttack {
            owner_team: self.owner_team,
            owner_player: self.owner_player,
            kind: self.kind.clone(),
            facing: self.facing,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.timer += dt;
    }

    pub fn owner_team(&self) -> usize {
        self.owner_team
    }

    pub fn owner_player(&self) -> usize {
        self.owner_player
    }

    pub fn kind(&self) -> &AttackKind {
        &self.kind
    }

    pub fn facing(&self) -> [f32; 2] {
        self.facing
    }

    pub fn is_expired(&self) -> bool {
        self.timer >= self.duration
    }

    pub fn get_rect(&self, player_pos: [f32; 2]) -> Rect {
        Rect::new(
            self.x(player_pos),
            self.y(player_pos),
            self.size,
            self.size,
        )
    }

    pub fn x(&self, player_pos: [f32; 2]) -> f32 {
        player_pos[0] - self.offset + (self.offset * self.facing[0])
    }

    pub fn y(&self, player_pos: [f32; 2]) -> f32 {
        player_pos[1] - self.offset + (self.offset * self.facing[1])
    }
}
