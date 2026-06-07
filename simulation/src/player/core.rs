use super::PlayerInput;
use super::{PlayerCombat, PlayerCooldowns, PlayerPhysics, PlayerStatus};
use crate::constants::PLAYER_SIZE;
use crate::{
    attack::{Attack, AttackKind},
    utils::get_combo_multiplier,
};
use foundation::rect::Rect;
use glam::Vec2;
use uuid::Uuid;

pub enum HitResult {
    Hit,
    DashClash,
    Parried,
    Ignored,
}

#[derive(Clone)]
pub struct Player {
    pub combat: PlayerCombat,
    pub cooldowns: PlayerCooldowns,
    pub physics: PlayerPhysics,
    pub status: PlayerStatus,
    pub input: PlayerInput,
}

impl Player {
    #[must_use]
    pub fn new(start_pos: [f32; 2], team_idx: usize) -> Self {
        Self {
            combat: PlayerCombat::default(),
            cooldowns: PlayerCooldowns::default(),
            physics: PlayerPhysics::new(start_pos.into(), team_idx),
            status: PlayerStatus::default(),
            input: PlayerInput::new(),
        }
    }

    pub fn update(
        &mut self,
        map: &Rect,
        player_id: Uuid,
        enemies: &[(Rect, bool)], // hitbox, invulnerable
        dt: f32,
    ) {
        self.tick(dt, map, enemies);

        if self.status.respawning() {
            return;
        }

        if self.physics.is_on_platform(map) {
            self.combat.remove_slams();
            self.status.touch_platform();
        }

        if !self.status.stunned() && self.combat.is_alive() {
            self.apply_input(map, player_id, dt);
        }

        if self.physics.should_lose_life() {
            self.lose_life();
        }
    }

    pub fn apply_input(&mut self, map: &Rect, player_id: Uuid, dt: f32) {
        let mut kind: Option<AttackKind> = None;

        if self.input.slam() && self.status.can_slam {
            self.physics.slam(dt);
            kind = Some(AttackKind::Slam);
        } else {
            self.status.can_slam = true;
            self.combat.remove_slams();
        }

        if self.input.light() && self.cooldowns.can_light() {
            kind = Some(AttackKind::Light);
            self.cooldowns.activate_light();
        }

        if self.input.normal() && self.cooldowns.can_normal() {
            kind = Some(AttackKind::Normal);
            self.cooldowns.activate_normal();
        }

        if self.input.dash() && self.cooldowns.can_dash() && !self.status.parrying() {
            self.physics.dash();
            kind = Some(AttackKind::Dash);
            self.cooldowns.activate_dash();
        }

        if self.input.parry()
            && self.physics.is_on_platform(map)
            && self.cooldowns.can_parry()
            && !self.combat.is_dashing()
            && !self.combat.is_slamming()
        {
            self.status.activate_parry();
            self.cooldowns.activate_parry();
        }

        if let Some(kind) = kind {
            self.combat.spawn_attack(kind, &self.physics, player_id);
        }
    }

    fn tick(
        &mut self,
        dt: f32,
        map: &Rect,
        enemies: &[(Rect, bool)], // hitbox, invulnerable
    ) {
        self.combat.tick(dt);
        self.cooldowns.tick(dt);
        self.status.tick(dt);
        self.physics
            .tick(dt, &self.combat, &self.input, &self.status, map, enemies);
    }

    pub fn lose_life(&mut self) {
        self.combat.lose_life();
        self.physics.reset();
        self.status.lose_life();
    }

    pub fn apply_hit(&mut self, atk: &Attack, attacker_pos: Vec2, attacker_vel: Vec2) -> HitResult {
        if self.status.invulnerable() {
            return HitResult::Ignored;
        }

        if self.status.parrying() {
            self.cooldowns.dash = 0.0;
            self.combat.combo = 0;

            return HitResult::Parried;
        }

        match atk.kind() {
            AttackKind::Dash => {
                self.status.stun(atk.stun());
                self.combat.remove_dashes();
                self.combat.remove_slams();
                if self.combat.is_dashing() {
                    self.physics
                        .apply_dash_collision(self.combat.knockback_multiplier);
                    self.combat.knockback_multiplier += atk.knockback_increase();
                    return HitResult::DashClash;
                } else {
                    self.physics.vel = attacker_vel * self.combat.knockback_multiplier;
                }
            }
            AttackKind::Light => {
                // if player is in a combo, this
                // attack is used as a finisher
                if self.combat.combo > 0 {
                    // overwrite default attack stun
                    self.status.stun(0.5);

                    // launch player
                    self.physics.vel = atk.facing.normalize_or_zero()
                        * 600.0
                        * self.combat.knockback_multiplier
                        * get_combo_multiplier(self.combat.combo);

                    // apply knockback multiplier boost for combo
                    self.combat.knockback_multiplier +=
                        0.1 * get_combo_multiplier(self.combat.combo);

                    // apply invulnerability because generic attack
                    // traits are not applied due to early return
                    self.status.invulnerable_timer = 0.3;

                    return HitResult::Hit;
                }
            }
            AttackKind::Slam => {
                // attacker has to be above victim for slam
                if attacker_pos.y + PLAYER_SIZE < self.physics.pos.y {
                    // knockback is only vertical
                    self.physics
                        .get_slammed(atk.knockback[1] * self.combat.knockback_multiplier);
                } else {
                    return HitResult::Ignored;
                }
            }
            AttackKind::Normal => {
                self.physics.vel = atk.facing * 450.0;
            }
        }
        self.apply_generic_attack_traits(atk);
        HitResult::Hit
    }

    pub fn apply_hit_effects(&mut self, attack: &Attack) {
        match attack.kind() {
            AttackKind::Dash => {
                self.physics.vel *= -0.5;
            }
            AttackKind::Light => {}
            AttackKind::Slam => {
                self.physics.vel.y = -50.0;
                self.status.can_slam = false;
                self.combat.remove_slams();
            }
            AttackKind::Normal => {
                self.cooldowns.normal_hit();
            }
        }
    }

    pub fn apply_dash_clash_effects(&mut self, atk: &Attack) {
        self.physics
            .apply_dash_collision(self.combat.knockback_multiplier);

        self.status.stun(atk.stun());
        self.combat.knockback_multiplier += atk.knockback_increase();
        self.combat.remove_dashes();
    }

    pub fn apply_parry_penalty(&mut self, atk: &Attack) {
        self.status.stunned = atk.stun();
        self.physics.set_parried_vel();
    }

    fn apply_generic_attack_traits(&mut self, atk: &Attack) {
        self.combat.remove_dashes();
        self.combat.remove_slams();

        self.status.stun(atk.stun());
        self.combat.knockback_multiplier += atk.knockback_increase();
        self.status.invulnerable_timer = 0.3;

        self.combat.increase_combo();
    }

    #[must_use]
    pub fn get_input(&self) -> &PlayerInput {
        &self.input
    }
}
