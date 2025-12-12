use std::sync::Arc;
use std::net::SocketAddr;
use std::collections::HashSet;
use tokio::net::UdpSocket;
use tokio::sync::{Mutex, RwLock};
use game_config::read::Config;
use display::render::RenderState;
use protocol::{
    constants::TEAM_SIZE,
    net_client::ClientMessage,
    net_game_state,
    net_server::ServerMessage,
    lobby::Lobby,
    utils::{
        broadcast,
        send_to,
    },
};
use simulation::{
    constants::{
        ENABLE_VSYNC,
        TICK_RATE,
        VIRTUAL_HEIGHT,
        VIRTUAL_WIDTH,
    },
    game_state::GameState,
};
use bincode::{serde::{encode_to_vec, decode_from_slice}, config};
use ggez::{
    Context,
    ContextBuilder,
    event::EventHandler,
    GameResult,
};

#[tokio::main]
async fn main() -> GameResult {
    let config = Config::get()?;

    // initialize lobby state
    let lobby_state = Arc::new(RwLock::new(Lobby::new()));

    let bincode_config = config::standard();

    // store connected client addresses to broadcast state
    let clients = Arc::new(RwLock::new(HashSet::<SocketAddr>::new()));
    let clients_recv = Arc::clone(&clients);
    let clients_send = Arc::clone(&clients);

    // bind UDP socket to listen on server port
    let ip = config.serverip();
    let port = config.serverport();
    let socket = Arc::new(UdpSocket::bind(format!("{ip}:{port}")).await.unwrap());
    println!("Server listening on {ip}:{port}");

    // handshake with clients
    let mut lobby = lobby_state.write().await;
    while lobby.connected_count() != TEAM_SIZE * 2 {
        let mut buf = [0u8; 1500];
        let (len, addr) = socket.recv_from(&mut buf).await?;

        let (msg, _): (ClientMessage, usize) = decode_from_slice(
            &buf[..len],
            bincode_config,
        ).map_err(|e| ggez::GameError::CustomError(e.to_string()))?;

        if let ClientMessage::Hello { ref name } = msg {
            println!("{addr} connected as {name}");

            clients.write().await.insert(addr);

            let (team_id, player_id) = lobby.assign_slot(addr, name.clone());

            // send welcome to this client
            let welcome = ServerMessage::Welcome {
                team_id,
                player_id,
            };
            send_to(addr, welcome, &socket, &bincode_config).await;

            // send lobby status to everyone
            let status = ServerMessage::LobbyStatus {
                players: lobby.players.clone(),
                required: TEAM_SIZE * 2,
            };
            broadcast(status, &clients, &socket, &bincode_config).await;
        }
    }
    println!("Starting game...");

    // setup the ggez window and run event loop
    let (ctx, event_loop) = ContextBuilder::new("server", "platform")
        .window_setup(
            ggez::conf::WindowSetup::default()
                .vsync(ENABLE_VSYNC)
                .title("Server")
        )
        .window_mode(
            ggez::conf::WindowMode::default()
                .dimensions(VIRTUAL_WIDTH, VIRTUAL_HEIGHT)
                .resizable(true)
                .visible(config.render_server())
        )
        .build()?;

    // generate InitTeamData from lobby
    let init_teams = lobby.initial_teams(
        config.team_one_color(),
        config.team_two_color()
    );

    // create actual GameState from InitTeamData
    let gs = net_game_state::new_from_initial(0, 0, init_teams.clone())?;

    // store game state inside Arc<Mutex<_>>
    let game_state = Arc::new(Mutex::new(gs));

    // broadcast to clients
    broadcast(
        ServerMessage::StartGame {
            teams: init_teams
        },
        &clients,
        &socket,
        &bincode_config
    ).await;

    // task to receive client messages, update GameState, and track clients
    let game_state_recv = Arc::clone(&game_state);
    let game_state_send = Arc::clone(&game_state);
    let socket_recv = Arc::clone(&socket);
    let config_recv = bincode_config;
    tokio::spawn(async move {
        let mut buf = [0u8; 2048];
    loop {
            match socket_recv.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    // remember the client to send snapshots
                    {
                        let mut c = clients_recv.write().await;
                        c.insert(addr);
                    }

                    // decode incoming ClientMessage
                    if let Ok((msg, _)) = decode_from_slice::<ClientMessage, _>(&buf[..len], config_recv) {
                        match msg {
                            ClientMessage::Input { tick: _, team_id, player_id, input } => {
                                let mut gs = game_state_recv.lock().await;
                                // update player input in game state
                                if let Some(team) = gs.teams.get_mut(team_id)
                                    && let Some(player) = team.players.get_mut(player_id) {
                                        player.set_input(input.clone());
                                    }
                            },
                            ClientMessage::Hello { .. } => {}
                        }
                    }
                }
                Err(e) => eprintln!("Receive error: {e}"),
            }
        }
    });

    // simulation loop
    let game_state_tick = Arc::clone(&game_state);
    tokio::spawn(async move {
        let tick_duration = std::time::Duration::from_millis(1000 / TICK_RATE as u64);
        let mut last = std::time::Instant::now();

        loop {
            let now = std::time::Instant::now();
            let dt = (now - last).as_secs_f32();
            last = now;

            {
                let mut gs = game_state_tick.lock().await;
                gs.fixed_update(dt);
            }

            tokio::time::sleep(tick_duration).await;
        }
    });

    // task to periodically send ServerMessage snapshots to all connected clients
    let socket_send = Arc::clone(&socket);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(8)).await;

            let gs = game_state_send.lock().await;
            let snapshot_msg = ServerMessage::Snapshot(net_game_state::to_net(&gs));
            drop(gs);

            let data = match encode_to_vec(&snapshot_msg, bincode_config) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Encoding error: {e}");
                    continue;
                }
            };

            let client_addrs = {
                let c = clients_send.read().await;
                c.clone()
            };

            // broadcast snapshot to all clients
            for client in &client_addrs {
                let _ = socket_send.send_to(&data, client).await;
            }
        }
    });

    let render_state = RenderState::new(&ctx);
    ggez::event::run(
        ctx,
        event_loop,
        SharedGameState {
            game_state,
            render_state,
        }
    )
}

struct SharedGameState{
    game_state: Arc<Mutex<GameState>>,
    render_state: RenderState,
}

impl EventHandler for SharedGameState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        // this must stay empty in order
        // to sepparate logic from rendering
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        if let Ok(gs) = self.game_state.try_lock() {
            self.render_state.render(ctx, &gs)
        } else {
            Ok(())
        }
    }
}
