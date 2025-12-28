use ggez::input::keyboard::KeyCode;
use std::collections::HashSet;
use crate::{
    attack::{
        Attack,
        AttackKind,
    },
    constants::PLAYER_SIZE,
    team::Team,
    utils::get_combo_multiplier,
};
use foundation::color::Color;
use foundation::rect::Rect;
use super::{
    PlayerCombat,
    PlayerCooldowns,
    PlayerIdentity,
    PlayerInput,
    PlayerPhysics,
    PlayerStatus,
    PlayerVisuals,
};

#[derive(Clone)]
pub struct Player {
    pub combat: PlayerCombat,
    pub cooldowns: PlayerCooldowns,
    pub identity: PlayerIdentity,
    pub physics: PlayerPhysics,
    pub status: PlayerStatus,
    pub visuals: PlayerVisuals,
    pub double_jumps: u8,
    pub input: PlayerInput,
}

impl Player {
    #[must_use]
    pub fn new(
        start_pos: [f32; 2],
        name: String,
        color: Color,
        team_idx: usize,
    ) -> Self {
        Self {
            combat: PlayerCombat::default(),
            cooldowns: PlayerCooldowns::default(),
            identity: PlayerIdentity::new(
                name,
                color,
            ),
            physics: PlayerPhysics::new(
                start_pos.into(),
                team_idx
            ),
            status: PlayerStatus::default(),
            visuals: PlayerVisuals::default(),
            double_jumps: 2,
            input: PlayerInput::new(),
        }
    }

    pub fn update(
        &mut self,
        map: &Rect,
        player_idx: usize,
        enemy_team: &Team,
        dt: f32,
    ) {
        self.physics.update_facing(&self.input);
        self.update_cooldowns(dt);

        if self.status.respawning() { return; }

        self.combat.expire_combo_if_needed();

        let slamming = self.combat.is_doing_attack(&AttackKind::Slam);
        let dashing = self.combat.is_doing_attack(&AttackKind::Dash);

        if slamming || dashing {
            self.visuals.update_trail(
                self.physics.get_rect(),
                self.identity.color().clone(),
                dt,
            );
        }

        self.physics.update_position(map, enemy_team, slamming, dt);
        self.physics.check_platform_collision(
            map,
            &self.input,
            &mut self.double_jumps,
            self.status.stunned(),
            dt,
        );

        if self.physics.is_on_platform(map) {
            self.combat.remove_slams();
            self.status.touch_platform();
            self.double_jumps = 2;
        }


        if !self.status.stunned() && self.combat.is_alive() {
            self.apply_input(map, player_idx, dt);
        }

        if self.physics.should_lose_life() {
            self.lose_life();
        }
    }

    pub fn apply_input(
        &mut self,
        map: &Rect,
        player_idx: usize,
        dt: f32,
    ) {
        self.physics.apply_movement_input(
            map,
            &self.input,
            &mut self.double_jumps,
            self.status.has_jumped,
            dt,
        );
        self.status.handle_jump_input(self.input.jump());

        if self.input.slam() {
            if self.status.can_slam {
                self.physics.slam(dt);
                self.combat.spawn_attack(
                    AttackKind::Slam,
                    self.physics.team_idx,
                    player_idx,
                    self.physics.facing,
                );
            }
        } else {
            self.status.can_slam = true;
            self.combat.remove_slams();
        }
        if self.input.light() && self.cooldowns.can_light() {
            self.combat.spawn_attack(
                AttackKind::Light,
                self.physics.team_idx,
                player_idx,
                self.physics.facing,
            );
            self.cooldowns.activate_light();
        }
        if self.input.normal() && self.cooldowns.can_normal() {
            self.combat.spawn_attack(
                AttackKind::Normal,
                self.physics.team_idx,
                player_idx,
                self.physics.facing,
            );
            self.cooldowns.activate_normal();
        }
        if self.input.dash()
        && self.cooldowns.can_dash()
        && !self.status.parrying() {
            self.physics.dash();
            self.combat.spawn_attack(
                AttackKind::Dash,
                self.physics.team_idx,
                player_idx,
                self.physics.facing,
            );

            self.cooldowns.activate_dash();
        }
        if self.input.parry()
        && self.physics.is_on_platform(map)
        && self.cooldowns.can_parry()
        && !self.combat.is_doing_attack(&AttackKind::Dash)
        && !self.combat.is_doing_attack(&AttackKind::Slam) {
            self.cooldowns.activate_parry();
            self.status.activate_parry();
        }
    }

