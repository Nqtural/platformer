use anyhow::Result;
use ggez::input::keyboard::KeyCode;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::Mutex;
use crate::utils::{
    color_to_ggez,
    IntoMint,
    rect_to_ggez,
};
use ggez::{
    Context,
    event::EventHandler,
    GameResult,
    graphics::{
        Canvas,
        Color as GgezColor,
        GraphicsContext,
            Drawable,
        DrawMode,
        DrawParam,
        Image,
        ImageFormat,
        Mesh,
        PxScale,
        Rect as GgezRect,
        Text,
        TextFragment,
    },
    input::keyboard::KeyInput,
};
use glam::Vec2;
use client_logic::interpolation::SnapshotHistory;
use client_logic::ClientState;
use game_config::read::Config;
use foundation::color::Color;
use simulation::{
    attack::{
        Attack,
        AttackKind,
    },
    constants::{
        FIXED_DT,
        NAME_COLOR,
        PLAYER_SIZE,
        VIRTUAL_HEIGHT,
        VIRTUAL_WIDTH,
    },
    game_state::GameState,
};
use crate::input::InputState;

pub struct Renderer {
    render_state: RenderState,
    snapshot_history: Arc<Mutex<SnapshotHistory>>,
    render_tick: Arc<Mutex<f32>>,

    client_state: Arc<ClientState>,
    predicted_snapshot_history: SnapshotHistory,

    // INPUT
    input_state: InputState,
    input_tx: tokio::sync::mpsc::UnboundedSender<HashSet<KeyCode>>,
}

impl Renderer {
    pub fn new(
        ctx: &Context,
        snapshot_history: Arc<Mutex<SnapshotHistory>>,
        render_tick: Arc<Mutex<f32>>,
        client_state: Arc<ClientState>,
        input_tx: tokio::sync::mpsc::UnboundedSender<HashSet<KeyCode>>,
    ) -> Result<Self> {
        Ok(Self {
            render_state: RenderState::new(ctx)?,
            snapshot_history,
            render_tick,
            client_state,
            predicted_snapshot_history: SnapshotHistory::default(),
            input_state: InputState::new(),
            input_tx,
        })
    }

    pub fn render(&mut self, ctx: &mut Context) -> GameResult {
        let state = if let (Ok(history), Ok(render_tick)) =
        (self.snapshot_history.try_lock(), self.render_tick.try_lock())
        {
            let mut gs = history.get_interpolated(*render_tick);
            gs.update_local_player(
                self.predicted_snapshot_history.get_latest_local_player()
            );

            gs
        } else {
            self.predicted_snapshot_history.get_last().clone()
        };

        self.render_state.render(ctx, &state)?;

        Ok(())
    }
}

impl EventHandler for Renderer {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        let dt = ctx.time.delta().as_secs_f32();

        let mut core = match self.client_state.core.try_lock() {
            Ok(core) => core,
            Err(_) => return Ok(()),
        };

        let mut tick_accumulator = match self.client_state.tick_accumulator.try_lock() {
            Ok(tick_accumulator) => tick_accumulator,
            Err(_) => return Ok(()),
        };

        *tick_accumulator += dt;

        while *tick_accumulator >= FIXED_DT {
            *tick_accumulator -= FIXED_DT;

            // Increment simulation tick
            self.client_state.tick.fetch_add(1, Ordering::Relaxed);

            // Apply inputs for this tick
            core.step(FIXED_DT);

            // Store in predicted buffer
            self.predicted_snapshot_history.push(
                self.client_state.tick.load(Ordering::SeqCst),
                core.game_state().clone()
            );
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        self.render(ctx)
    }

    // INPUT
    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        key: KeyInput,
        _: bool,
    ) -> GameResult {
        if let Some(keycode) = key.keycode {
            self.input_state.press(keycode);
            let _ = self.input_tx.send(self.input_state.pressed.clone()); // send to async task
        }

        Ok(())
    }

    fn key_up_event(
        &mut self,
        _ctx: &mut Context,
        key: KeyInput,
    ) -> GameResult {
        if let Some(keycode) = key.keycode {
            self.input_state.release(keycode);
            let _ = self.input_tx.send(self.input_state.pressed.clone()); // send to async task
        }

        Ok(())
    }
}

