use glam::Vec2;
use foundation::math_helpers::approach;
use foundation::rect::Rect;
use crate::constants::{
    ACCELERATION,
    GRAVITY,
    MAX_SPEED,
    PLAYER_SIZE,
    RESISTANCE,
    VIRTUAL_HEIGHT,
    VIRTUAL_WIDTH,
    WALL_SLIDE_SPEED,
};
use crate::team::Team;
use super::{
    PlayerCombat,
    PlayerInput,
    PlayerStatus,
};

#[derive(Clone)]
pub struct PlayerPhysics {
    pub start_pos: Vec2,
    pub pos: Vec2,
    pub vel: Vec2,
    pub facing: Vec2,
    pub team_idx: usize,
    pub double_jumps: u8,
    pub has_jumped: bool,
}

impl PlayerPhysics {
    pub fn new(start_pos: Vec2, team_idx: usize) -> Self {
        Self {
            start_pos,
            pos: start_pos,
            vel: Vec2::new(0.0, 0.0),
            facing: get_facing_from_team(team_idx),
            team_idx,
            double_jumps: 2,
            has_jumped: false,
        }
    }

    pub fn tick(
        &mut self,
        dt: f32,
        combat: &PlayerCombat,
        input: &PlayerInput,
        status: &PlayerStatus,
        map: &Rect,
        enemy_team: &Team,
    ) {
        if status.respawning() { return; }

        self.update_facing(input);
        self.apply_movement_input(dt, input, map);
        self.update_position(dt, combat, map, enemy_team);
        self.check_platform_collision(dt, input, status, map);
    }

    fn update_position(
        &mut self,
        dt: f32,
        combat: &PlayerCombat,
        map: &Rect,
        enemy_team: &Team,
    ) {
        let old_pos = self.pos;

        self.pos += self.vel * dt;

        // sweep test to prevent downward tunneling through platform
        if let Some(corrected_y) = self.sweep_down(
            old_pos.y,
            self.pos.y,
            map
        ) {
            // snap onto platform
            self.pos.y = corrected_y;
            self.vel.y = 0.0;
        }

        // sweep test to prevent downward tunneling through an opponent
        if combat.is_slamming() {
            for opponent in &enemy_team.players {
                if !opponent.status.invulnerable()
                && let Some(corrected_y) = self.sweep_down(
                    old_pos[1],
                    self.pos[1],
                    &opponent.physics.get_rect()
                ) {
                    // snap onto opponent
                    self.pos.y = corrected_y;
                    self.vel.y = 0.0;
                }
            }
        }

        // apply friction
        self.vel.x = approach(self.vel.x, 0.0, RESISTANCE * dt);
    }

    fn sweep_down(
        &self,
        old_y: f32,
        new_y: f32,
        object: &Rect,
    ) -> Option<f32> {
        if self.get_rect().x + PLAYER_SIZE > object.x
        && self.get_rect().x < object.x + object.w {
            // only downward motion matters for slam
            if new_y > old_y {
                let old_bottom = old_y + PLAYER_SIZE;
                let new_bottom = new_y + PLAYER_SIZE;

                // if player bottom crossed the
                // object's top between frames:
                if old_bottom <= object.y && new_bottom >= object.y {
                    return Some(object.y - PLAYER_SIZE);
                }
            }
        }

        None
    }

    pub fn should_lose_life(&self) -> bool {
        self.pos[1] > VIRTUAL_HEIGHT
        || self.pos[0] > VIRTUAL_WIDTH
        || self.pos[0] < 0.0
    }

    pub fn is_on_platform(&self, platform: &Rect) -> bool {
        let player = self.get_rect();

        let player_bottom = player.y + player.h;
        let platform_top = platform.y;

        // Check horizontal overlap (X)
        let horizontal_overlap =
        player.x < platform.x + platform.w &&
        player.x + player.w > platform.x;

        // Check if player is on top (Y)
        let on_top =
        player_bottom <= platform_top + 5.0 &&  // within tolerance above top
        player_bottom >= platform_top - 5.0;    // avoid floating-point misses

        horizontal_overlap && on_top
    }

    fn check_platform_collision(
        &mut self,
        dt: f32,
        input: &PlayerInput,
        status: &PlayerStatus,
        map: &Rect,
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
                self.vel.y = 0.0;
                self.double_jumps = 2;
            } else {
                rect.y = map.y + map.h;
                if self.vel.y < 0.0 {
                    self.vel.y = 0.0;
                }
            }
        }

        let holding_toward_wall_right = on_wall_right && input.right();
        let holding_toward_wall_left = on_wall_left && input.left();
        let holding_wall = holding_toward_wall_right || holding_toward_wall_left;
        let on_platform = self.is_on_platform(map);

        if on_platform {
            self.double_jumps = 2;
        }

        if holding_wall && !on_platform && !status.stunned() {
            self.vel.y = WALL_SLIDE_SPEED;
        } else {
            self.vel.y += GRAVITY * dt;
        }

        self.pos.x = rect.x;
        self.pos.y = rect.y;
    }

    fn update_facing(&mut self, input: &PlayerInput) {
        self.facing = Vec2::new(0.0, 0.0);
        if input.left() { self.facing.x -= 1.0; }
        if input.right() { self.facing.x += 1.0; }
        if input.up() { self.facing.y -= 1.0; }
        if input.slam() { self.facing.y += 1.0; }
    }

    fn apply_movement_input(
        &mut self,
        dt: f32,
        input: &PlayerInput,
        map: &Rect,
    ) {
        if self.facing.x != 0.0 && self.vel.x.abs() < MAX_SPEED[0] {
            self.vel.x += ACCELERATION * dt * self.facing.x;
        }

        if input.jump() && !self.has_jumped {
            self.has_jumped = true;
            if self.is_on_platform(map) || self.double_jumps > 0 {
                self.vel.y = -500.0;
                if !self.is_on_platform(map) {
                    self.double_jumps -= 1;
                }
            }
        } else if !input.jump() {
            self.has_jumped = false;
        }
    }

    pub fn slam(&mut self, dt: f32) {
        if self.vel.y < MAX_SPEED[1] {
            self.vel.y += ACCELERATION * dt;
        }
    }

    pub fn dash(&mut self) {
        self.vel = self.facing.normalize_or_zero() * 1000.0;
    }

    pub fn apply_knockback(&mut self, force: Vec2, multiplier: f32) {
        self.vel += force * multiplier;
    }

    pub fn apply_dash_collision(&mut self, multiplier: f32) {
        self.vel.x = self.vel.x.signum() * -50.0 * multiplier;
        self.vel.y = self.vel.y.signum() * -200.0 * multiplier;
    }

    pub fn set_parried_vel(&mut self) {
        self.vel *= 0.5;
    }

    pub fn get_slammed(&mut self, force: f32) {
        self.vel.y = force;
    }

    pub fn reset(&mut self) {
        self.pos = self.start_pos;
        self.vel = Vec2::new(0.0, 0.0);
        self.facing = get_facing_from_team(self.team_idx);
        self.double_jumps = 2;
    }

    #[must_use]
    pub fn get_rect(&self) -> Rect {
        Rect::new(self.pos.x, self.pos.y, PLAYER_SIZE, PLAYER_SIZE)
    }
}

fn get_facing_from_team(team_idx: usize) -> Vec2 {
    Vec2::new(if team_idx == 0 { 1.0 } else { -1.0 }, 0.0)
}
