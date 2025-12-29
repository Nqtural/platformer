use crate::constants::RESPAWN_TIME;
use crate::utils::tick_timers;

#[derive(Clone)]
pub struct PlayerStatus {
    pub stunned: f32,
    pub respawn_timer: f32,
    pub invulnerable_timer: f32,
    pub parry: f32,
    pub can_slam: bool,
}

impl Default for  PlayerStatus {
    fn default() -> Self {
        Self {
            stunned: RESPAWN_TIME,
            respawn_timer: RESPAWN_TIME,
            invulnerable_timer: 0.0,
            parry: 0.0,
            can_slam: true,
        }
    }
}

impl PlayerStatus {
    pub fn tick(&mut self, dt: f32) {
        tick_timers(&mut [
            &mut self.stunned,
            &mut self.respawn_timer,
            &mut self.invulnerable_timer,
            &mut self.parry,
        ], dt);
    }

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

    #[must_use]
    pub fn respawning(&self) -> bool { self.respawn_timer > 0.0 }

    #[must_use]
    pub fn stunned(&self) -> bool { self.stunned > 0.0 }

    #[must_use]
    pub fn invulnerable(&self) -> bool { self.invulnerable_timer > 0.0 }

    #[must_use]
    pub fn parrying(&self) -> bool { self.parry > 0.0 }
}
