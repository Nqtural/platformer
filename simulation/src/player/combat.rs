use crate::attack::{Attack, AttackKind};
use crate::utils::tick_timers;
use super::PlayerPhysics;

#[derive(Clone)]
pub struct PlayerCombat {
    pub lives: u8,
    pub combo: u32,
    pub combo_timer: f32,
    pub knockback_multiplier: f32,
    pub attacks: Vec<Attack>,
}

impl Default for  PlayerCombat {
    fn default() -> Self {
        Self {
            lives: 3,
            combo: 0,
            combo_timer: 0.0,
            knockback_multiplier: 1.0,
            attacks: Vec::default(),
        }
    }
}

impl PlayerCombat {
    pub fn tick(&mut self, dt: f32) {
        tick_timers(&mut [
            &mut self.combo_timer,
        ], dt);

        // reset combo if needed
        if self.combo > 0 && self.combo_timer == 0.0 {
            self.combo = 0;
        }

        // update attacks
        for attack in &mut self.attacks {
            attack.update(dt);
        }
        self.attacks.retain(|atk| !atk.is_expired());
    }

    pub fn lose_life(&mut self) {
        self.lives -= 1;
        self.combo = 0;
        self.knockback_multiplier = 1.0;
    }

    pub fn remove_slams(&mut self) {
        self.attacks.retain(|a| *a.kind() != AttackKind::Slam);
    }

    pub fn remove_dashes(&mut self) {
        self.attacks.retain(|a| *a.kind() != AttackKind::Dash);
    }

    pub fn increase_combo(&mut self) {
        self.combo += 1;
        self.combo_timer = 1.0;
    }

    pub fn spawn_attack(
        &mut self,
        kind: AttackKind,
        physics: &PlayerPhysics,
        player_idx: usize,
    ) {
        self.attacks.push(
            Attack::new(kind, physics.team_idx, player_idx, physics.facing)
        );
    }

    #[must_use]
    fn is_doing_attack(&self, kind: &AttackKind) -> bool {
        self.attacks.iter().any(|atk| atk.kind() == kind)
    }

    #[must_use]
    pub fn is_alive(&self) -> bool { self.lives > 0 }

    #[must_use]
    pub fn is_slamming(&self) -> bool {
        self.is_doing_attack(&AttackKind::Slam)
    }
    #[must_use]
    pub fn is_dashing(&self) -> bool {
        self.is_doing_attack(&AttackKind::Dash)
    }

    pub fn trail_active(&self) -> bool {
        self.is_slamming() || self.is_dashing()
    }

    #[must_use]
    pub fn attacks(&self) -> &Vec<Attack> { &self.attacks }
}
