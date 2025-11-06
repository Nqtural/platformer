// Imports {{{

use ggez::{
    Context,
    ContextBuilder,
    GameResult,
    graphics::{
        Canvas,
        Color,
        Drawable,
        DrawMode,
        DrawParam,
        GraphicsContext,
        Image,
        ImageFormat,
        PxScale,
        Mesh,
        Rect,
        Text,
        TextFragment,
    },
    event::{
        self,
        EventHandler,
    },
    input::keyboard::{
        KeyCode,
        KeyInput,
    },
};
use glam::Vec2;
use mint::{Point2, Vector2};
use serde::{Serialize, Deserialize};

// }}}

// Mint trait {{{

trait IntoMint {
    fn to_mint_point(self) -> Point2<f32>;
    fn to_mint_vec(self) -> Vector2<f32>;
}

impl IntoMint for Vec2 {
    fn to_mint_point(self) -> Point2<f32> {
        Point2 { x: self.x, y: self.y }
    }

    fn to_mint_vec(self) -> Vector2<f32> {
        Vector2 { x: self.x, y: self.y }
    }
}

// }}}

// Constants {{{

const ENABLE_VSYNC: bool = true;

const C_TEAM: usize = 0;
const C_PLAYER: usize = 0;

const VIRTUAL_WIDTH: f32 = 1980.0;
const VIRTUAL_HEIGHT: f32 = 1080.0;

const BACKGROUND: Color = Color::new(0.15, 0.15, 0.2, 1.0);
const MAP_COLOR: Color = Color::new(0.0, 0.0, 0.0, 1.0);
const NAME_COLOR: Color = Color::new(0.6, 0.6, 0.6, 1.0);
const TEAM_ONE_COLOR: Color = Color::new(0.0, 0.0, 1.0, 1.0);
const TEAM_TWO_COLOR: Color = Color::new(1.0, 0.0, 0.0, 1.0);
const TRAIL_OPACITY: f32 = 0.15;

const TEAM_ONE_START_POS: [f32; 2] = [250.0, 300.0];
const TEAM_TWO_START_POS: [f32; 2] = [550.0, 300.0];

const PLAYER_SIZE: f32 = 20.0;

const MAX_SPEED: [f32; 2] = [300.0, 600.0];
const ACCELERATION: f32 = 5000.0;
const GRAVITY: f32 = 1200.0;
const RESISTANCE: f32 = 1800.0;
const WALL_SLIDE_SPEED: f32 = 0.0;

const RESPAWN_TIME: f32 = 2.5;

// }}}

// Helper functions {{{

fn approach_zero(value: f32, step: f32) -> f32 {
    if value > 0.0 {
        (value - step).max(0.0)
    } else if value < 0.0 {
        (value + step).min(0.0)
    } else {
        0.0
    }
}

fn handle_dash_collision(player: &mut Player, enemy: &mut Player) {
    if enemy.dashing > 0.0 {
        enemy.vel[0] = player.vel[0].signum() * 100.0 * enemy.knockback_multiplier;
        enemy.dashing = 0.0;
        enemy.stunned = 0.5;
        enemy.knockback_multiplier += 0.01;

        player.stunned = 0.5;
    } else {
        enemy.vel[0] = player.vel[0] * enemy.knockback_multiplier;
        enemy.stunned = 0.1;
    }

    enemy.vel[1] -= 200.0;
    enemy.slow = 0.5;

    player.vel[1] -= 200.0;
    player.vel[0] = player.vel[0] * -0.5;
    player.dashing = 0.0;
    player.slow = 0.5;
}

fn handle_slam_collision(player: &mut Player, enemy: &mut Player) {
    let player_bottom = player.pos[1] + PLAYER_SIZE;
    let enemy_top = enemy.pos[1];

    if player_bottom <= enemy_top + 5.0 && player.vel[1] > 0.0 {
        enemy.vel[1] = player.vel[1] * 1.5 * enemy.knockback_multiplier;
        enemy.stunned = 0.1;
        enemy.slow = 0.5;
        enemy.knockback_multiplier += 0.03;

        player.vel[1] = -100.0;
        player.slow = 0.5;
        player.input.slam = false;
    }
}

