use crate::attack::Attack;
use crate::constants::*;
use crate::map::Map;
use crate::team::Team;
use crate::traits::IntoMint;
use ggez::{
    Context,
    event::EventHandler,
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
    input::keyboard::KeyInput,
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
    pub active_attacks: Vec<Attack>,
    pub camera_pos: Vec2,
    pub winner: usize,
}

impl GameState {
    pub fn new(teams: [Team; 2]) -> GameState {
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
            let owner_team_idx = atk.owner_team();
            let owner_player_idx = atk.owner_player();

            let (left, right) = self.teams.split_at_mut(owner_team_idx);
            let (owner_team, other_teams) = right.split_first_mut().unwrap();

            let owner_player = &mut owner_team.players[owner_player_idx];

            for team in left.iter_mut().chain(other_teams.iter_mut()) {
                for player in &mut team.players {
                    if player.stunned > 0.0
                    || player.invulnerable_timer > 0.0
                    || !atk.get_rect().overlaps(&player.get_rect())
                    {
                        continue;
                    }

                    atk.attack(owner_player, player);
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

        let map_rect = self.map.get_rect();
        let map_center = Vec2::new(
            map_rect.x + map_rect.w / 2.0,
            map_rect.y + map_rect.h / 2.0,
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
            self.map.get_rect(),
            self.map.get_color(),
        )?;
        game_canvas.draw(&map_mesh, *camera_transform);
        Ok(())
    }
    
    fn draw_attacks(
        &self,
        game_canvas: &mut Canvas,
        gfx: &mut GraphicsContext,
        camera_transform: &DrawParam,
    ) -> GameResult {
        for atk in &self.active_attacks {
            let mesh = Mesh::new_rectangle(
                gfx,
                DrawMode::stroke(1.0),
                atk.get_rect(),
                Color::new(1.0, 0.0, 0.0, 0.4),
            )?;
            game_canvas.draw(&mesh, *camera_transform);
        }

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
                    gfx,
                    DrawMode::fill(),
                    square.rect,
                    square.color,
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

                let rect = player.get_rect();
                let mesh = Mesh::new_rectangle(
                    &ctx.gfx,
                    DrawMode::fill(),
                    rect,
                    team.get_color(
                        player.invulnerable_timer > 0.0,
                        player.stunned > 0.0,
                    )
                )?;
                game_canvas.draw(&mesh, camera_transform);
                let outline = Mesh::new_rectangle(
                    &ctx.gfx,
                    DrawMode::stroke(2.0),
                    rect,
                    Color::new(0.0, 0.0, 0.0, 1.0)
                )?;
                game_canvas.draw(&outline, camera_transform);

                //Attackbox
                let mesh = Mesh::new_rectangle(
                    &ctx.gfx,
                    DrawMode::stroke(1.0),
                    Rect::new(
                        player.pos[0] - 5.0,
                        player.pos[1] - 5.0,
                        PLAYER_SIZE + 10.0,
                        PLAYER_SIZE + 10.0,
                    ),
                    Color::new(1.0, 1.0, 1.0, 0.4),
                )?;
                game_canvas.draw(&mesh, camera_transform);

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
            attack.update(dt);
        }
        self.active_attacks.retain(|atk| !atk.is_expired());

        self.handle_attack_collisions();

        self.check_for_win();

        for team_idx in 0..self.teams.len() {
            let (left, right) = self.teams.split_at_mut(team_idx);
            let (team, others) = right.split_first_mut().unwrap();

            team.update_players(
                left,
                others,
                team_idx,
                &self.map.get_rect(),
                self.winner,
                &mut self.active_attacks,
                dt,
            );
        }

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
        self.draw_attacks(&mut game_canvas, &mut ctx.gfx, &camera_transform)?;
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
        _ctx: &mut Context,
        key: KeyInput,
        _repeated: bool,
    ) -> GameResult {
        if let Some(keycode) = key.keycode {
            let input = &mut self.teams[C_TEAM].players[C_PLAYER].input;
            input.update(keycode, true)
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
            input.update(keycode, false)
        }

        Ok(())
    }
}
