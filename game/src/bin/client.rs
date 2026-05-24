use anyhow::Result;
use client_logic::{ClientState, NetworkClient};
use game_config::read::Config;
use ggez::{
    Context, ContextBuilder, GameResult,
    event::EventHandler,
    input::keyboard::{KeyCode, KeyInput},
};
use simulation::constants::{VIRTUAL_HEIGHT, VIRTUAL_WIDTH};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

enum ClientView {
    Menu,
    Queue,
    InGame(GameSession),
}

struct App {
    view: ClientView,
    network: NetworkClient,
    config: Config,
}

struct GameSession {
    client: Arc<ClientState>,
    input_tx: UnboundedSender<HashSet<KeyCode>>,
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

    fn update_menu(app: &mut App, _ctx: &mut Context) -> GameResult<Option<ClientView>> {
        Ok(None)
        // let (team_id, player_id, init_teams) =
        //     self.network.handshake(self.config.playername()).await?;

        // let client = Arc::new(ClientState::new(
        //     team_id,
        //     player_id,
        //     init_teams,
        //     self.config.trail_delay(),
        //     self.config.trail_opacity(),
        //     self.config.trail_lifetime(),
        // )?);

        // self.network.spawn_receive_task(Arc::clone(&client));
        // self.network.spawn_send_task(Arc::clone(&client));

        // let current_input_write = Arc::clone(&client.current_input);
        // let (input_tx, mut input_rx) = unbounded_channel::<HashSet<KeyCode>>();
        // tokio::spawn(async move {
        //     while let Some(input) = input_rx.recv().await {
        //         let mut current = current_input_write.lock().await;
        //         *current = input;
        //     }
        // });

        // let session = GameSession { client, input_tx };

        // self.view = ClientView::InGame(session);
    }

    fn update_queue(app: &mut App, _ctx: &mut Context) -> GameResult<Option<ClientView>> {
        Ok(None)
    }

    fn update_game(
        _ctx: &mut Context,
        session: &mut GameSession,
    ) -> GameResult<Option<ClientView>> {
        if game_finished(session) {
            return Ok(Some(ClientView::Menu));
        }

        Ok(None)
    }

    fn draw_menu(_ctx: &mut Context) -> GameResult {
        dbg!("view: Main Menu");
        Ok(())
    }

    fn draw_queue(_ctx: &mut Context) -> GameResult {
        dbg!("view: Queue");
        Ok(())
    }

    fn draw_game(_ctx: &mut Context, _session: &mut GameSession) -> GameResult {
        dbg!("view: Game");
        Ok(())
    }
}

impl EventHandler for App {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        let transition = match &mut self.view {
            ClientView::Menu => App::update_menu(self, ctx)?,
            ClientView::Queue => App::update_queue(self, ctx)?,
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
            ClientView::Queue => App::draw_queue(ctx),
            ClientView::InGame(session) => App::draw_game(ctx, session),
        }
    }

    fn key_down_event(&mut self, _ctx: &mut Context, input: KeyInput, _repeat: bool) -> GameResult {
        if let Some(keycode) = input.keycode {
            match &self.view {
                ClientView::Menu => match keycode {
                    KeyCode::R => self.view = ClientView::Queue,
                    KeyCode::Q => panic!("Exiting..."), // exit hack, TODO
                    _ => {}
                },
                ClientView::Queue => match keycode {
                    KeyCode::Escape => self.view = ClientView::Menu,
                    _ => {}
                },
                _ => {}
            }
        }

        Ok(())
    }
}

fn game_finished(session: &GameSession) -> bool {
    session.client.core.blocking_lock().game_state().winner != 0
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
