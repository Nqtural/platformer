use anyhow::{Error, Result};
use client_logic::{ClientState, NetworkClient, interpolation::SnapshotHistory};
use display::render::RenderState;
use game_config::read::Config;
use ggez::glam::Vec2;
use ggez::graphics::{Canvas, Color as GgezColor, DrawParam, PxScale, Text, TextFragment};
use ggez::{
    Context, ContextBuilder, GameResult,
    event::EventHandler,
    graphics::Drawable,
    input::keyboard::{KeyCode, KeyInput},
};
use simulation::constants::{VIRTUAL_HEIGHT, VIRTUAL_WIDTH};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{
    Mutex,
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};

enum ClientView {
    Menu,
    Queue(QueueSession),
    InGame(Box<GameSession>),
}

struct QueueSession {
    event_rx: UnboundedReceiver<QueueEvent>,
}

enum QueueEvent {
    MatchFound(Box<GameSession>),
    Error(Error),
}

struct App {
    view: ClientView,
    network: NetworkClient,
    config: Config,
}

struct GameSession {
    input_tx: UnboundedSender<HashSet<KeyCode>>,
    input_state: HashSet<KeyCode>,
    snapshot_history: Arc<Mutex<SnapshotHistory>>,
    render_tick: Arc<Mutex<f32>>,
    render_state: RenderState,
    post_game: bool,
    post_game_timer: f32,
}

fn draw_centered_text(
    game_canvas: &mut Canvas,
    ctx: &Context,
    text: &str,
    scale: f32,
    y_offset: f32,
) -> GameResult {
    let (w, h) = ctx.gfx.drawable_size();
    let center = Vec2::new(w / 2.0, h / 2.0);

    let text = Text::new(TextFragment {
        text: text.to_string(),
        font: None,
        scale: Some(PxScale::from(scale)),
        color: Some(GgezColor::WHITE),
    });

    let dims = text.dimensions(ctx).unwrap_or_default();

    let pos = Vec2::new(center.x - dims.w / 2.0, center.y - dims.h / 2.0 + y_offset);

    game_canvas.draw(&text, DrawParam::default().dest(pos));

    Ok(())
}

impl App {
    async fn new(config: Config) -> Self {
        Self {
            view: ClientView::Menu,
            network: NetworkClient::new(
                config.clientip(),
                config.clientport(),
                config.serverip(),
                config.serverport(),
            )
            .await,
            config,
        }
    }

    fn start_queue(&mut self, ctx: &Context) -> Result<QueueSession> {
        let (event_tx, event_rx) = unbounded_channel();

        let network = self.network.clone();
        let config = self.config.clone();
        let render_state = RenderState::new(ctx, &config)?;

        tokio::spawn(async move {
            match App::queue_and_connect(render_state, network, &config).await {
                Ok(session) => {
                    let _ = event_tx.send(QueueEvent::MatchFound(Box::new(session)));
                }
                Err(err) => {
                    let _ = event_tx.send(QueueEvent::Error(err));
                }
            }
        });

        Ok(QueueSession { event_rx })
    }

    async fn queue_and_connect(
        render_state: RenderState,
        network: NetworkClient,
        config: &Config,
    ) -> Result<GameSession> {
        let (team_id, player_id, init_teams) = network.handshake(config.playername()).await?;

        let client = Arc::new(ClientState::new(
            team_id,
            player_id,
            init_teams,
            config.trail_delay(),
            config.trail_opacity(),
            config.trail_lifetime(),
        )?);

        // spawn networking tasks.
        network.spawn_receive_task(Arc::clone(&client));
        network.spawn_send_task(Arc::clone(&client));

        // forward keyboard input into the shared client input state.
        let current_input_write = Arc::clone(&client.current_input);

        let (input_tx, mut input_rx) = unbounded_channel::<HashSet<KeyCode>>();

        tokio::spawn(async move {
            while let Some(input) = input_rx.recv().await {
                let mut current = current_input_write.lock().await;
                *current = input;
            }
        });

        Ok(GameSession {
            input_tx,
            input_state: HashSet::new(),
            snapshot_history: Arc::clone(&client.snapshot_history),
            render_tick: Arc::clone(&client.render_tick),
            render_state,
            post_game: false,
            post_game_timer: 3.0,
        })
    }

    fn update_menu(_app: &mut App, _ctx: &mut Context) -> GameResult<Option<ClientView>> {
        Ok(None)
    }

    fn update_queue(
        _ctx: &mut Context,
        session: &mut QueueSession,
    ) -> GameResult<Option<ClientView>> {
        if let Ok(event) = session.event_rx.try_recv() {
            match event {
                QueueEvent::MatchFound(game) => {
                    return Ok(Some(ClientView::InGame(game)));
                }
                QueueEvent::Error(err) => {
                    eprintln!("{err}");
                    return Ok(Some(ClientView::Menu));
                }
            }
        }

        Ok(None)
    }

