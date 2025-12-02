use crate::{
    attack::{
        Attack,
        AttackKind,
    },
    constants::{
        ATTACK_IMAGE,
        BACKGROUND_IMAGE,
        PARY_IMAGE,
        C_PLAYER,
        C_TEAM,
        NAME_COLOR,
        PLAYER_SIZE,
        VIRTUAL_HEIGHT,
        VIRTUAL_WIDTH,
    },
    map::Map,
    network::NetSnapshot,
    read_config::Config,
    team::Team,
    traits::IntoMint,
    utils::current_and_enemy,
};
use ggez::{
    Context,
    GameResult,
    graphics::{
        Canvas,
        Color,
        GraphicsContext,
            Drawable,
        DrawMode,
        DrawParam,
        Image,
        ImageFormat,
        Mesh,
        PxScale,
        Rect,
        Text,
        TextFragment,
    },
};
use glam::Vec2;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct GameState {
    pub teams: [Team; 2],
    pub map: Map,
    pub camera_pos: Vec2,
    bias_strength: f32,
    pub winner: usize,
    team_one_color: Color,
    team_two_color: Color,
    #[serde(skip)]
    #[serde(default)]
    background_image: Option<Image>,
    #[serde(skip)]
    #[serde(default)]
    attack_image: Option<Image>,
    #[serde(skip)]
    #[serde(default)]
    pary_image: Option<Image>,
}

impl GameState {
    pub fn new(teams: [Team; 2], ctx: &mut Context) -> GameResult<Self> {
        let bg_img = Image::from_path(&ctx.gfx, BACKGROUND_IMAGE)?;
        let attack_img = Image::from_path(&ctx.gfx, ATTACK_IMAGE)?;
        let pary_img = Image::from_path(&ctx.gfx, PARY_IMAGE)?;
        let config = Config::get()?;

        Ok(Self {
            teams,
            map: Map::new(),
            camera_pos: Vec2::new(0.0, 0.0),
            bias_strength: config.camera_bias(),
            winner: 0,
            team_one_color: config.team_one_color(),
            team_two_color: config.team_two_color(),
            background_image: Some(bg_img),
            attack_image: Some(attack_img),
            pary_image: Some(pary_img),
        })
    }

    pub fn render_update(&mut self, ctx: &mut Context) -> GameResult {
        let dt = ctx.time.delta().as_secs_f32();

        self.check_for_win();

        for i in 0..2 {
            let (current, enemy) = current_and_enemy(&mut self.teams, i);
            current.update_players(
                enemy,
                i,
                &self.map.get_rect(),
                self.winner,
                dt,
            );
        }

        self.update_camera();

        Ok(())
    }

    pub fn fixed_update(&mut self, dt: f32) {
        self.check_for_win();

        for i in 0..2 {
            let (current, enemy) = current_and_enemy(&mut self.teams, i);
            current.update_players(
                enemy,
                i,
                &self.map.get_rect(),
                self.winner,
                dt,
            );
        }
    }

