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

fn offset_and_size(kind: &AttackKind) -> (f32, f32) {
    match kind {
        AttackKind::Dash => {
            (0.0, PLAYER_SIZE)
        }
        AttackKind::Light => {
            (15.0, PLAYER_SIZE + 30.0)
        }
        AttackKind::Normal => {
            (15.0, PLAYER_SIZE + 30.0)
        }
        AttackKind::Slam => {
            (0.0, PLAYER_SIZE)
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
}

impl Attack {
    pub fn new(
        kind: AttackKind,
        owner_team: usize,
        owner_player: usize,
    ) -> Attack {
        let (offset, size) = offset_and_size(&kind);
        Attack {
            offset,
            size,
            kind,
            duration: 0.1,
            timer: 0.0,
            owner_team,
            owner_player,
        }
    }

    pub fn from_net(net: NetAttack) -> Self {
        let (offset, size) = offset_and_size(&net.kind);

        Attack {
            offset,
            size,
            kind: net.kind,
            duration: net.duration,
            timer: 0.0,
            owner_team: net.owner_team,
            owner_player: net.owner_player,
        }
    }

    pub fn to_net(&self) -> NetAttack {
        NetAttack {
            duration: self.duration,
            owner_team: self.owner_team,
            owner_player: self.owner_player,
            kind: self.kind.clone(),
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

    pub fn is_expired(&self) -> bool {
        self.timer >= self.duration
    }

    pub fn get_rect(&self, player_pos: [f32; 2], player_facing: [f32; 2]) -> Rect {
        Rect::new(
            self.x(player_pos, player_facing),
            self.y(player_pos, player_facing),
            self.size,
            self.size,
        )
    }

    pub fn x(&self, player_pos: [f32; 2], player_facing: [f32; 2]) -> f32 {
        player_pos[0] - self.offset + (self.offset * player_facing[0])
    }

    pub fn y(&self, player_pos: [f32; 2], player_facing: [f32; 2]) -> f32 {
        player_pos[1] - self.offset + (self.offset * player_facing[1])
    }
}