fn handle_collisions<'a>(
    player: &mut Player,
    teams: impl Iterator<Item = &'a mut Team>,
) {
    let player_rect = player.get_rect();

    for enemy in teams.flat_map(|team| team.players.iter_mut()) {
        if player_rect.overlaps(&enemy.get_rect()) && enemy.invulnerable_timer == 0.0 {
            if player.dashing > 0.0 {
                handle_dash_collision(player, enemy);
            } else if player.input.slam {
                handle_slam_collision(player, enemy);
            }
        }
    }
}

// }}}

// Game objects {{{

// Attack {{{

#[derive(Serialize, Deserialize, Clone)]
struct Attack {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    power: f32,
    stun: f32,
    knockback: [f32; 2],
    slow: f32,
    duration: f32,
    timer: f32,
    owner_team: usize,
}

impl Attack {
    fn light(player: &Player, owner_team: usize) -> Attack {
        Attack {
            x: player.pos[0] - 9.0,
            y: player.pos[1] - 10.0,
            w: 40.0,
            h: 40.0,
            power: 0.01,
            stun: 0.2,
            knockback: [
                (player.vel[0] / 2.0) + (400.0 * player.facing),
                -200.0
            ],
            slow: 0.5,
            duration: 0.1,
            timer: 0.0,
            owner_team,
        }
    }
    fn uppercut(player: &Player, owner_team: usize) -> Attack {
        Attack {
            x: player.pos[0] - 9.0,
            y: player.pos[1] - 10.0,
            w: 40.0,
            h: 40.0,
            power: 0.02,
            stun: 0.4,
            knockback: [0.0, -500.0],
            slow: 0.5,
            duration: 0.15,
            timer: 0.0,
            owner_team,
        }
    }

    fn get_rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.w, self.h)
    }
}

// }}}

// Player {{{

#[derive(Serialize, Deserialize, Clone)]
struct Player {
    pos: [f32; 2],
    vel: [f32; 2],
    lives: i32,
    name: String,
    stunned: f32,
    invulnerable_timer: f32,
    slow: f32,
    double_jumps: u8,
    knockback_multiplier: f32,
    slamming: bool,
    dashing: f32,
    dash_cooldown: f32,
    attack_cooldown: f32,
    respawn_timer: f32,
    trail_timer: f32,
    facing: f32,
    input: PlayerInput,
}

