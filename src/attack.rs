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
                frame_count: 1,
                stun: 0.5,
            },
            AttackKind::Light => AttackProperties {
                offset: 15.0,
                size: PLAYER_SIZE + 30.0,
                duration: 0.1,
                frame_count: 4,
                stun: 2.0,
            },
            AttackKind::Normal => AttackProperties {
                offset: 15.0,
                size: PLAYER_SIZE + 30.0,
                duration: 0.1,
                frame_count: 4,
                stun: 0.4,
            },
            AttackKind::Slam => AttackProperties {
                offset: 0.0,
                size: PLAYER_SIZE,
                duration: 99.9,
                frame_count: 1,
                stun: 0.1,
            },
        }
    }
}

pub struct AttackProperties {
    offset: f32,
    size: f32,
    duration: f32,
    frame_count: usize,
    stun: f32,
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
    facing: [f32; 2],
    stun: f32,

    // animation
    #[serde(skip)]
    #[serde(default)]
    frame: usize,

    #[serde(skip)]
    #[serde(default)]
    frame_count: usize,
}

impl Attack {
    #[must_use]
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
            stun: properties.stun,
            frame: 0,
            frame_count: properties.frame_count,
        }
    }

    #[must_use]
    pub fn from_net(net: NetAttack) -> Self {
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
            frame: net.frame,
            frame_count: properties.frame_count,
        }
    }

    #[must_use]
    pub fn to_net(&self) -> NetAttack {
        NetAttack {
            timer: self.timer,
            owner_team: self.owner_team,
            owner_player: self.owner_player,
            kind: self.kind.clone(),
            facing: self.facing,
            frame: self.frame,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.timer += dt;

        self.frame = ((self.timer / self.duration) * self.frame_count as f32)
            .floor() as usize % self.frame_count;
    }

    // GETTERS
    #[must_use]
    pub fn owner_team(&self) -> usize { self.owner_team }

    #[must_use]
    pub fn owner_player(&self) -> usize { self.owner_player }

    #[must_use]
    pub fn kind(&self) -> &AttackKind { &self.kind }

    #[must_use]
    pub fn facing(&self) -> [f32; 2] { self.facing }

    #[must_use]
    pub fn stun(&self) -> f32 { self.stun }

    #[must_use]
    pub fn is_expired(&self) -> bool { self.timer >= self.duration }

    #[must_use]
    pub fn get_rect(&self, player_pos: [f32; 2]) -> Rect {
        Rect::new(
            self.x(player_pos),
            self.y(player_pos),
            self.size,
            self.size,
        )
    }

    #[must_use]
    pub fn x(&self, player_pos: [f32; 2]) -> f32 {
        player_pos[0] - self.offset + (self.offset * self.facing[0])
    }

    #[must_use]
    pub fn y(&self, player_pos: [f32; 2]) -> f32 {
        player_pos[1] - self.offset + (self.offset * self.facing[1])
    }

    #[must_use]
    pub fn frame(&self) -> usize { self.frame }

    #[must_use]
    pub fn frame_count(&self) -> usize { self.kind.properties().frame_count }
}
