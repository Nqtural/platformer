use serde::{
    Deserialize,
    Serialize,
};
use crate::constants::PLAYER_SIZE;
use foundation::rect::Rect;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum AttackKind {
    Dash,
    Light,
    Normal,
    Slam,
}

pub struct AttackProperties {
    pub offset: f32,
    pub size: f32,
    pub duration: f32,
    pub frame_count: usize,
    pub stun: f32,
    pub knockback_increase: f32,
}

impl AttackKind {
    pub fn properties(&self) -> AttackProperties {
        match self {
            AttackKind::Dash => AttackProperties {
                offset: 0.0,
                size: PLAYER_SIZE,
                duration: 0.3,
                frame_count: 1,
                stun: 0.5,
                knockback_increase: 0.01,
            },
            AttackKind::Light => AttackProperties {
                offset: 15.0,
                size: PLAYER_SIZE + 30.0,
                duration: 0.1,
                frame_count: 4,
                stun: 2.0,
                knockback_increase: 0.01,
            },
            AttackKind::Normal => AttackProperties {
                offset: 15.0,
                size: PLAYER_SIZE + 30.0,
                duration: 0.1,
                frame_count: 4,
                stun: 0.4,
                knockback_increase: 0.015,
            },
            AttackKind::Slam => AttackProperties {
                offset: 5.0,
                size: PLAYER_SIZE + 10.0,
                duration: 99.9,
                frame_count: 1,
                stun: 0.1,
                knockback_increase: 0.02,
            },
        }
    }
}

#[derive(Clone)]
pub struct Attack {
    pub offset: f32,
    pub size: f32,
    pub kind: AttackKind,
    pub duration: f32,
    pub timer: f32,
    pub owner_team: usize,
    pub owner_player: usize,
    pub facing: [f32; 2],
    pub stun: f32,
    pub knockback_increase: f32,

    // animation
    pub frame: usize,
    pub frame_count: usize,
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
            knockback_increase: properties.knockback_increase,
            frame: 0,
            frame_count: properties.frame_count,
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
    pub fn knockback_increase(&self) -> f32 { self.knockback_increase }

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
