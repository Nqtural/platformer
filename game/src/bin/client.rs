use anyhow::{Error, Result};
use client_logic::{ClientState, GameSession, NetworkClient};
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
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

enum ClientView {
    Menu,
    Queue(QueueController),
    InGame(Box<GameSession>),
}

struct QueueController {
    event_rx: UnboundedReceiver<QueueEvent>,
}

struct InitialGameData {
    c_team_id: usize,
    c_player_id: usize,
    player_names: [Vec<String>; 2],
}

enum QueueEvent {
    MatchFound(InitialGameData),
    Error(Error),
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
        tokio::spawn({
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
        // tokio::spawn(async move {
        //     loop {
        //         let _ = match network.poll_queue().await {
        //             Ok(server_message) => {
        //                 match server_message {
        //                     ServerMessage::StartGame {
        //                         c_team_id,
        //                         c_player_id,
        //                         player_names,
        //                     } => {
        //                         let client = Arc::new(
        //                             match ClientState::new(
        //                                 c_team_id,
        //                                 c_player_id,
        //                                 player_names,
        //                                 config.trail_delay(),
        //                                 config.trail_opacity(),
        //                                 config.trail_lifetime(),
        //                             ) {
        //                                 Ok(client) => client,
        //                                 Err(e) => {
        //                                     println!("Error initializing client: {e}");
        //                                     return;
        //                                 }
        //                             },
        //                         );

        //                         // spawn networking tasks.
        //                         network.spawn_receive_task(Arc::clone(&client));
        //                         network.spawn_send_task(Arc::clone(&client));

        //                         // forward keyboard input into the shared client input state.
        //                         let current_input_write = Arc::clone(&client.current_input);

        //                         let (input_tx, mut input_rx) =
        //                             unbounded_channel::<HashSet<KeyCode>>();

        //                         tokio::spawn(async move {
        //                             while let Some(input) = input_rx.recv().await {
        //                                 let mut current = current_input_write.lock().await;
        //                                 *current = input;
        //                             }
        //                         });

        //                         let render_state = match RenderState::new(ctx, &config) {
        //                             Ok(render_state) => render_state,
        //                             Err(e) => {
        //                                 eprintln!("Error initializing render_state: {e}");
        //                                 return;
        //                             }
        //                         };

        //                         // Ok(GameSession::new(
        //                         //     input_tx,
        //                         //     Arc::clone(&client.snapshot_history),
        //                         //     Arc::clone(&client.render_tick),
        //                         //     render_state,
        //                         // ));

        //                         event_tx.send(QueueEvent::MatchFound(Box::new(GameSession::new(
        //                             input_tx,
        //                             Arc::clone(&client.snapshot_history),
        //                             Arc::clone(&client.render_tick),
        //                             render_state,
        //                         ))));
        //                         break;
        //                     }
        //                     _ => {}
        //                 }
        //             }
        //             Err(e) => {}
        //         };
        //     }
        // });

        Ok(QueueController { event_rx })
    }

    // async fn queue_and_connect(
    //     render_state: RenderState,
    //     network: NetworkClient,
    //     config: &Config,
    // ) -> Result<GameSession> {
    //     let client = Arc::new(ClientState::new(
    //         team_id,
    //         player_id,
    //         init_teams,
    //         config.trail_delay(),
    //         config.trail_opacity(),
    //         config.trail_lifetime(),
    //     )?);

    //     // spawn networking tasks.
    //     network.spawn_receive_task(Arc::clone(&client));
    //     network.spawn_send_task(Arc::clone(&client));

    //     // forward keyboard input into the shared client input state.
    //     let current_input_write = Arc::clone(&client.current_input);

    //     let (input_tx, mut input_rx) = unbounded_channel::<HashSet<KeyCode>>();

    //     tokio::spawn(async move {
    //         while let Some(input) = input_rx.recv().await {
    //             let mut current = current_input_write.lock().await;
    //             *current = input;
    //         }
    //     });

    //     Ok(GameSession::new(
    //         input_tx,
    //         Arc::clone(&client.snapshot_history),
    //         Arc::clone(&client.render_tick),
    //         render_state,
    //     ))
    // }

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

                    let session = GameSession::new(
                        input_tx,
                        Arc::clone(&client.snapshot_history),
                        Arc::clone(&client.render_tick),
                        render_state,
                    );

                    return Ok(Some(ClientView::InGame(Box::new(session))));
                }
                QueueEvent::Error(_) => {
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
                    KeyCode::Q => panic!("Exiting..."), // exit hack, TODO
                    _ => {}
                },
                ClientView::Queue(_) => match keycode {
                    KeyCode::Escape => {
                        let network = Arc::clone(&self.network);
                        tokio::spawn(async move {
                            let _ = network.leave_queue().await;
                        });
                        self.view = ClientView::Menu;
                    }
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
