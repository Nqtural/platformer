use anyhow::{Error, Result};
use client_logic::{ClientState, GameSession, NetworkClient};
use display::menus;
use display::render::RenderState;
use game_config::read::Config;
use ggez::{
    Context, ContextBuilder, GameResult,
    event::EventHandler,
    input::keyboard::{KeyCode, KeyInput},
};
use simulation::constants::{VIRTUAL_HEIGHT, VIRTUAL_WIDTH};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

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

        Ok(GameSession::new(
            input_tx,
            Arc::clone(&client.snapshot_history),
            Arc::clone(&client.render_tick),
            render_state,
            config.team_size(),
        ))
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
        let dt = ctx.time.delta().as_secs_f32();
        if session.has_ended(dt) {
            return Ok(Some(ClientView::Menu));
        }

        session.update_replay();

        Ok(None)
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
            ClientView::Menu => menus::draw_menu(ctx),
            ClientView::Queue(_) => menus::draw_queue(ctx),
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
                ClientView::InGame(session) => session.press(keycode),
            }
        }

        Ok(())
    }

    fn key_up_event(&mut self, _ctx: &mut Context, input: KeyInput) -> GameResult {
        if let Some(keycode) = input.keycode {
            match &mut self.view {
                ClientView::InGame(session) => session.release(&keycode),
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
