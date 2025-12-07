use ggez::{
    graphics::Color,
    input::keyboard::KeyCode,
};
use std::collections::HashSet;
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
        VIRTUAL_WIDTH,
        WALL_SLIDE_SPEED,
    },
    input::PlayerInput,
    network::NetPlayer,
    rect::Rect,
    team::Team,
    trail::TrailSquare,
    utils::{
        approach_zero,
        get_combo_multiplier,
    },
};

#[derive(Clone)]
pub struct Player {
    pos: [f32; 2],
    vel: [f32; 2],
    lives: u8,
    name: String,
    stunned: f32,
    invulnerable_timer: f32,
    parry: f32,
    double_jumps: u8,
    combo: u32,
    combo_timer: f32,
    knockback_multiplier: f32,
    attacks: Vec<Attack>,
    trail_squares: Vec<TrailSquare>,
    can_slam: bool,
    dash_cooldown: f32,
    normal_cooldown: f32,
    light_cooldown: f32,
    parry_cooldown: f32,
    respawn_timer: f32,
    trail_timer: f32,
    team_idx: usize,
    facing: [f32; 2],
    input: PlayerInput,
    has_jumped: bool,
    start_pos: [f32; 2],
    color: Color,
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
            pos: start_pos,
            vel: [0.0, 0.0],
            lives: 3,
            name,
            stunned: RESPAWN_TIME,
            invulnerable_timer: 0.0,
            parry: 0.0,
            double_jumps: 2,
            combo: 0,
            combo_timer: 0.0,
            knockback_multiplier: 1.0,
            attacks: Vec::new(),
            trail_squares: Vec::new(),
            can_slam: true,
            dash_cooldown: 0.0,
            normal_cooldown: 0.0,
            light_cooldown: 0.0,
            parry_cooldown: 0.0,
            respawn_timer: RESPAWN_TIME,
            trail_timer: 0.0,
            team_idx,
            facing: get_facing_from_team(team_idx),
            input: PlayerInput::new(),
            has_jumped: false,
            start_pos,
            color,
        }
    }

    #[must_use]
    pub fn to_net(&self, player_idx: usize) -> NetPlayer {
        NetPlayer {
            team_idx: self.team_idx,
            player_idx,
            pos: self.pos,
            vel: self.vel,
            combo: self.combo,
            knockback_multiplier: self.knockback_multiplier,
            attacks: self.attacks
                .iter()
                .map(Attack::to_net)
                .collect(),
            stunned: self.stunned,
            invulnerable: self.invulnerable_timer,
            parry: self.parry,
            lives: self.lives,
        }
    }

    pub fn from_net(&mut self, net_player: NetPlayer) {
        self.pos = net_player.pos;
        self.vel = net_player.vel;
        self.lives = net_player.lives;
        self.combo = net_player.combo;
        self.knockback_multiplier = net_player.knockback_multiplier;
        self.attacks = net_player.attacks
            .iter()
            .map(|na| Attack::from_net(na.clone()))
            .collect();
        self.stunned = net_player.stunned;
        self.invulnerable_timer = net_player.invulnerable;
        self.parry = net_player.parry;
    }

    pub fn update(
        &mut self,
        map: &Rect,
        enemy_team: &Team,
        dt: f32,
    ) {
        self.facing = [0.0, 0.0];

        self.update_cooldowns(dt);

        if self.respawn_timer > 0.0 { return; }

        if self.combo > 0 && self.combo_timer == 0.0 {
            self.combo = 0;
        }

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
            self.double_jumps = 2;
        }

        self.update_trail(dt);
    }

    fn update_trail(&mut self, dt: f32) {
        self.trail_squares.iter_mut().for_each(|s| s.update(dt));
        self.trail_squares.retain(|s| s.lifetime > 0.0);

        if self.trail_timer >= 0.01
        && (
            self.is_doing_attack(&AttackKind::Slam)
            || self.is_doing_attack(&AttackKind::Dash)
            )
        {
            self.trail_timer = 0.0;
            self.trail_squares.push(
                TrailSquare::new(
                    self.pos,
                    self.color,
                )
            );
        }
    }

    fn update_cooldowns(&mut self, dt: f32) {
        let mut cooldowns = [
            &mut self.normal_cooldown,
            &mut self.light_cooldown,
            &mut self.parry,
            &mut self.parry_cooldown,
            &mut self.stunned,
            &mut self.invulnerable_timer,
            &mut self.dash_cooldown,
            &mut self.respawn_timer,
            &mut self.combo_timer,
        ];
        for cooldown in &mut cooldowns {
            if **cooldown > 0.0 {
                **cooldown -= dt;
            }
            **cooldown = (**cooldown).max(0.0);
        }

        self.update_attacks(dt);
    }

    fn update_position(
        &mut self,
        map: &Rect,
        enemy_team: &Team,
        dt: f32,
    ) {
        let old_pos = self.pos;

        self.pos[0] += self.vel[0] * dt;
        self.pos[1] += self.vel[1] * dt;

        // sweep test to prevent downward tunneling through platform
        if let Some(corrected_y) = self.sweep_down(
            old_pos[1],
            self.pos[1],
            map
        ) {
            // snap onto platform
            self.pos[1] = corrected_y;
            self.vel[1] = 0.0;
        }

        // sweep test to prevent downward tunneling through an opponent
        if self.is_doing_attack(&AttackKind::Slam) {
            for opponent in &enemy_team.players {
                if opponent.invulnerable_timer == 0.0
                && let Some(corrected_y) = self.sweep_down(
                    old_pos[1],
                    self.pos[1],
                    &opponent.get_rect()
                ) {
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
        if self.pos[1] > VIRTUAL_HEIGHT
        || self.pos[1] < 0.0
        || self.pos[0] > VIRTUAL_WIDTH
        || self.pos[0] < 0.0 {
            self.die();
        }
    }

    pub fn die(&mut self) {
        self.lives -= 1;
        self.double_jumps = 2;
        self.combo = 0;
        self.knockback_multiplier = 1.0;
        self.respawn_timer = RESPAWN_TIME;
        self.stunned = RESPAWN_TIME;
        self.invulnerable_timer = RESPAWN_TIME + 0.5;
        self.facing = get_facing_from_team(self.team_idx);
        self.vel = [0.0, 0.0];
        self.pos = self.start_pos;
    }

    pub fn apply_input(
        &mut self,
        map: &Rect,
        player_idx: usize,
        dt: f32,
    ) {
        if self.stunned > 0.0 || self.lives == 0 { return; }

        if self.input.up() {
            self.facing[1] = -1.0;
        }
        if self.input.jump() && !self.has_jumped {
            if self.is_on_platform(map) {
                self.vel[1] = -500.0;
            } 
            else if self.double_jumps > 0 {
                self.vel[1] = -500.0;
                self.double_jumps -= 1;
            }
            self.has_jumped = true;
        } else if !self.input.jump() {
            self.has_jumped = false;
        }
        if self.input.slam() {
            self.facing[1] = 1.0;
            if self.can_slam {
                self.attacks.push(
                    Attack::new(
                        AttackKind::Slam,
                        self.team_idx,
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
            self.facing[0] = -1.0;
            if self.vel[0] > -MAX_SPEED[0] {
                self.vel[0] -= ACCELERATION * dt;
            }
        }
        if self.input.right() {
            self.facing[0] = 1.0;
            if self.vel[0] < MAX_SPEED[0] {
                self.vel[0] += ACCELERATION * dt;
            }
        }
        if self.input.light() && self.light_cooldown <= 0.0 {
            self.attacks.push(
                Attack::new(
                    AttackKind::Light,
                    self.team_idx,
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
                    self.team_idx,
                    player_idx,
                    self.facing,
                )
            );
            self.normal_cooldown = 0.75;
        }
        if self.input.dash()
        && self.dash_cooldown <= 0.0
        && !self.parrying() {
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
                    self.team_idx,
                    player_idx,
                    self.facing,
                )
            );

            self.dash_cooldown = 3.0;
        }
        if self.input.parry()
        && self.parry_cooldown <= 0.0
        && !self.is_doing_attack(&AttackKind::Dash)
        && !self.is_doing_attack(&AttackKind::Slam) {
            self.parry_cooldown = 4.0;
            self.parry = 0.5;
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

    pub fn attack(&mut self, atk: &Attack, attacker: &mut Player) {
        if self.invulnerable_timer > 0.0 { return; }

        if self.parry > 0.0 {
            // get dash ability back when successfully parrying
            self.dash_cooldown = 0.0;

            // reset combo
            self.combo = 0;

            // stun attacker with own attack's stun
            attacker.stunned = atk.stun();

            // half the velocity to avoid next
            // to garantueed kill when dashing
            attacker.vel[0] /= 2.0;
            attacker.vel[1] /= 2.0;

            return;
        }

        match atk.kind() {
            AttackKind::Dash => {
                if self.is_doing_attack(atk.kind()) {
                    for player in [&mut *self, attacker] {
                        player.vel[0] = player.vel[0].signum()
                            * -50.0
                            * player.knockback_multiplier;
                        player.vel[1] = player.vel[1].signum()
                            * -200.0
                            * player.knockback_multiplier;
                        player.stunned = atk.stun();
                        player.knockback_multiplier += atk.knockback_increase();
                        player.remove_dashes();
                    }
                } else {
                    self.vel[0] = attacker.vel[0]
                        * self.knockback_multiplier;
                    self.vel[1] = attacker.vel[1]
                        * self.knockback_multiplier;
                }
                attacker.vel[0] *= -0.5;
                attacker.vel[1] *= -0.5;
            }
            AttackKind::Light => {
                // if player is in a combo, this
                // attack is used as a finisher
                if self.combo > 0 {
                    // overwrite default attack stun
                    self.stunned = 0.5;

                    // launch player
                    self.vel[0] = attacker.facing[0]
                        * 600.0
                        * self.knockback_multiplier
                        * get_combo_multiplier(self.combo);
                    self.vel[1] = attacker.facing[1]
                        * 600.0
                        * self.knockback_multiplier
                        * get_combo_multiplier(self.combo);

                    // apply knockback multiplier boost for combo
                    self.knockback_multiplier += 0.1 * get_combo_multiplier(self.combo);

                    // apply invulnerability because generic attack
                    // traits are not applied due to early return
                    self.invulnerable_timer = 0.3;

                    return;
                }
            }
            AttackKind::Slam => {
                // must be above player and moving downwards
                if attacker.pos[1] + PLAYER_SIZE > self.pos[1]
                || attacker.vel[1] <= 0.0 { return; }

                self.vel[1] = attacker.vel[1]
                    * 1.5
                    * self.knockback_multiplier;

                attacker.vel[1] = -50.0;
                attacker.can_slam = false;
                attacker.remove_slams();
            }
            AttackKind::Normal => {
                self.vel[0] = attacker.facing[0] * 450.0;
                self.vel[1] = attacker.facing[1] * 450.0;

                attacker.normal_cooldown -= 0.25;
            }
        }

        // if not returned by this point,
        // apply generic attack traits
        self.remove_dashes();
        self.remove_slams();

        self.stunned = atk.stun();
        self.knockback_multiplier += atk.knockback_increase();
        self.invulnerable_timer = 0.3;

        self.combo += 1;
        self.combo_timer = 1.0;
    }

    pub fn update_input(&mut self, pressed: &HashSet<KeyCode>) {
        self.input.update(pressed);
    }

    pub fn set_input(&mut self, input: PlayerInput) {
        self.input = input;
    }

    // GETTERS
    #[must_use]
    pub fn attacks(&self) -> &Vec<Attack> { &self.attacks }

    #[must_use]
    pub fn trail_squares(&self) -> &Vec<TrailSquare> { &self.trail_squares }

    #[must_use]
    pub fn is_doing_attack(&self, kind: &AttackKind) -> bool {
        self.attacks.iter().any(|atk| atk.kind() == kind)
    }

    #[must_use]
    pub fn get_rect(&self) -> Rect {
        Rect::new(self.pos[0], self.pos[1], PLAYER_SIZE, PLAYER_SIZE)
    }

    fn is_on_platform(&self, platform: &Rect) -> bool {
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

    #[must_use]
    pub fn get_color(&self) -> Color {
        if self.stunned > 0.0 {
            Color::new(
                (self.color.r + 0.4).min(1.0),
                (self.color.g + 0.4).min(1.0),
                (self.color.b + 0.4).min(1.0),
                1.0,
            )
        } else {
            self.get_color_default()
        }
    }

    #[must_use]
    pub fn get_color_default(&self) -> Color { self.color }

    #[must_use]
    pub fn parrying(&self) -> bool { self.parry > 0.0 }

    #[must_use]
    pub fn lives(&self) -> u8 { self.lives }

    #[must_use]
    pub fn is_dead(&self) -> bool { self.lives == 0 }

    #[must_use]
    pub fn name(&self) -> String { self.name.clone() }

    #[must_use]
    pub fn position(&self) -> [f32; 2] { self.pos }

    #[must_use]
    pub fn combo(&self) -> u32 { self.combo }

    #[must_use]
    pub fn get_input(&self) -> &PlayerInput { &self.input }
}

fn get_facing_from_team(team_idx: usize) -> [f32; 2] {
    [if team_idx == 0 { 1.0 } else { -1.0 }, 0.0]
}
