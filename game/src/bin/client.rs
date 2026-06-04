use anyhow::Result;
use client_logic::replay::list_view::ReplayListView;
use client_logic::replay::recorder::ReplayRecorder;
use client_logic::replay::viewer::ReplayViewer;
use client_logic::{ClientEvent, ClientState, GameSession, NetworkClient};
use display::menus;
use display::render::RenderState;
use foundation::GameMode;
use game_config::read::Config;
use ggez::{
    Context, ContextBuilder, GameResult,
    event::EventHandler,
    input::keyboard::{KeyCode, KeyInput},
};
use protocol::net_server::ServerMessage;
use simulation::constants::{VIRTUAL_HEIGHT, VIRTUAL_WIDTH};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::{
    sync::mpsc::{UnboundedReceiver, unbounded_channel},
    task::JoinHandle,
};

enum ClientView {
    Menu,
    Queue(QueueController),
    InGame {
        session: Box<GameSession>,
        client: Arc<ClientState>,
    },
    ReplayPicker(ReplayListView),
    ReplayView(Box<ReplayViewer>),
}

struct QueueController {
    event_rx: UnboundedReceiver<QueueEvent>,
    task: JoinHandle<()>,
}

struct InitialGameData {
    c_team_id: usize,
    c_player_id: usize,
    player_names: [Vec<String>; 2],
}

enum QueueEvent {
    MatchFound(InitialGameData),
}

struct App {
    view: ClientView,
    network: Arc<NetworkClient>,
    config: Config,
}

impl App {
    async fn new(config: Config) -> Result<Self> {
        let network = Arc::new(
            NetworkClient::new(
                config.clientip(),
                config.clientport(),
                config.serverip(),
                config.serverport(),
            )
            .await,
        );

        network.handshake(config.playername()).await?;

        Ok(Self {
            view: ClientView::Menu,
            network,
            config,
        })
    }

    fn start_queue(&self, _ctx: &Context, mode: GameMode) -> Result<QueueController> {
        let (event_tx, event_rx) = unbounded_channel();

        let network = Arc::clone(&self.network);
        tokio::spawn(async move {
            if let Err(e) = network.enter_queue(mode).await {
                eprintln!("Failed to join queue: {e}");
            }
        });

        let network = Arc::clone(&self.network);
        let task = tokio::spawn({
            async move {
                loop {
                    match network.poll_queue().await {
                        Ok(ServerMessage::StartGame {
                            c_team_id,
                            c_player_id,
                            player_names,
                        }) => {
                            let _ = event_tx.send(QueueEvent::MatchFound(InitialGameData {
                                c_team_id,
                                c_player_id,
                                player_names,
                            }));
                            break;
                        }
                        _ => {}
                    }
                }
            }
        });

        Ok(QueueController { event_rx, task })
    }

    fn update_menu(_app: &mut App, _ctx: &mut Context) -> GameResult<Option<ClientView>> {
        Ok(None)
    }

    fn update_queue(
        ctx: &mut Context,
        controller: &mut QueueController,
        config: &Config,
        network: Arc<NetworkClient>,
    ) -> GameResult<Option<ClientView>> {
        if let Ok(event) = controller.event_rx.try_recv() {
            match event {
                QueueEvent::MatchFound(data) => {
                    let render_state = match RenderState::new(ctx, config) {
                        Ok(render_state) => render_state,
                        Err(e) => {
                            eprintln!("Error initializing render_state: {e}");
                            return Ok(Some(ClientView::Menu));
                        }
                    };

                    let replay_recorder = ReplayRecorder::new(data.player_names.clone());

                    let client = Arc::new(
                        match ClientState::new(
                            data.c_team_id,
                            data.c_player_id,
                            data.player_names,
                            config.trail_delay(),
                            config.trail_opacity(),
                            config.trail_lifetime(),
                        ) {
                            Ok(client) => client,
                            Err(e) => {
                                eprintln!("Unable to initialize client: {e}");
                                return Ok(Some(ClientView::Menu));
                            }
                        },
                    );

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

                    let session = Box::new(GameSession::new(
                        input_tx,
                        Arc::clone(&client.snapshot_history),
                        Arc::clone(&client.render_tick),
                        render_state,
                        replay_recorder,
                    ));

                    return Ok(Some(ClientView::InGame { session, client }));
                }
            }
        }

        Ok(None)
    }

    fn update_game(
        client: &ClientState,
        session: &mut GameSession,
    ) -> GameResult<Option<ClientView>> {
        if client.event_rx.has_changed().unwrap()
            && let Some(ClientEvent::EndGame) = client.event_rx.borrow().clone()
        {
            session.save_replay();
            return Ok(Some(ClientView::Menu));
        }

        session.update_replay();

        Ok(None)
    }

    fn update_replay_picker(
        _ctx: &mut Context,
        _list_view: &mut ReplayListView,
    ) -> GameResult<Option<ClientView>> {
        Ok(None)
    }