    #[must_use]
    pub fn to_net(&self) -> NetSnapshot {
        NetSnapshot {
            tick: 0,
            winner: self.winner,
            players: self.teams.iter().enumerate().flat_map(|(team_id, team)| {
                team.players.iter().enumerate().map(move |(player_id, p)| {
                    p.to_net(team_id, player_id)
                })
            }).collect(),
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: NetSnapshot) {
        self.winner = snapshot.winner;

        for net_player in snapshot.players {
            if let Some(team) = self.teams.get_mut(net_player.team_id)
                && let Some(player) = team.players.get_mut(net_player.player_id) {
                    player.pos = net_player.pos;
                    player.vel = net_player.vel;
                    player.lives = net_player.lives;
                    player.attacks = net_player.attacks
                        .iter()
                        .map(|na| Attack::from_net(na.clone()))
                        .collect();
                    player.stunned = net_player.stunned;
                    player.invulnerable_timer = net_player.invulnerable;
                    player.pary = net_player.pary;
                }
        }
    }

    #[must_use]
    pub fn to_snapshot(&self) -> NetSnapshot {
        let mut net_players = Vec::new();

        for (team_id, team) in self.teams.iter().enumerate() {
            for (player_id, p) in team.players.iter().enumerate() {
                net_players.push(p.to_net(team_id, player_id));
            }
        }

        NetSnapshot {
            tick: 0,
            winner: self.winner,
            players: net_players,
        }
    }

    fn check_for_win(&mut self) {
        if self.winner > 0 {
            return;
        }

        for (team_idx, team) in self.teams.iter_mut().enumerate() {
            if team.players.iter().all(|p| p.lives == 0) {
                self.winner = if team_idx == 0 { 2 } else { 1 };
                break;
            }
        }
    }

    fn drawparam_constructor(&self, x: f32, y: f32) -> DrawParam {
        let zoom = 1.1;
        let screen_center = Vec2::new(VIRTUAL_WIDTH / 2.0, VIRTUAL_HEIGHT / 2.0);

        DrawParam::default()
            .dest(
                screen_center 
                + Vec2::new(x, y) * zoom 
                - self.camera_pos * zoom
            )
            .scale(Vec2::new(zoom, zoom).to_mint_vec())
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

        let map_rect = self.map.get_rect();
        let map_center = Vec2::new(
            map_rect.x + map_rect.w / 2.0,
            map_rect.y + map_rect.h / 2.0,
        );

        let biased_target = player_center.lerp(map_center, self.bias_strength);

        let lerp_factor = 0.1;
        self.camera_pos = self.camera_pos.lerp(biased_target, lerp_factor);
    }

    pub fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let target_image = Image::new_canvas_image(
            &ctx.gfx,
            ImageFormat::Rgba8UnormSrgb,
            VIRTUAL_WIDTH as u32,
            VIRTUAL_HEIGHT as u32,
            1,
        );

        let mut game_canvas = Canvas::from_image(
            &ctx.gfx,
            target_image.clone(),
            Color::new(0.1, 0.1, 0.15, 1.0),
        );
        game_canvas.set_screen_coordinates(
            Rect::new(
                0.0,
                0.0,
                VIRTUAL_WIDTH,
                VIRTUAL_HEIGHT
            )
        );

        // draw background
        if let Some(img) = self.background_image.as_ref() {
            game_canvas.draw(
                img,
                DrawParam::default()
                    .dest([0.0, 0.0])
                    .scale([
                        VIRTUAL_WIDTH  / img.width()  as f32,
                        VIRTUAL_HEIGHT / img.height() as f32,
                    ]),
            );
        }

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
        self.draw_hud(&mut game_canvas, ctx);

        game_canvas.finish(&mut ctx.gfx)?;

        let mut final_canvas = Canvas::from_frame(&ctx.gfx, Color::BLACK);

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

    fn draw_map(&self, game_canvas: &mut Canvas, gfx: &mut GraphicsContext, camera_transform: &DrawParam) -> GameResult {
        let map_mesh = Mesh::new_rectangle(
            gfx,
            DrawMode::fill(),
            self.map.get_rect(),
            self.map.get_color(),
        )?;
        game_canvas.draw(&map_mesh, *camera_transform);

        Ok(())
    }

    fn draw_pary(
        &self,
        game_canvas: &mut Canvas,
        player_pos: [f32; 2],
    ) {
        if let Some(img) = self.pary_image.as_ref() {
            // draw frame
            let draw_param = self.drawparam_constructor(
                (player_pos[0] + PLAYER_SIZE / 2.0) - (img.width() as f32 / 2.0),
                (player_pos[1] + PLAYER_SIZE / 2.0) - (img.width() as f32 / 2.0),
            );

            game_canvas.draw(img, draw_param);
        }
    }

    fn draw_attacks(
        &self,
        game_canvas: &mut Canvas,
        player_pos: [f32; 2],
        attacks: &Vec<Attack>,
    ) {
        for atk in attacks {
            if *atk.kind() == AttackKind::Dash
            || *atk.kind() == AttackKind::Slam {
                continue;
            }

            let rect = atk.get_rect(player_pos);

            // get attack image rotation
            let rotation_degrees: f32 = match atk.facing() {
                [0.0, 1.0] => 90.0,
                [0.0, -1.0] => -90.0,
                [1.0, 0.0] => 0.0,
                [-1.0, 0.0] => 180.0,
                [1.0, 1.0] => 45.0,
                [1.0, -1.0] => -45.0,
                [-1.0, 1.0] => 125.0,
                [-1.0, -1.0] => -125.0,
                _ => 0.0
            };

            // DrawParam.rotation needs radians
            let rotation = rotation_degrees.to_radians();

            if let Some(img) = self.attack_image.as_ref() {
                // get frame to draw
                // get height of a single frame
                let img_h = img.height() as f32;
                let frame_h = img_h / atk.frame_count() as f32;

                // normalized source rect
                let src = Rect::new(
                    0.0,
                    (atk.frame() as f32 * frame_h) / img_h,
                    1.0,
                    frame_h / img_h,
                );

                // draw frame
                let draw_param = self.drawparam_constructor(
                    // add half the width to balance offset
                    rect.x + rect.w * 0.5,
                    rect.y + rect.h * 0.5,
                )
                    // offset to be able to rotate around centre
                    .offset([0.5, 0.5])
                    .rotation(rotation)
                    .src(src);

                game_canvas.draw(img, draw_param);
            }
        }
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
                    gfx,
                    DrawMode::fill(),
                    square.rect,
                    square.color,
                )?;
                game_canvas.draw(&mesh,
                    *camera_transform);
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
        for (ti, team) in self.teams.iter().enumerate() {
            for (pi, player) in team.players.iter().enumerate() {
                if player.lives == 0 { continue; }

                let rect = player.get_rect();
                let mesh = Mesh::new_rectangle(
                    &ctx.gfx,
                    DrawMode::fill(),
                    rect,
                    player.get_color(),
                )?;
                game_canvas.draw(&mesh, camera_transform);
                let outline = Mesh::new_rectangle(
                    &ctx.gfx,
                    DrawMode::stroke(2.0),
                    rect,
                    if ti == C_TEAM && pi == C_PLAYER {
                        Color::new(0.75, 0.75, 0.75, 1.0)
                    } else {
                        Color::new(0.0, 0.0, 0.0, 1.0)
                    },
                )?;
                game_canvas.draw(&outline, camera_transform);

                let text = Text::new(TextFragment {
                    text: player.name.clone(),
                    font: None,
                    scale: Some(PxScale::from(14.0 * zoom)),
                    color: Some(NAME_COLOR),
                });

                let text_dims = text.dimensions(ctx).unwrap();
                let text_pos = Vec2::new(
                    player.pos[0] + (PLAYER_SIZE / 2.0) - (text_dims.w / 2.0),
                    player.pos[1] + 25.0,
                ) * zoom + camera_translation;

                game_canvas.draw(&text, DrawParam::default().dest(text_pos));

                self.draw_attacks(game_canvas, player.pos, &player.attacks);
                if player.parying() {
                    self.draw_pary(game_canvas, player.pos)
                }
            }
        }

        Ok(())
    }

    fn draw_hud(
        &self,
        game_canvas: &mut Canvas,
        ctx: &Context,
    ) {
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
                        "{}: {}",
                        player.name,
                        player.lives,
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
            text: format!("FPS: {fps:.0}"),
            font: None,
            scale: Some(PxScale::from(32.0)),
            ..Default::default()
        });

        let fps_dims = fps_text.dimensions(ctx).unwrap();
        let fps_x = (VIRTUAL_WIDTH - fps_dims.w as f32) / 2.0;
        let fps_y = MARGIN / 2.0;

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
                        self.team_one_color
                    } else {
                        self.team_two_color
                    }
                ),
            });

            let winner_dims = winner_text.dimensions(ctx).unwrap();
            let winner_x = (VIRTUAL_WIDTH - winner_dims.w as f32) / 2.0;
            let winner_y = (VIRTUAL_HEIGHT - winner_dims.h as f32) / 3.5;

            game_canvas.draw(
                &winner_text,
                DrawParam::default().dest(Vec2::new(
                    winner_x,
                    winner_y,
                ).to_mint_point())
            );
        }
    }
}