struct RenderState {
    team_one_color: Color,
    team_two_color: Color,
    player_name_above: bool,
    zoom: f32,
    camera_pos: Vec2,
    bias_strength: f32,
    background_image: Option<Image>,
    attack_image: Option<Image>,
    parry_image: Option<Image>,
}

impl RenderState {
    pub fn new(ctx: &Context) -> Result<Self> {
        let config = Config::get().expect("Unable to get config file");
        let bg_img = Image::from_bytes(&ctx.gfx, &config.background_image()?)?;
        let attack_img = Image::from_bytes(&ctx.gfx, &config.attack_image()?)?;
        let parry_img = Image::from_bytes(&ctx.gfx, &config.parry_image()?)?;

        Ok(Self {
            camera_pos: Vec2::new(0.0, 0.0),
            bias_strength: config.camera_bias(),
            team_one_color: config.team_one_color(),
            team_two_color: config.team_two_color(),
            player_name_above: config.player_name_above(),
            zoom: config.camera_zoom(),
            background_image: Some(bg_img),
            attack_image: Some(attack_img),
            parry_image: Some(parry_img),
        })
    }

    pub fn render(
        &mut self,
        ctx: &mut Context,
        gs: &GameState,
    ) -> GameResult {
        self.update_camera(gs);

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
            GgezColor::new(0.1, 0.1, 0.15, 1.0),
        );
        game_canvas.set_screen_coordinates(
            GgezRect::new(
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

        let screen_center = Vec2::new(
            VIRTUAL_WIDTH / 2.0,
            VIRTUAL_HEIGHT / 2.0
        );
        let camera_translation = screen_center - self.camera_pos * self.zoom;

        let camera_transform = DrawParam::default()
            .dest(camera_translation)
            .scale(Vec2::new(self.zoom, self.zoom).to_mint_vec());

        self.draw_map(&mut game_canvas, &mut ctx.gfx, &camera_transform, gs)?;
        self.draw_trails(&mut game_canvas, &mut ctx.gfx, &camera_transform, gs)?;
        self.draw_players(&mut game_canvas, ctx, &camera_translation, gs)?;
        self.draw_hud(&mut game_canvas, ctx, gs);

        // DEBUG
        //let _ = gs.draw_attack_hurtbox(&mut game_canvas, &ctx.gfx, camera_transform);

        game_canvas.finish(&mut ctx.gfx)?;

        let mut final_canvas = Canvas::from_frame(&ctx.gfx, GgezColor::BLACK);

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

    fn update_camera(&mut self, gs: &GameState) {
        let mut sum = Vec2::ZERO;
        let mut count: usize = 0;

        for team in &gs.teams {
            for player in &team.players {
                if !player.combat.is_alive() { continue; }
                sum += player.physics.pos + PLAYER_SIZE / 2.0;
                count += 1;
            }
        }

        if count == 0 { return; }

        let player_center = sum / count as f32;

        let map_rect = gs.map.get_rect();
        let map_center = Vec2::new(
            map_rect.x + map_rect.w / 2.0,
            map_rect.y + map_rect.h / 2.0,
        );

        let biased_target = player_center.lerp(map_center, self.bias_strength);

        let lerp_factor = 0.1;
        self.camera_pos = self.camera_pos.lerp(biased_target, lerp_factor);
    }


    fn drawparam_constructor(&self, pos: Vec2) -> DrawParam {
        let screen_center = Vec2::new(VIRTUAL_WIDTH / 2.0, VIRTUAL_HEIGHT / 2.0);

        DrawParam::default()
            .dest(
                screen_center 
                + pos * self.zoom 
                - self.camera_pos * self.zoom
            )
            .scale(Vec2::new(self.zoom, self.zoom).to_mint_vec())
    }

    fn draw_map(
        &self,
        game_canvas: &mut Canvas,
        gfx: &mut GraphicsContext,
        camera_transform: &DrawParam,
        gs: &GameState,
    ) -> GameResult {
        let map_mesh = Mesh::new_rectangle(
            gfx,
            DrawMode::fill(),
            rect_to_ggez(gs.map.get_rect()),
            color_to_ggez(&gs.map.get_color()),
        )?;
        game_canvas.draw(&map_mesh, *camera_transform);

        Ok(())
    }

    fn draw_parry(
        &self,
        game_canvas: &mut Canvas,
        player_pos: Vec2,
    ) {
        if let Some(img) = self.parry_image.as_ref() {
            // draw frame
            let draw_param = self.drawparam_constructor(
                (player_pos + PLAYER_SIZE / 2.0) - (img.width() as f32 / 2.0),
            );

            game_canvas.draw(img, draw_param);
        }
    }

    fn draw_attacks(
        &self,
        game_canvas: &mut Canvas,
        player_pos: Vec2,
        attacks: &[Attack],
    ) {
        for atk in attacks {
            if *atk.kind() == AttackKind::Dash
            || *atk.kind() == AttackKind::Slam {
                continue;
            }

            let rect = atk.get_rect(player_pos);

            // get attack image rotation
            let facing: Vec2 = atk.facing();
            let rotation_radians = facing.y.atan2(facing.x);
            let rotation_degrees = rotation_radians.to_degrees();

            // DrawParam.rotation needs radians
            let rotation = rotation_degrees.to_radians();

            if let Some(img) = self.attack_image.as_ref() {
                // get frame to draw
                // get height of a single frame
                let img_h = img.height() as f32;
                let frame_h = img_h / atk.frame_count() as f32;

                // normalized source rect
                let src = GgezRect::new(
                    0.0,
                    (atk.frame() as f32 * frame_h) / img_h,
                    1.0,
                    frame_h / img_h,
                );

                // draw frame
                let draw_param = self.drawparam_constructor(
                    // add half the width to balance offset
                    Vec2::new(
                        rect.x + rect.w * 0.5,
                        rect.y + rect.h * 0.5,
                    ),
                )
                    // offset to be able to rotate around centre
                    .offset([0.5, 0.5])
                    .rotation(rotation)
                    .src(src);

                game_canvas.draw(img, draw_param);
            }
        }
    }

    fn _draw_attack_hurtbox(
        &self,
        game_canvas: &mut Canvas,
        gfx: &GraphicsContext,
        camera_transform: DrawParam,
        gs: &GameState,
    ) -> GameResult<()> {
        for team in &gs.teams {
            for player in &team.players {
                for attack in &player.combat.attacks {
                    let mesh = Mesh::new_rectangle(
                        gfx,
                        DrawMode::stroke(1.0),
                        rect_to_ggez(&attack.get_rect(player.physics.pos)),
                        GgezColor::new(1.0, 1.0, 1.0, 0.4),
                    )?;
                    game_canvas.draw(&mesh, camera_transform);
                }
            }
        }

        Ok(())
    }

    fn draw_trails(
        &self,
        game_canvas: &mut Canvas,
        gfx: &mut GraphicsContext,
        camera_transform: &DrawParam,
        gs: &GameState,
    ) -> GameResult {
        for team in &gs.teams {
            for player in &team.players {
                for square in &player.visuals.trail_squares {
                    let mesh = Mesh::new_rectangle(
                        gfx,
                        DrawMode::fill(),
                        rect_to_ggez(&square.rect),
                        color_to_ggez(&square.color),
                    )?;
                    game_canvas.draw(&mesh, *camera_transform);
                }
            }
        }

        Ok(())
    }


    fn draw_players(
        &self,
        game_canvas: &mut Canvas,
        ctx: &mut Context,
        camera_translation: &Vec2,
        gs: &GameState,
    ) -> GameResult {
        let camera_transform = DrawParam::default()
            .dest(*camera_translation)
            .scale(Vec2::new(self.zoom, self.zoom).to_mint_vec());
        for (ti, team) in gs.teams.iter().enumerate() {
            for (pi, player) in team.players.iter().enumerate() {
                if !player.combat.is_alive() { continue; }

                let rect = player.physics.get_rect();
                let mesh = Mesh::new_rectangle(
                    &ctx.gfx,
                    DrawMode::fill(),
                    rect_to_ggez(&rect),
                    color_to_ggez(&player.get_color()),
                )?;
                game_canvas.draw(&mesh, camera_transform);
                let outline = Mesh::new_rectangle(
                    &ctx.gfx,
                    DrawMode::stroke(2.0),
                    rect_to_ggez(&rect),
                    if ti == gs.c_team && pi == gs.c_player {
                        GgezColor::new(0.75, 0.75, 0.75, 1.0)
                    } else {
                        GgezColor::new(0.0, 0.0, 0.0, 1.0)
                    },
                )?;
                game_canvas.draw(&outline, camera_transform);

                let text = Text::new(TextFragment {
                    text: player.identity.name().to_string(),
                    font: None,
                    scale: Some(PxScale::from(14.0)),
                    color: Some(color_to_ggez(&NAME_COLOR)),
                });

                let text_dims = text.dimensions(ctx).unwrap();

                let draw_param = self.drawparam_constructor(
                    Vec2::new(
                        player.physics.pos.x + (PLAYER_SIZE / 2.0) - (text_dims.w / 2.0),
                        if self.player_name_above {
                            player.physics.pos.y - (text_dims.h * 1.5)
                        } else {
                            player.physics.pos.y + PLAYER_SIZE + (text_dims.h / 2.0)
                        },
                    ),
                );
                game_canvas.draw(&text, draw_param);

                if player.combat.combo > 0 {
                    let combo_number = Text::new(TextFragment {
                        text: format!("{}", player.combat.combo),
                        font: None,
                        scale: Some(PxScale::from(20.0)),
                        color: Some(GgezColor::new(1.0, 1.0, 1.0, 1.0)),
                    });
                    let draw_param_number = self.drawparam_constructor(
                        Vec2::new(
                            player.physics.pos.x + PLAYER_SIZE + 5.0,
                            player.physics.pos.y - 10.0,
                        ),
                    );
                    game_canvas.draw(&combo_number, draw_param_number);
                }

                self.draw_attacks(game_canvas, player.physics.pos, &player.combat.attacks);

                if player.status.parrying() {
                    self.draw_parry(game_canvas, player.physics.pos)
                }
            }
        }

        Ok(())
    }

    fn draw_hud(
        &self,
        game_canvas: &mut Canvas,
        ctx: &Context,
        gs: &GameState,
    ) {
        const MARGIN: f32 = 40.0;
        const START_X_LEFT: f32 = MARGIN;
        const START_X_RIGHT: f32 = VIRTUAL_WIDTH - MARGIN;
        const START_Y: f32 = MARGIN;
        const LINE_HEIGHT: f32 = MARGIN;

        for (team_idx, team) in gs.teams.iter().enumerate() {
            let is_right_team = team_idx == 1;

            for (player_idx, player) in team.players.iter().enumerate() {
                let y = START_Y + player_idx as f32 * LINE_HEIGHT;
                let text = Text::new(TextFragment {
                    text: format!(
                        "{}: {}",
                        player.identity.name(),
                        player.combat.lives,
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

        if gs.winner > 0 {
            let winner_text = Text::new(TextFragment {
                text: format!("TEAM {} WINS!", gs.winner),
                font: None,
                scale: Some(PxScale::from(200.0)),
                color: Some(
                    if gs.winner == 1 {
                        color_to_ggez(&self.team_one_color)
                    } else {
                        color_to_ggez(&self.team_two_color)
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