    fn update_replay_viewer(
        ctx: &mut Context,
        replay_viewer: &mut ReplayViewer,
    ) -> GameResult<Option<ClientView>> {
        let dt = ctx.time.delta().as_secs_f32();

        replay_viewer.update(dt);

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
            ClientView::Queue(controller) => {
                App::update_queue(ctx, controller, &self.config, Arc::clone(&self.network))?
            }
            ClientView::InGame { session, client } => App::update_game(client, session)?,
            ClientView::ReplayPicker(list_view) => App::update_replay_picker(ctx, list_view)?,
            ClientView::ReplayView(replay_viewer) => App::update_replay_viewer(ctx, replay_viewer)?,
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
            ClientView::InGame { session, client: _ } => App::draw_game(ctx, session),
            ClientView::ReplayPicker(list_view) => menus::draw_replay_picker(
                ctx,
                list_view.get_current_page_items_pretty(),
                list_view.get_selected_row_index(),
                list_view.current_page() + 1,
                list_view.total_pages(),
            ),
            ClientView::ReplayView(replay_viewer) => replay_viewer
                .render_state
                .render(ctx, &replay_viewer.get_current_state()),
        }
    }

    fn key_down_event(&mut self, ctx: &mut Context, input: KeyInput, _repeat: bool) -> GameResult {
        if let Some(keycode) = input.keycode {
            match &mut self.view {
                ClientView::Menu => match keycode {
                    KeyCode::Key1 => {
                        self.view =
                            ClientView::Queue(match App::start_queue(self, ctx, GameMode::Solos) {
                                Ok(controller) => controller,
                                Err(e) => {
                                    eprintln!("Failed to start queue: {e}");
                                    return Ok(());
                                }
                            });
                    }
                    KeyCode::Key2 => {
                        self.view =
                            ClientView::Queue(match App::start_queue(self, ctx, GameMode::Duos) {
                                Ok(controller) => controller,
                                Err(e) => {
                                    eprintln!("Failed to start queue: {e}");
                                    return Ok(());
                                }
                            });
                    }
                    KeyCode::R => {
                        self.view = ClientView::ReplayPicker(match ReplayListView::new() {
                            Ok(list_view) => list_view,
                            Err(e) => {
                                eprintln!("Failed to initialize replay list view: {e}");
                                return Ok(());
                            }
                        })
                    }
                    KeyCode::Q => panic!("Exiting..."), // exit hack, TODO
                    _ => {}
                },
                ClientView::Queue(controller) => match keycode {
                    KeyCode::Escape => {
                        controller.task.abort();
                        let network = Arc::clone(&self.network);
                        tokio::spawn(async move {
                            let _ = network.leave_queue().await;
                        });
                        self.view = ClientView::Menu;
                    }
                    _ => {}
                },
                ClientView::InGame { session, client: _ } => session.press(keycode),
                ClientView::ReplayPicker(list_view) => match keycode {
                    KeyCode::Q => self.view = ClientView::Menu,
                    KeyCode::H | KeyCode::Left => list_view.left(),
                    KeyCode::J | KeyCode::Down => list_view.down(),
                    KeyCode::K | KeyCode::Up => list_view.up(),
                    KeyCode::L | KeyCode::Right => list_view.right(),
                    KeyCode::Return => {
                        self.view = ClientView::ReplayView(
                            match ReplayViewer::new(
                                match RenderState::new(ctx, &self.config) {
                                    Ok(render_state) => render_state,
                                    Err(e) => {
                                        eprintln!("Failed to start renderer for replay: {e}");
                                        return Ok(());
                                    }
                                },
                                &match list_view.selected() {
                                    Some(replay_path) => replay_path,
                                    None => {
                                        eprintln!("Failed to get selected replay file path");
                                        return Ok(());
                                    }
                                },
                                self.config.trail_delay(),
                                self.config.trail_opacity(),
                                self.config.trail_lifetime(),
                            ) {
                                Ok(replay_viewer) => Box::new(replay_viewer),
                                Err(e) => {
                                    eprintln!("Failed to initialize replay viewer: {e}");
                                    return Ok(());
                                }
                            },
                        )
                    }
                    _ => {}
                },
                ClientView::ReplayView(replay_viewer) => match keycode {
                    KeyCode::Space => replay_viewer.toggle_pause(),
                    KeyCode::Comma => replay_viewer.previous_tick(),
                    KeyCode::Period => replay_viewer.next_tick(),
                    KeyCode::Left => replay_viewer.seek_backwards(),
                    KeyCode::Right => replay_viewer.seek_forwards(),
                    KeyCode::Up => replay_viewer.speed_increase(),
                    KeyCode::Down => replay_viewer.speed_decrease(),
                    KeyCode::Q => self.view = ClientView::Menu,
                    _ => {}
                },
            }
        }

        Ok(())
    }

    fn key_up_event(&mut self, _ctx: &mut Context, input: KeyInput) -> GameResult {
        if let Some(keycode) = input.keycode {
            match &mut self.view {
                ClientView::InGame { session, client: _ } => session.release(&keycode),
                _ => {}
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::get()?;
    let app = App::new(config).await?;

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
