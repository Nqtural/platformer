use ggez::graphics::{
    Color,
    Rect,
};
use serde::{
    Deserialize,
    Serialize,
};
use crate::{
    attack::{
        Attack,
        AttackKind,
    },
    constants::{
        ACCELERATION,
        GRAVITY,
        MAX_SPEED,
        PLAYER_SIZE,
        RESISTANCE,
        RESPAWN_TIME,
        VIRTUAL_HEIGHT,
        WALL_SLIDE_SPEED,
    },
    input::PlayerInput,
    network::NetPlayer,
    team::Team,
    utils::approach_zero,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Player {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub lives: i32,
    pub name: String,
    pub stunned: f32,
    pub invulnerable_timer: f32,
    pub slow: f32,
    pub double_jumps: u8,
    pub knockback_multiplier: f32,
    pub attacks: Vec<Attack>,
    pub can_slam: bool,
    pub dash_cooldown: f32,
    pub normal_cooldown: f32,
    pub light_cooldown: f32,
    pub respawn_timer: f32,
    pub trail_timer: f32,
    pub facing: [f32; 2],
    pub input: PlayerInput,
    pub has_jumped: bool,
    pub start_pos: [f32; 2],
    pub color: Color,
}

impl Player {
    pub fn new(start_pos: [f32; 2], name: String, color: Color) -> Player {
        Player {
            pos: start_pos,
            vel: [0.0, 0.0],
            lives: 3,
            name,
            stunned: 0.0,
            invulnerable_timer: 0.0,
            slow: 0.0,
            double_jumps: 2,
            knockback_multiplier: 1.0,
            attacks: Vec::new(),
            can_slam: true,
            dash_cooldown: 0.0,
            normal_cooldown: 0.0,
            light_cooldown: 0.0,
            respawn_timer: 0.0,
            trail_timer: 0.0,
            facing: [0.0, 0.0],
            input: PlayerInput::new(),
            has_jumped: false,
            start_pos,
            color,
        }
    }

    pub fn to_net(&self, team_id: usize, player_id: usize) -> NetPlayer {
        NetPlayer {
            team_id,
            player_id,
            pos: self.pos,
            vel: self.vel,
            attacks: self.attacks
                .iter()
                .map(|a| a.to_net())
                .collect(),
            stunned: self.stunned,
            invulnerable: self.invulnerable_timer,
            lives: self.lives.max(0) as u8,
        }
    }

    pub fn update(&mut self, map: &Rect, enemy_team: &Team, normal_dt: f32) {
        self.update_cooldowns(normal_dt);
        if self.respawn_timer > 0.0 {
            return
        }

        let dt = if self.slow > normal_dt {
            normal_dt / 2.0
        } else {
            normal_dt
        };

        if self.is_doing_attack(&AttackKind::Slam)
        || self.is_doing_attack(&AttackKind::Dash) {
            self.trail_timer += dt;
        }

        self.update_position(map, enemy_team, dt);
        self.check_platform_collision(map, dt);
        self.check_for_death();

        if self.is_on_platform(map) {
            self.remove_slams();
            self.can_slam = false;
        }
    }

    fn update_cooldowns(&mut self, dt: f32) {
        let mut cooldowns = [
            &mut self.normal_cooldown,
            &mut self.light_cooldown,
            &mut self.stunned,
            &mut self.invulnerable_timer,
            &mut self.slow,
            &mut self.dash_cooldown,
            &mut self.respawn_timer,
        ];
        for cooldown in &mut cooldowns {
            if **cooldown > 0.0 {
                **cooldown -= dt;
            }
            **cooldown = (**cooldown).max(0.0);
        }

        self.update_attacks(dt);
    }

    fn update_position(&mut self, map: &Rect, enemy_team: &Team, dt: f32) {
        let old_pos = self.pos;

        self.pos[0] += self.vel[0] * dt;
        self.pos[1] += self.vel[1] * dt;

        // sweep test to prevent downward tunneling through platform
        if let Some(corrected_y) = self.sweep_down(old_pos[1], self.pos[1], map)
        {
            // snap onto platform
            self.pos[1] = corrected_y;
            self.vel[1] = 0.0;
        }

        // sweep test to prevent downward tunneling through an opponent
        if self.is_doing_attack(&AttackKind::Slam) {
            for opponent in enemy_team.players.iter() {
                if opponent.invulnerable_timer == 0.0
                    && let Some(corrected_y) = self.sweep_down(old_pos[1], self.pos[1], &opponent.get_rect()) {
                        // snap onto opponent
                        self.pos[1] = corrected_y;
                        self.vel[1] = 0.0;
                    }
            }
        }

        // apply friction
        self.vel[0] = approach_zero(self.vel[0], RESISTANCE * dt);
    }

    fn sweep_down(
        &self,
        old_y: f32,
        new_y: f32,
        object: &Rect,
    ) -> Option<f32> {
        if self.get_rect().x + PLAYER_SIZE > object.x && self.get_rect().x < object.x + object.w {
            // only downward motion matters for slam
            if new_y > old_y {
                let old_bottom = old_y + PLAYER_SIZE;
                let new_bottom = new_y + PLAYER_SIZE;

                // if player bottom crossed the object's top between frames:
                if old_bottom <= object.y && new_bottom >= object.y {
                    return Some(object.y - PLAYER_SIZE);
                }
            }
        }

        None
    }

    fn check_platform_collision(
        &mut self,
        map: &Rect,
        dt: f32,
    ) {
        let mut rect = self.get_rect();
        let mut on_wall_right = false;
        let mut on_wall_left = false;

        if rect.overlaps(map) {
            let overlap_x1 = map.x + map.w - rect.x;
            let overlap_x2 = rect.x + rect.w - map.x;
            let overlap_y1 = map.y + map.h - rect.y;
            let overlap_y2 = rect.y + rect.h - map.y;

            let resolve_x = overlap_x1.min(overlap_x2);
            let resolve_y = overlap_y1.min(overlap_y2);

            if resolve_x < resolve_y {
                if rect.x < map.x {
                    rect.x = map.x - rect.w;
                    on_wall_right = true;
                } else {
                    rect.x = map.x + map.w;
                    on_wall_left = true;
                }
                self.double_jumps = 2;
            } else if rect.y < map.y {
                rect.y = map.y - rect.h;
                self.vel[1] = 0.0;
                self.double_jumps = 2;
            } else {
                rect.y = map.y + map.h;
                if self.vel[1] < 0.0 {
                    self.vel[1] = 0.0;
                }
            }
        }

        let holding_toward_wall_right = on_wall_right && self.input.right();
        let holding_toward_wall_left = on_wall_left && self.input.left();
        let holding_wall = holding_toward_wall_right || holding_toward_wall_left;
        let on_platform = self.is_on_platform(map);

        if holding_wall && !on_platform && self.stunned == 0.0 {
            self.vel[1] = WALL_SLIDE_SPEED;
        } else {
            self.vel[1] += GRAVITY * dt;
        }

        self.pos[0] = rect.x;
        self.pos[1] = rect.y;
    }

    pub fn check_for_death(&mut self) {
        if self.pos[1] > VIRTUAL_HEIGHT {
            self.lives -= 1;
            self.double_jumps = 2;
            self.knockback_multiplier = 1.0;
            self.respawn_timer = RESPAWN_TIME;
            self.stunned = RESPAWN_TIME;
            self.invulnerable_timer = RESPAWN_TIME + 0.5;
            self.facing = [0.0, 0.0];
            self.vel = [0.0, 0.0];
            self.pos = self.start_pos;
        }
    }

    pub fn apply_input(
        &mut self,
        map: &Rect,
        team_idx: usize,
        player_idx: usize,
        dt: f32,
    ) {
        self.facing = [0.0, 0.0];
        if self.stunned > 0.0 || self.lives <= 0 {
            return;
        }

        if self.input.up() {
            self.facing[1] -= 1.0;
        }
        if self.input.jump() {
            if !self.has_jumped {
                if self.is_on_platform(map) {
                    self.vel[1] = -500.0;
                } 
                else if self.double_jumps > 0 {
                    self.vel[1] = -500.0;
                    self.double_jumps -= 1;
                }
                self.has_jumped = true;
            }
        } else {
            self.has_jumped = false;
        }
        if self.input.slam() {
            self.facing[1] += 1.0;
            if self.can_slam {
                self.attacks.push(
                    Attack::new(
                        AttackKind::Slam,
                        team_idx,
                        player_idx,
                        self.facing,
                    )
                );
                if self.vel[1] < MAX_SPEED[1] {
                    self.vel[1] += ACCELERATION * dt;
                }
            }
        } else {
            self.can_slam = true;
            self.remove_slams();
        }
        if self.input.left() {
            self.facing[0] -= 1.0;
            if self.vel[0] > -MAX_SPEED[0] {
                self.vel[0] -= ACCELERATION * dt;
            }
        }
        if self.input.right() {
            self.facing[0] += 1.0;
            if self.vel[0] < MAX_SPEED[0] {
                self.vel[0] += ACCELERATION * dt;
            }
        }
        if self.input.light() && self.light_cooldown <= 0.0 {
            self.attacks.push(
                Attack::new(
                    AttackKind::Light,
                    team_idx,
                    player_idx,
                    self.facing,
                )
            );
            self.light_cooldown = 2.0;
        }
        if self.input.normal() && self.normal_cooldown <= 0.0 {
            self.attacks.push(
                Attack::new(
                    AttackKind::Normal,
                    team_idx,
                    player_idx,
                    self.facing,
                )
            );
            self.normal_cooldown = 0.6;
        }
        if self.input.dash() && self.dash_cooldown <= 0.0 {
            let x = self.facing[0];
            let y = self.facing[1];
            let mag = (x * x + y * y).sqrt();

            let (nx, ny) = if mag > 0.0 {
                (x / mag, y / mag)
            } else {
                (0.0, 0.0)
            };

            let dash_speed = 1000.0;

            self.vel[0] = nx * dash_speed;
            self.vel[1] = ny * dash_speed;

            self.attacks.push(
                Attack::new(
                    AttackKind::Dash,
                    team_idx,
                    player_idx,
                    self.facing,
                )
            );

            self.dash_cooldown = 3.0;
        }
    }

    fn remove_slams(&mut self) {
        self.attacks.retain(|a| *a.kind() != AttackKind::Slam);
    }

    fn remove_dashes(&mut self) {
        self.attacks.retain(|a| *a.kind() != AttackKind::Dash);
    }

    fn update_attacks(&mut self, dt: f32) {
        for attack in &mut self.attacks {
            attack.update(dt);
        }
        self.attacks.retain(|atk| !atk.is_expired());
    }

    pub fn handle_attack_collisions(&mut self, atk: &Attack, attacker: &mut Player) {
        if self.invulnerable_timer > 0.0 // invulnerable
        || !atk.get_rect(attacker.pos).overlaps(&self.get_rect()) { // miss
            return;
        }

        self.attack(atk.kind(), attacker);
    }

    fn attack(&mut self, kind: &AttackKind, attacker: &mut Player) {
        match kind {
            AttackKind::Dash => {
                if self.is_doing_attack(&kind) {
                    for player in [self, attacker] {
                        player.vel[0] = player.vel[0].signum() * -50.0 * player.knockback_multiplier;
                        player.vel[1] = player.vel[1].signum() * -200.0 * player.knockback_multiplier;
                        player.stunned = 5.0;
                        player.knockback_multiplier += 0.01;
                        player.remove_dashes();
                    }
                } else {
                    self.vel[0] = attacker.vel[0] * self.knockback_multiplier;
                    self.vel[1] = attacker.vel[1] * self.knockback_multiplier;
                    self.stunned = 0.5;
                    self.remove_dashes();
                }
                attacker.vel[0] *= -0.5;
                attacker.vel[1] *= -0.5;
            }
            AttackKind::Light => {
                self.stunned = 1.0;
                self.invulnerable_timer = 0.1;
                self.knockback_multiplier += 0.01;
                self.remove_dashes();
                self.remove_slams();
            }
            AttackKind::Slam => {
                self.vel[1] = attacker.vel[1] * 1.5 * self.knockback_multiplier;
                self.stunned = 0.1;
                self.knockback_multiplier += 0.02;
                self.remove_dashes();
                self.remove_slams();

                attacker.vel[1] = -50.0;
                attacker.can_slam = false;
                attacker.remove_slams();
            }
            AttackKind::Normal => {
                self.stunned = 0.4;
                self.invulnerable_timer = 0.1;
                self.vel[0] = attacker.facing[0] * 400.0 * self.knockback_multiplier;
                self.vel[1] = attacker.facing[1] * 400.0 * self.knockback_multiplier;
                self.knockback_multiplier += 0.015;
                self.remove_dashes();
                self.remove_slams();
            }
        }
    }

    // GETTERS
    pub fn attacks(&self) -> &Vec<Attack> { &self.attacks }
    pub fn is_doing_attack(&self, kind: &AttackKind) -> bool {
        self.attacks.iter().any(|atk| atk.kind() == kind)
    }
    pub fn get_rect(&self) -> Rect {
        Rect::new(self.pos[0], self.pos[1], PLAYER_SIZE, PLAYER_SIZE)
    }
    fn is_on_platform(&self, map: &Rect) -> bool {
        let rect = self.get_rect();
        let player_bottom = rect.y + rect.h;
        let platform_top = map.y;

        (player_bottom - platform_top).abs() < 5.0 && rect.overlaps(map)
    }

    pub fn get_color(&self) -> Color {
        let color = if self.stunned > 0.0 {
            Color::new(
                (self.color.r + 0.4).min(1.0),
                (self.color.g + 0.4).min(1.0),
                (self.color.b + 0.4).min(1.0),
                1.0,
            )
        } else {
            self.color
        };

        Color::new(
            color.r,
            color.g,
            color.b,
            if self.invulnerable_timer > 0.0 { 0.5 } else { 1.0 }
        )
    }
}
