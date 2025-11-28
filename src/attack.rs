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

fn get_attack_properties(kind: &AttackKind) -> (f32, f32, f32) {
    match kind {
        AttackKind::Dash => {
            (0.0, PLAYER_SIZE, 0.3)
        }
        AttackKind::Light => {
            (15.0, PLAYER_SIZE + 30.0, 0.1)
        }
        AttackKind::Normal => {
            (15.0, PLAYER_SIZE + 30.0, 0.1)
        }
        AttackKind::Slam => {
            (0.0, PLAYER_SIZE, 99.9)
        }
    }
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
        let (offset, size, duration) = get_attack_properties(&kind);
        Attack {
            offset,
            size,
            kind,
            duration,
            timer: 0.0,
            owner_team,
            owner_player,
            facing,
        }
    }

    pub fn from_net(net: NetAttack) -> Self {
        let (offset, size, duration) = get_attack_properties(&net.kind);

        Attack {
            offset,
            size,
            kind: net.kind,
            duration,
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
