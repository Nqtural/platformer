use ggez::graphics::Rect;
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
    team::Team,
    utils::approach_zero,
};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
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
    pub slamming: bool,
    pub dashing: f32,
    pub dash_cooldown: f32,
    pub attack_cooldown: f32,
    pub respawn_timer: f32,
    pub trail_timer: f32,
    pub facing: [f32; 2],
    pub input: PlayerInput,
    pub has_jumped: bool,
}

impl Player {
    pub fn new(pos: [f32; 2], name: String) -> Player {
        Player {
            pos,
            vel: [0.0, 0.0],
            lives: 3,
            name,
            stunned: 0.0,
            invulnerable_timer: 0.0,
            slow: 0.0,
            double_jumps: 2,
            knockback_multiplier: 1.0,
            slamming: false,
            dashing: 0.0,
            dash_cooldown: 0.0,
            attack_cooldown: 0.0,
            respawn_timer: 0.0,
            trail_timer: 0.0,
            facing: [0.0, 0.0],
            input: PlayerInput::new(),
            has_jumped: false,
        }
    }

    pub fn get_rect(&self) -> Rect {
        Rect::new(self.pos[0], self.pos[1], PLAYER_SIZE, PLAYER_SIZE)
    }

    pub fn update_cooldowns(&mut self, dt: f32) {
        let mut cooldowns = [
            &mut self.attack_cooldown,
            &mut self.stunned,
            &mut self.invulnerable_timer,
            &mut self.slow,
            &mut self.dashing,
            &mut self.dash_cooldown,
            &mut self.respawn_timer,
        ];
        for cooldown in &mut cooldowns {
            if **cooldown > 0.0 {
                **cooldown -= dt;
            }
            **cooldown = (**cooldown).max(0.0);
        }
    }

    pub fn update_position(&mut self, map: &Rect, enemy_team: &Team, dt: f32) {
        let old_pos = self.pos;

        self.pos[0] += self.vel[0] * dt;
        self.pos[1] += self.vel[1] * dt;

        // Sweep test to prevent downward tunneling through platform
        if let Some(corrected_y) = self.sweep_down(old_pos[1], self.pos[1], map)
        {
            // Snap onto platform
            self.pos[1] = corrected_y;
            self.vel[1] = 0.0;
        }

        // Sweep test to prevent downward tunneling through an opponent
        if self.slamming {
            for opponent in enemy_team.players.iter() {
                if let Some(corrected_y) = self.sweep_down(old_pos[1], self.pos[1], &opponent.get_rect())
                {
                    // Snap onto opponent
                    self.pos[1] = corrected_y;
                    self.vel[1] = 0.0;
                }
            }
        }

        // Apply friction
        self.vel[0] = approach_zero(self.vel[0], RESISTANCE * dt);
    }

    fn sweep_down(
        &self,
        old_y: f32,
        new_y: f32,
        object: &Rect,
    ) -> Option<f32> {
        if self.get_rect().x + PLAYER_SIZE > object.x && self.get_rect().x < object.x + object.w {
            // Only downward motion matters for slam
            if new_y > old_y {
                let old_bottom = old_y + PLAYER_SIZE;
                let new_bottom = new_y + PLAYER_SIZE;

                // If player bottom crossed the object's top between frames:
                if old_bottom <= object.y && new_bottom >= object.y {
                    return Some(object.y - PLAYER_SIZE);
                }
            }
        }

        None
    }

    fn is_on_platform(&self, map: &Rect) -> bool {
        let rect = self.get_rect();
        let player_bottom = rect.y + rect.h;
        let platform_top = map.y;

        (player_bottom - platform_top).abs() < 5.0 && rect.overlaps(&map)
    }

    pub fn check_platform_collision(
        &mut self,
        map: &Rect,
        dt: f32,
    ) {
        let mut rect = self.get_rect();
        let mut on_wall_right = false;
        let mut on_wall_left = false;

        if rect.overlaps(&map) {
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
            } else {
                if rect.y < map.y {
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
        }

        let holding_toward_wall_right = on_wall_right && self.input.right();
        let holding_toward_wall_left = on_wall_left && self.input.left();
        let holding_wall = holding_toward_wall_right || holding_toward_wall_left;
        let on_platform = self.is_on_platform(&map);

        if holding_wall && !on_platform && self.stunned == 0.0 {
            self.vel[1] = WALL_SLIDE_SPEED;
        } else {
            self.vel[1] += GRAVITY * dt;
        }

        self.pos[0] = rect.x;
        self.pos[1] = rect.y;
    }

    pub fn check_for_death(&mut self, start_pos: [f32; 2]) {
        if self.pos[1] > VIRTUAL_HEIGHT {
            self.lives -= 1;
            self.double_jumps = 2;
            self.knockback_multiplier = 1.0;
            self.respawn_timer = RESPAWN_TIME;
            self.stunned = RESPAWN_TIME;
            self.invulnerable_timer = RESPAWN_TIME + 0.5;
            self.facing = [0.0, 0.0];
            self.vel = [0.0, 0.0];
            self.pos = start_pos;
        }
    }

    pub fn apply_input(
        &mut self,
        map: &Rect,
        team_idx: usize,
        player_idx: usize,
        dt: f32,
    ) -> Vec<Attack> {
        let mut new_attacks = Vec::new();
        self.facing = [0.0, 0.0];
        if self.stunned > 0.0 {
            return new_attacks;
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
        } else if self.has_jumped {
            self.has_jumped = false;
        }
        self.slamming = false;
        if self.input.slam() {
            self.facing[1] += 1.0;
            if !self.is_on_platform(&map) {
                self.slamming = true;
                if self.vel[1] < MAX_SPEED[1] {
                    self.vel[1] += ACCELERATION * dt;
                }
            }
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
        if self.attack_cooldown <= 0.0 {
            if self.input.light() {
                new_attacks.push(
                    Attack::new(
                        &self,
                        AttackKind::Light,
                        team_idx,
                        player_idx,
                    )
                );
                self.attack_cooldown = 0.3;
            }
            if self.input.normal() {
                new_attacks.push(
                    Attack::new(
                        &self,
                        AttackKind::Normal,
                        team_idx,
                        player_idx,
                    )
                );
                self.attack_cooldown = 0.3;
            }
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

            self.dashing = 0.3;
            self.dash_cooldown = 3.0;
        }

        new_attacks
    }
}