impl Player {
    fn new(pos: [f32; 2], name: String) -> Player {
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

    fn get_rect(&self) -> Rect {
        Rect::new(self.pos[0], self.pos[1], PLAYER_SIZE, PLAYER_SIZE)
    }

    fn attack(&mut self, attack: &Attack) {
        self.stunned = attack.stun;
        self.invulnerable_timer = 0.1;
        self.slow = attack.slow;
        self.vel[0] = attack.knockback[0] * self.knockback_multiplier;
        self.vel[1] = attack.knockback[1];
        self.knockback_multiplier += attack.power;
        self.dashing = 0.0;
    }

    fn update_cooldowns(&mut self, dt: f32) {
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

    fn update_position(&mut self, dt: f32) {
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

    fn check_platform_collision(
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

    fn apply_input(&mut self, map: &Rect, team: usize, dt: f32) -> Vec<Attack> {
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
                new_attacks.push(Attack::light(&self, team));
                self.slow = 0.5;
                self.attack_cooldown = 0.3;
                self.input.uppercut = true;
            }
            if self.input.uppercut {
                new_attacks.push(Attack::uppercut(&self, C_TEAM));
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

// }}}

// Player input {{{

#[derive(Serialize, Deserialize, Clone, Default)]
struct PlayerInput {
    left: bool,
    right: bool,
    up: bool,
    slam: bool,
    light: bool,
    uppercut: bool,
    dash: bool,
}

impl PlayerInput {
    fn new() -> PlayerInput {
        PlayerInput {
            left: false,
            right: false,
            up: false,
            slam: false,
            light: false,
            uppercut: false,
            dash: false,
        }
    }
}

// }}}

// TrailSquare {{{

#[derive(Serialize, Deserialize, Clone)]
struct TrailSquare {
    rect: Rect,
    color: Color,
    lifetime: f32,
}

impl TrailSquare {
    fn new(x: f32, y: f32, color: Color) -> TrailSquare {
        TrailSquare {
            rect: Rect::new(x, y, PLAYER_SIZE, PLAYER_SIZE),
            color: Color::new(color.r, color.g, color.b, TRAIL_OPACITY),
            lifetime: 0.15,
        }
    }

    fn update(&mut self, dt: f32) {
        self.lifetime -= dt;
        self.color = Color::new(
            self.color.r,
            self.color.g,
            self.color.b,
            TRAIL_OPACITY * (self.lifetime / 0.15).powf(2.0) 
        );
    }
}

// }}}

// Team {{{

#[derive(Serialize, Deserialize, Clone)]
struct Team {
    players: Vec<Player>,
    color_default: Color,
    color_stunned: Color,
    trail_interval: f32,
    trail_squares: Vec<TrailSquare>,
    start_pos: [f32; 2],
}

impl Team {
    fn new(players: Vec<Player>, color: Color, start_pos: [f32; 2]) -> Team {
        Team {
            players,
            color_default: color,
            color_stunned: Color::new(color.r * 2.0, color.g * 2.0, color.b * 2.0, 1.0),
            trail_interval: 0.01,
            trail_squares: Vec::new(),
            start_pos
        }
    }

    fn update_players(
        &mut self,
        left: &mut [Team],
        others: &mut [Team],
        team_idx: usize,
        map: &Rect,
        winner: usize,
        active_attacks: &mut Vec<Attack>,
        mut normal_dt: f32,
    ) {
        if winner > 0 {
            normal_dt = normal_dt / 2.0;
        }

        let slow_dt = normal_dt / 2.0;

        self.trail_squares.iter_mut().for_each(|s| s.update(normal_dt));
        self.trail_squares.retain(|s| s.lifetime > 0.0);

        for player in &mut self.players {
            player.update_cooldowns(normal_dt);

            if player.respawn_timer > 0.0 { continue; }

            let dt = if player.slow > normal_dt {
                slow_dt
            } else {
                normal_dt
            };

            active_attacks.extend(player.apply_input(map, team_idx, dt));

            if player.dashing > 0.0 || player.input.slam {
                handle_collisions(player, left.iter_mut().chain(others.iter_mut()));
                player.trail_timer += dt;
                while player.trail_timer >= self.trail_interval {
                    player.trail_timer -= self.trail_interval;
                    self.trail_squares.push(
                        TrailSquare::new(
                            player.pos[0],
                            player.pos[1],
                            self.color_default
                        )
                    )
                }
            }

            player.update_position(dt);

            player.check_platform_collision(
                &map,
                dt,
            );
        }
    }
}

// }}}

// Map {{{

#[derive(Serialize, Deserialize, Clone)]
struct Map {
    rect: Rect,
    color: Color,
}

impl Map {
    fn new() -> Map {
        Map {
            rect: Rect::new(200.0, 350.0, 400.0, 30.0),
            color: MAP_COLOR,
        }
    }
}

// }}}

// GameState {{{

#[derive(Serialize, Deserialize, Clone)]
struct GameState {
    teams: [Team; 2],
    map: Map,
    active_attacks: Vec<Attack>,
    camera_pos: Vec2,
    winner: usize,
}

impl GameState {
    fn new(teams: [Team; 2]) -> GameState {
        GameState {
            teams,
            map: Map::new(),
            active_attacks: Vec::new(),
            camera_pos: Vec2::new(0.0, 0.0),
            winner: 0,
        }
    }

    fn check_for_win(&mut self) {
        if self.winner > 0 {
            return;
        }

        for (team_idx, team) in self.teams.iter_mut().enumerate() {
            for player in &team.players {
                if player.lives > 0 { continue; }
                self.winner = if team_idx == 0 { 2 } else { 1 };
                println!("Winner: Team {}", self.winner)
            }
        }
    }

    fn handle_attack_collisions(&mut self) {
        for atk in &self.active_attacks {
            for (team_idx, team) in self.teams.iter_mut().enumerate() {
                if team_idx == atk.owner_team { continue; }
                for player in &mut team.players {
                    if player.stunned > 0.0
                        || player.invulnerable_timer > 0.0
                        || !atk.get_rect().overlaps(&player.get_rect())
                    { continue; }
                    player.attack(&atk);
                }
            }
        }
    }

    fn check_for_death(&mut self) {
        let death_y = VIRTUAL_HEIGHT;
        for team in &mut self.teams {
            for player in &mut team.players {
                if player.pos[1] > death_y {
                    player.lives -= 1;
                    player.double_jumps = 2;
                    player.knockback_multiplier = 1.0;
                    player.respawn_timer = RESPAWN_TIME;
                    player.stunned = RESPAWN_TIME;
                    player.invulnerable_timer = RESPAWN_TIME + 0.5;
                    player.facing = 0.0;
                    player.vel = [0.0, 0.0];
                    player.pos = team.start_pos;
                }
            }
        }
    }

    fn update_camera(&mut self) {
        let mut sum = Vec2::ZERO;
        let mut count: usize = 0;

        for team in &self.teams {
            for player in &team.players {
                if player.lives == 0 { continue; }
                sum += Vec2::new(player.pos[0] + PLAYER_SIZE / 2.0, player.pos[1] + PLAYER_SIZE / 2.0);
                count += 1;
            }
        }

        if count == 0 { return; }

        let player_center = sum / count as f32;

        let map_center = Vec2::new(
            self.map.rect.x + self.map.rect.w / 2.0,
            self.map.rect.y + self.map.rect.h / 2.0,
        );

        // future client side option together with colors,
        // background color or image, controls, etc.
        let bias_strength = 0.7;

        let biased_target = player_center.lerp(map_center, bias_strength);

        let lerp_factor = 0.1;
        self.camera_pos = self.camera_pos.lerp(biased_target, lerp_factor);
    }

    fn draw_map(&self, game_canvas: &mut Canvas, gfx: &mut GraphicsContext, camera_transform: &DrawParam) -> GameResult {
        let map_mesh = Mesh::new_rectangle(
            gfx,
            DrawMode::fill(),
            self.map.rect,
            self.map.color,
        )?;
        game_canvas.draw(&map_mesh, *camera_transform);
        Ok(())
    }

    fn draw_trails(
        &self,
        game_canvas: &mut Canvas,
        gfx: &mut GraphicsContext,
        camera_transform: &DrawParam,
    ) -> GameResult {
        for team in &self.teams {
            for square in &team.trail_squares {
                let mesh = Mesh::new_rectangle(
                    gfx, DrawMode::fill(),
                    square.rect,
                    square.color
                )?;
                game_canvas.draw(&mesh, *camera_transform);
            }
        }

        Ok(())
    }


    fn draw_players(
        &self,
        game_canvas: &mut Canvas,
        ctx: &mut Context,
        camera_translation: Vec2,
        zoom: f32,
    ) -> GameResult {
        let camera_transform = DrawParam::default()
            .dest(camera_translation)
            .scale(Vec2::new(zoom, zoom).to_mint_vec());
        for team in &self.teams {
            for player in &team.players {
                if player.lives <= 0 { continue; }

                let opacity = if player.invulnerable_timer > 0.0 {
                    0.5
                } else {
                    1.0
                };
                let color = if player.stunned > 0.0 {
                    Color::new(
                        team.color_stunned.r,
                        team.color_stunned.g,
                        team.color_stunned.b,
                        opacity
                    )
                } else {
                    Color::new(
                        team.color_default.r,
                        team.color_default.g,
                        team.color_default.b,
                        opacity
                    )
                };
                let rect = player.get_rect();
                let mesh = Mesh::new_rectangle(
                    &ctx.gfx,
                    DrawMode::fill(),
                    rect,
                    color
                )?;
                game_canvas.draw(&mesh, camera_transform);
                let outline = Mesh::new_rectangle(
                    &ctx.gfx,
                    DrawMode::stroke(2.0),
                    rect,
                    Color::new(0.0, 0.0, 0.0, 1.0)
                )?;
                game_canvas.draw(&outline, camera_transform);

                let text = Text::new(TextFragment {
                    text: format!("{}", player.name),
                    font: None,
                    scale: Some(PxScale::from(14.0 * zoom)),
                    color: Some(NAME_COLOR),
                    //color: Some(team.color_default),
                    ..Default::default()
                });

                let text_dims = text.dimensions(ctx).unwrap();
                let text_pos = Vec2::new(
                    player.pos[0] + (PLAYER_SIZE / 2.0) - (text_dims.w as f32 / 2.0),
                    player.pos[1] + 25.0
                ) * zoom + camera_translation;

                game_canvas.draw(&text, DrawParam::default().dest(text_pos));
            }
        }

        Ok(())
    }

    fn draw_hud(
        &self,
        game_canvas: &mut Canvas,
        ctx: &Context,
    ) -> GameResult {
        const MARGIN: f32 = 40.0;
        const START_X_LEFT: f32 = MARGIN;
        const START_X_RIGHT: f32 = VIRTUAL_WIDTH - MARGIN;
        const START_Y: f32 = MARGIN;
        const LINE_HEIGHT: f32 = MARGIN;

        for (team_idx, team) in self.teams.iter().enumerate() {
            let is_right_team = team_idx == 1;

            for (player_idx, player) in team.players.iter().enumerate() {
                let y = START_Y + player_idx as f32 * LINE_HEIGHT;
                let text = Text::new(TextFragment {
                    text: format!(
                        "Player {} Lives: {}",
                        player_idx + 1,
                        player.lives
                    ),
                    font: None,
                    scale: Some(PxScale::from(36.0)),
                    ..Default::default()
                });

                let dest = if is_right_team {
                    let dims = text.dimensions(ctx);
                    Vec2::new(START_X_RIGHT - dims.unwrap().w as f32, y)
                } else {
                    Vec2::new(START_X_LEFT, y)
                };

                game_canvas.draw(&text, DrawParam::default().dest(dest));
            }
        }

        let fps = ctx.time.fps();
        let fps_text = Text::new(TextFragment {
            text: format!("FPS: {:.0}", fps),
            font: None,
            scale: Some(PxScale::from(32.0)),
            ..Default::default()
        });

        let fps_dims = fps_text.dimensions(ctx).unwrap();
        let fps_x = (VIRTUAL_WIDTH - fps_dims.w as f32) / 2.0;
        let fps_y = MARGIN / 2.0; // slightly above other HUD elements

        game_canvas.draw(
            &fps_text,
            DrawParam::default().dest(
                Vec2::new(fps_x, fps_y).to_mint_point()
            )
        );

        if self.winner > 0 {
            let winner_text = Text::new(TextFragment {
                text: format!("TEAM {} WINS!", self.winner),
                font: None,
                scale: Some(PxScale::from(200.0)),
                color: Some(
                    if self.winner == 1 {
                        TEAM_ONE_COLOR
                    } else {
                        TEAM_TWO_COLOR
                    }
                ),
                ..Default::default()
            });

            let winner_dims = winner_text.dimensions(ctx).unwrap();
            let winner_x = (VIRTUAL_WIDTH - winner_dims.w as f32) / 2.0;
            let winner_y = (VIRTUAL_HEIGHT - winner_dims.h as f32) / 3.5;

            game_canvas.draw(
                &winner_text,
                DrawParam::default().dest(Vec2::new(
                    winner_x,
                    winner_y
                ).to_mint_point())
            );
        }

        Ok(())
    }
}

impl EventHandler for GameState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        let dt = ctx.time.delta().as_secs_f32();

        for attack in &mut self.active_attacks {
            attack.timer += dt;
        }
        self.active_attacks.retain(|atk| atk.timer < atk.duration);

        self.handle_attack_collisions();

        self.check_for_win();

        for team_idx in 0..self.teams.len() {
            let (left, right) = self.teams.split_at_mut(team_idx);
            let (team, others) = right.split_first_mut().unwrap();

            team.update_players(
                left,
                others,
                team_idx,
                &self.map.rect,
                self.winner,
                &mut self.active_attacks,
                dt,
            );
        }

        self.check_for_death();

        self.update_camera();

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let target_image = Image::new_canvas_image(
            &mut ctx.gfx,
            ImageFormat::Rgba8UnormSrgb,
            VIRTUAL_WIDTH as u32,
            VIRTUAL_HEIGHT as u32,
            1,
        );

        let mut game_canvas = Canvas::from_image(
            &mut ctx.gfx,
            target_image.clone(),
            BACKGROUND,
        );
        game_canvas.set_screen_coordinates(
            Rect::new(
                0.0,
                0.0,
                VIRTUAL_WIDTH,
                VIRTUAL_HEIGHT
            )
        );

        let (win_w, win_h) = ctx.gfx.drawable_size();
        let virtual_aspect = VIRTUAL_WIDTH / VIRTUAL_HEIGHT;
        let window_aspect = win_w / win_h;

        let zoom = 1.1;

        let screen_center = Vec2::new(
            VIRTUAL_WIDTH / 2.0,
            VIRTUAL_HEIGHT / 2.0
        );
        let camera_translation = screen_center - self.camera_pos * zoom;

        let camera_transform = DrawParam::default()
            .dest(camera_translation)
            .scale(Vec2::new(zoom, zoom).to_mint_vec());

        self.draw_map(&mut game_canvas, &mut ctx.gfx, &camera_transform)?;
        self.draw_trails(&mut game_canvas, &mut ctx.gfx, &camera_transform)?;
        self.draw_players(&mut game_canvas, ctx, camera_translation, zoom)?;
        self.draw_hud(&mut game_canvas, &ctx)?;

        game_canvas.finish(&mut ctx.gfx)?;

        let mut final_canvas = Canvas::from_frame(&mut ctx.gfx, Color::BLACK);

        let scale = if window_aspect > virtual_aspect {
            let scale = win_h / VIRTUAL_HEIGHT;
            let x_offset = (win_w - VIRTUAL_WIDTH * scale) / 2.0;
            DrawParam::default()
                .dest(Vec2::new(x_offset, 0.0).to_mint_point())
                .scale(Vec2::new(scale, scale).to_mint_vec())
        } else {
            let scale = win_w / VIRTUAL_WIDTH;
            let y_offset = (win_h - VIRTUAL_HEIGHT * scale) / 2.0;
            DrawParam::default()
                .dest(Vec2::new(0.0, y_offset).to_mint_point())
                .scale(Vec2::new(scale, scale).to_mint_vec())
        };

        final_canvas.draw(&target_image, scale);
        final_canvas.finish(&mut ctx.gfx)
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        key: KeyInput,
        _repeated: bool,
    ) -> GameResult {
        if let Some(keycode) = key.keycode {
            let input = &mut self.teams[C_TEAM].players[C_PLAYER].input;
            match keycode {
                KeyCode::Escape => ctx.request_quit(),
                KeyCode::W => input.up = true,
                KeyCode::A => input.left = true,
                KeyCode::D => input.right = true,
                KeyCode::S => input.slam = true,
                KeyCode::J => input.light = true,
                KeyCode::K => input.uppercut = true,
                KeyCode::H => input.dash = true,
                _ => {}
            }
        }

        Ok(())
    }

    fn key_up_event(
        &mut self,
        _ctx: &mut Context,
        key: KeyInput,
    ) -> GameResult {
        if let Some(keycode) = key.keycode {
            let input = &mut self.teams[C_TEAM].players[C_PLAYER].input;
            match keycode {
                KeyCode::W => input.up = false,
                KeyCode::A => input.left = false,
                KeyCode::D => input.right = false,
                KeyCode::S => input.slam = false,
                KeyCode::J => input.light = false,
                KeyCode::K => input.uppercut = false,
                KeyCode::H => input.dash = false,
                _ => {}
            }
        }

        Ok(())
    }
}

// }}}

// }}}

// Main function {{{

fn main() -> GameResult {
    let (ctx, event_loop) = ContextBuilder::new("game", "me")
        .window_setup(
            ggez::conf::WindowSetup::default()
                .vsync(ENABLE_VSYNC)
                .title("Game")
        )
        .window_mode(
            ggez::conf::WindowMode::default()
                .dimensions(VIRTUAL_WIDTH, VIRTUAL_HEIGHT) 
                .resizable(true)
        )
        .build()?;

    let game_state = GameState::new([
        Team::new(
            vec![Player::new(TEAM_ONE_START_POS, String::from("Player"))],
            TEAM_ONE_COLOR,
            TEAM_ONE_START_POS,
        ),
        Team::new(
            vec![Player::new(TEAM_TWO_START_POS, String::from("Player"))],
            TEAM_TWO_COLOR,
            TEAM_TWO_START_POS,
        ),
    ]);

    event::run(ctx, event_loop, game_state)
}

// }}}
