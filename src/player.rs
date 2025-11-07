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
        WALL_SLIDE_SPEED,
    },
    input::PlayerInput,
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
    slamming: bool,
    pub dashing: f32,
    dash_cooldown: f32,
    attack_cooldown: f32,
    pub respawn_timer: f32,
    pub trail_timer: f32,
    pub facing: f32,
    pub input: PlayerInput,
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
            facing: 0.0,
            input: PlayerInput::new(),
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

    pub fn update_position(&mut self, dt: f32) {
        self.pos[0] += self.vel[0] * dt;
        self.pos[1] += self.vel[1] * dt;
        self.vel[0] = approach_zero(self.vel[0], RESISTANCE * dt);
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

        let holding_toward_wall_right = on_wall_right && self.input.right;
        let holding_toward_wall_left = on_wall_left && self.input.left;
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

    pub fn apply_input(&mut self, map: &Rect, team_idx: usize, player_idx: usize, dt: f32) -> Vec<Attack> {
        let mut new_attacks = Vec::new();
        if self.stunned > 0.0 {
            return new_attacks;
        }

        if self.input.up {
            if self.is_on_platform(map) {
                self.vel[1] = -500.0;
            } 
            else if self.double_jumps > 0 {
                self.vel[1] = -500.0;
                self.double_jumps -= 1;
            }
            self.input.up = false;
        }
        if self.input.left && self.vel[0] > -MAX_SPEED[0] {
            self.facing = -1.0;
            self.vel[0] -= ACCELERATION * dt;
        }
        if self.input.right && self.vel[0] < MAX_SPEED[0] {
            self.facing = 1.0;
            self.vel[0] += ACCELERATION * dt;
        }
        if self.attack_cooldown <= 0.0 {
            if self.input.light {
                new_attacks.push(
                    Attack::new(
                        &self,
                        AttackKind::Light,
                        team_idx,
                        player_idx,
                    )
                );
                self.slow = 0.5;
                self.attack_cooldown = 0.3;
                self.input.uppercut = true;
            }
            if self.input.uppercut {
                new_attacks.push(
                    Attack::new(
                        &self,
                        AttackKind::Uppercut,
                        team_idx,
                        player_idx,
                    )
                );
                self.slow = 0.5;
                self.attack_cooldown = 0.3;
                self.input.uppercut = false;
            }
        }
        if self.input.dash && self.dash_cooldown <= 0.0 {
            self.vel[0] = self.facing * 1000.0;
            self.dashing = 0.3;
            self.dash_cooldown = 3.0;
        }
        if self.input.slam && self.vel[1] < MAX_SPEED[1] {
            self.vel[1] += ACCELERATION * dt;
        }
        new_attacks
    }
}