    fn update_game(ctx: &mut Context, session: &mut GameSession) -> GameResult<Option<ClientView>> {
        if !session.post_game
            && let Ok(history) = session.snapshot_history.try_lock()
            && let Some(gs) = history.latest()
            && gs.winner != 0
        {
            session.post_game = true;
        }

        if session.post_game {
            let dt = ctx.time.delta().as_secs_f32();
            session.post_game_timer -= dt;

            if session.post_game_timer <= 0.0 {
                return Ok(Some(ClientView::Menu));
            }
        }

        Ok(None)
    }

    fn draw_menu(ctx: &mut Context) -> GameResult {
        use ggez::graphics::{Canvas, Color as GgezColor};

        let mut canvas = Canvas::from_frame(&ctx.gfx, GgezColor::BLACK);

        draw_centered_text(&mut canvas, ctx, "Main Menu", 64.0, -40.0)?;
        draw_centered_text(&mut canvas, ctx, "Press R to queue", 28.0, 40.0)?;

        canvas.finish(&mut ctx.gfx)
    }

    fn draw_queue(ctx: &mut Context, _session: &mut QueueSession) -> GameResult {
        use ggez::graphics::{Canvas, Color as GgezColor};

        let mut canvas = Canvas::from_frame(&ctx.gfx, GgezColor::BLACK);

        draw_centered_text(&mut canvas, ctx, "Queuing...", 48.0, -20.0)?;
        draw_centered_text(&mut canvas, ctx, "Press Esc to cancel", 24.0, 40.0)?;

        canvas.finish(&mut ctx.gfx)
    }

    fn draw_game(ctx: &mut Context, session: &mut GameSession) -> GameResult {
        let history = match session.snapshot_history.try_lock() {
            Ok(history) => history,
            Err(_) => return Ok(()), // skip this frame
        };

        let render_tick = match session.render_tick.try_lock() {
            Ok(render_tick) => render_tick,
            Err(_) => return Ok(()), // skip this frame
        };

        if let Some(game_state) = history.get_interpolated(*render_tick) {
            session.render_state.render(ctx, &game_state)?;
        }

        Ok(())
    }
}

impl EventHandler for App {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        let transition = match &mut self.view {
            ClientView::Menu => App::update_menu(self, ctx)?,
            ClientView::Queue(session) => App::update_queue(ctx, session)?,
            ClientView::InGame(session) => App::update_game(ctx, session)?,
        };

        if let Some(new_view) = transition {
            self.view = new_view;
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        match &mut self.view {
            ClientView::Menu => App::draw_menu(ctx),
            ClientView::Queue(session) => App::draw_queue(ctx, session),
            ClientView::InGame(session) => App::draw_game(ctx, session),
        }
    }

    fn key_down_event(&mut self, ctx: &mut Context, input: KeyInput, _repeat: bool) -> GameResult {
        if let Some(keycode) = input.keycode {
            match &mut self.view {
                ClientView::Menu => match keycode {
                    KeyCode::R => {
                        self.view = ClientView::Queue(
                            self.start_queue(ctx).expect("Fatal: Failed to start queue"),
                        )
                    }
                    KeyCode::Q => panic!("Exiting..."), // exit hack, TODO
                    _ => {}
                },
                ClientView::Queue(_) => match keycode {
                    KeyCode::Escape => self.view = ClientView::Menu,
                    _ => {}
                },
                ClientView::InGame(session) => {
                    session.input_state.insert(keycode);
                    let _ = session.input_tx.send(session.input_state.clone());
                }
            }
        }

        Ok(())
    }

    fn key_up_event(&mut self, _ctx: &mut Context, input: KeyInput) -> GameResult {
        if let Some(keycode) = input.keycode {
            match &mut self.view {
                ClientView::InGame(session) => {
                    session.input_state.remove(&keycode);
                    let _ = session.input_tx.send(session.input_state.clone());
                }
                _ => {}
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::get()?;
    let app = App::new(config).await;

    let (ctx, event_loop) = ContextBuilder::new("platform", "Nqtural")
        .window_setup(
            ggez::conf::WindowSetup::default()
                .vsync(app.config.vsync())
                .title("Game"),
        )
        .window_mode(
            ggez::conf::WindowMode::default()
                .dimensions(VIRTUAL_WIDTH, VIRTUAL_HEIGHT)
                .resizable(true),
        )
        .build()?;

    ggez::event::run(ctx, event_loop, app);
}
