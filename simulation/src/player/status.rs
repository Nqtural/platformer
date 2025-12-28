use crate::constants::RESPAWN_TIME;

#[derive(Clone)]
pub struct PlayerStatus {
    pub stunned: f32,
    pub respawn_timer: f32,
    pub invulnerable_timer: f32,
    pub parry: f32,
    pub can_slam: bool,
    pub has_jumped: bool,
}

impl Default for  PlayerStatus {
    fn default() -> Self {
        Self {
            stunned: RESPAWN_TIME,
            respawn_timer: RESPAWN_TIME,
            invulnerable_timer: 0.0,
            parry: 0.0,
            can_slam: true,
            has_jumped: false,
        }
    }
}

impl PlayerStatus {
    pub fn touch_platform(&mut self) {
        self.can_slam = false;
    }

    pub fn stun(&mut self, stun: f32) {
        self.stunned = stun;
    }

    pub fn activate_parry(&mut self) {
        self.parry = 0.5;
    }

    pub fn lose_life(&mut self) {
        self.respawn_timer = RESPAWN_TIME;
        self.stunned = RESPAWN_TIME;
        self.invulnerable_timer = RESPAWN_TIME + 0.5;
    }

    pub fn handle_jump_input(&mut self, jump_input: bool) {
        if jump_input && !self.has_jumped {
            self.has_jumped = true;
        } else if !jump_input {
            self.has_jumped = false;
        }
    }

    // GETTERS
    #[must_use]
    pub fn respawning(&self) -> bool { self.respawn_timer > 0.0 }

    #[must_use]
    pub fn stunned(&self) -> bool { self.stunned > 0.0 }

    #[must_use]
    pub fn invulnerable(&self) -> bool { self.invulnerable_timer > 0.0 }

    #[must_use]
    pub fn parrying(&self) -> bool { self.parry > 0.0 }
}