    fn update_cooldowns(&mut self, dt: f32) {
        let mut cooldowns = [
            &mut self.combat.combo_timer,
            &mut self.cooldowns.normal,
            &mut self.cooldowns.light,
            &mut self.cooldowns.parry,
            &mut self.cooldowns.dash,
            &mut self.status.parry,
            &mut self.status.stunned,
            &mut self.status.invulnerable_timer,
            &mut self.status.respawn_timer,
        ];
        for cooldown in &mut cooldowns {
            if **cooldown > 0.0 {
                **cooldown -= dt;
            }
            **cooldown = (**cooldown).max(0.0);
        }

        self.combat.update_attacks(dt);
    }

    pub fn lose_life(&mut self) {
        self.double_jumps = 2;
        self.combat.lose_life();
        self.physics.reset();
        self.status.lose_life();
    }

    pub fn attack(&mut self, atk: &Attack, attacker: &mut Player) {
        if self.status.invulnerable() { return; }

        if self.status.parrying() {
            // get dash ability back when successfully parrying
            self.cooldowns.dash = 0.0;

            // reset combo
            self.combat.combo = 0;

            // stun attacker with own attack's stun
            attacker.status.stunned = atk.stun();

            attacker.physics.get_parried_vel();

            return;
        }

        match atk.kind() {
            AttackKind::Dash => {
                if self.combat.is_doing_attack(atk.kind()) {
                    for player in [&mut *self, attacker] {
                        player.physics.apply_dash_collision(player.combat.knockback_multiplier);
                        player.status.stun(atk.stun());
                        player.combat.knockback_multiplier += atk.knockback_increase();
                        player.combat.remove_dashes();
                    }
                } else {
                    self.physics.vel = attacker.physics.vel * self.combat.knockback_multiplier;
                }
                attacker.physics.vel *= -0.5;
            }
            AttackKind::Light => {
                // if player is in a combo, this
                // attack is used as a finisher
                if self.combat.combo > 0 {
                    // overwrite default attack stun
                    self.status.stun(0.5);

                    // launch player
                    self.physics.vel = attacker.physics.facing.normalize_or_zero()
                        * 600.0
                        * self.combat.knockback_multiplier
                        * get_combo_multiplier(self.combat.combo);

                    // apply knockback multiplier boost for combo
                    self.combat.knockback_multiplier += 0.1 * get_combo_multiplier(self.combat.combo);

                    // apply invulnerability because generic attack
                    // traits are not applied due to early return
                    self.status.invulnerable_timer = 0.3;

                    return;
                }
            }
            AttackKind::Slam => {
                // must be above player and moving downwards
                if attacker.physics.pos.y + PLAYER_SIZE > self.physics.pos.y
                || attacker.physics.vel.y <= 0.0 { return; }

                self.physics.get_slammed(attacker.physics.vel.y * self.combat.knockback_multiplier);

                attacker.physics.vel.y = -50.0;
                attacker.status.can_slam = false;
                attacker.combat.remove_slams();
            }
            AttackKind::Normal => {
                self.physics.vel = attacker.physics.facing * 450.0;

                attacker.cooldowns.normal_hit();
            }
        }

        // if not returned by this point,
        // apply generic attack traits
        self.combat.remove_dashes();
        self.combat.remove_slams();

        self.status.stun(atk.stun());
        self.combat.knockback_multiplier += atk.knockback_increase();
        self.status.invulnerable_timer = 0.3;

        self.combat.combo += 1;
        self.combat.combo_timer = 1.0;
    }

    pub fn update_input(&mut self, pressed: &HashSet<KeyCode>) {
        self.input.update(pressed);
    }

    pub fn set_input(&mut self, input: PlayerInput) {
        self.input = input;
    }

    // GETTERS
    #[must_use]
    pub fn get_color(&self) -> Color {
        if self.status.stunned() {
            let color = self.identity.color();
            Color::new(
                (color.r + 0.4).min(1.0),
                (color.g + 0.4).min(1.0),
                (color.b + 0.4).min(1.0),
                1.0,
            )
        } else {
            self.identity.color().clone()
        }
    }

    #[must_use]
    pub fn name(&self) -> String { self.identity.name().to_string() }

    #[must_use]
    pub fn get_input(&self) -> &PlayerInput { &self.input }
}
