use anyhow::{Error, Result};
use client_logic::replay::constants::{REPLAY_DIRECTORY, REPLAY_LIST_ROWS};
use client_logic::replay::recorder::ReplayRecorder;
use client_logic::replay::viewer::ReplayViewer;
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
use std::fs;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

struct ReplayListView {
    replay_files: Vec<String>,
    page: usize,
    row: usize,
}

impl ReplayListView {
    pub fn new() -> Result<Self> {
        Ok(Self {
            replay_files: Self::load_replay_files()?,
            page: 0,
            row: 0,
        })
    }

    pub fn get_current_page_items_pretty(&self) -> Vec<String> {
        let start = self.page * REPLAY_LIST_ROWS;

        (start..start + REPLAY_LIST_ROWS)
            .filter_map(|i| {
                let f = self.replay_files.get(i)?;

                let f = f.strip_prefix(REPLAY_DIRECTORY)?;
                let f = f.strip_suffix(".prp")?;

                Some(f.to_string())
            })
            .collect()
    }

    pub fn get_selected_row_index(&self) -> usize {
        self.row
    }

    pub fn current_page(&self) -> usize {
        self.page
    }

    pub fn down(&mut self) {
        let max_row = if self.page == self.total_pages() - 1 {
            self.items_on_last_page()
        } else {
            REPLAY_LIST_ROWS
        };

        if self.row + 1 < max_row {
            self.row += 1;
        } else {
            let old_page = self.page;
            self.right();

            if old_page != self.page {
                self.row = 0;
            }
        }
    }

    pub fn up(&mut self) {
        if self.row > 0 {
            self.row -= 1;
        } else {
            let old_page = self.page;
            self.left();
            if old_page != self.page {
                self.row = REPLAY_LIST_ROWS - 1;
            }
        }
    }

    pub fn left(&mut self) {
        if self.page > 0 {
            self.page -= 1;
        }
    }

    pub fn right(&mut self) {
        if self.page < self.total_pages() - 1 {
            self.page += 1;
        }

        if self.page == self.total_pages() - 1 && self.row > self.items_on_last_page() {
            self.row = self.items_on_last_page() - 1;
        }
    }

    pub fn selected(&self) -> Option<String> {
        let index = self.page * REPLAY_LIST_ROWS + self.row;
        self.replay_files.get(index).cloned()
    }

    fn total_pages(&self) -> usize {
        self.replay_files.len().div_ceil(REPLAY_LIST_ROWS)
    }

    fn items_on_last_page(&self) -> usize {
        let length = self.replay_files.len();
        let rem = length % REPLAY_LIST_ROWS;
        if rem == 0 && length != 0 {
            REPLAY_LIST_ROWS
        } else {
            rem
        }
    }

    fn load_replay_files() -> Result<Vec<String>> {
        let mut files = Vec::new();

        for entry in fs::read_dir(REPLAY_DIRECTORY)? {
            let path = entry?.path();

            if path.is_file()
                && let Some(ext) = path.extension()
                && ext == "prp"
                && let Some(path_str) = path.to_str()
            {
                files.push(path_str.to_string());
            }
        }

        Ok(files)
    }
}

enum ClientView {
    Menu,
    Queue(QueueSession),
    InGame(Box<GameSession>),
    ReplayPicker(ReplayListView),
    ReplayView(Box<ReplayViewer>),
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

        let replay_recorder = ReplayRecorder::new(init_teams.clone());

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
            replay_recorder,
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
            ClientView::Queue(session) => App::update_queue(ctx, session)?,
            ClientView::InGame(session) => App::update_game(ctx, session)?,
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
            ClientView::InGame(session) => App::draw_game(ctx, session),
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
                    KeyCode::Space => {
                        self.view = ClientView::Queue(match self.start_queue(ctx) {
                            Ok(session) => session,
                            Err(e) => {
                                eprintln!("Failed to start queue: {e}");
                                return Ok(());
                            }
                        })
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
                ClientView::Queue(_) => match keycode {
                    KeyCode::Escape => self.view = ClientView::Menu,
                    _ => {}
                },
                ClientView::InGame(session) => session.press(keycode),
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
