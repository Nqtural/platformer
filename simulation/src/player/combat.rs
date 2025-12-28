use crate::attack::{Attack, AttackKind};
use glam::Vec2;

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
    pub fn expire_combo_if_needed(&mut self) {
        if self.combo > 0 && self.combo_timer == 0.0 {
            self.combo = 0;
        }
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

    pub fn update_attacks(&mut self, dt: f32) {
        for attack in &mut self.attacks {
            attack.update(dt);
        }
        self.attacks.retain(|atk| !atk.is_expired());
    }

    pub fn spawn_attack(
        &mut self,
        kind: AttackKind,
        team_idx: usize,
        player_idx: usize,
        facing: Vec2,
    ) {
        self.attacks.push(
            Attack::new(kind, team_idx, player_idx, facing)
        );
    }

    // GETTERS
    #[must_use]
    pub fn is_alive(&self) -> bool { self.lives > 0 }

    #[must_use]
    pub fn is_doing_attack(&self, kind: &AttackKind) -> bool {
        self.attacks.iter().any(|atk| atk.kind() == kind)
    }

    #[must_use]
    pub fn attacks(&self) -> &Vec<Attack> { &self.attacks }
}
