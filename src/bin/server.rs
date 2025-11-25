use rand;
use std::sync::Arc;
use std::net::SocketAddr;
use std::collections::HashSet;
use tokio::net::UdpSocket;
use tokio::sync::{Mutex, RwLock};
use platform::{
    constants::{
        ENABLE_VSYNC,
        REQUIRED_PLAYERS,
        TEAM_ONE_COLOR,
        TEAM_ONE_START_POS,
        TEAM_TWO_COLOR,
        TEAM_TWO_START_POS,
        VIRTUAL_HEIGHT,
        VIRTUAL_WIDTH,
    },
    game_state::GameState,
    lobby::Lobby,
    network::{
        ClientMessage,
        ServerMessage,
    },
    player::Player,
    read_config::Config,
    team::Team,
    utils::{
        broadcast,
        send_to,
    },
};
use bincode::{serde::{encode_to_vec, decode_from_slice}, config};
use ggez::{
    ContextBuilder,
    GameResult,
};

#[tokio::main]
async fn main() -> GameResult {
    let config = Config::get()?;
    // Setup the ggez window and run event loop (optional server GUI or visualizer)
    let (mut ctx, event_loop) = ContextBuilder::new("server", "platform")
        .window_setup(
            ggez::conf::WindowSetup::default()
                .vsync(ENABLE_VSYNC)
                .title("Server")
        )
        .window_mode(
            ggez::conf::WindowMode::default()
                .dimensions(VIRTUAL_WIDTH, VIRTUAL_HEIGHT)
                .resizable(true)
        )
        .build()?;

    // Initialize shared GameState
    let game_state = Arc::new(Mutex::new(GameState::new(
        [
            Team::new(
                vec![Player::new(TEAM_ONE_START_POS, "Player1".into())],
                TEAM_ONE_COLOR,
            ),
            Team::new(
                vec![Player::new(TEAM_TWO_START_POS, "Player2".into())],
                TEAM_TWO_COLOR,
            ),
        ],
        &mut ctx
    )?));

    // Initialize lobby state
    let lobby_state = Arc::new(RwLock::new(Lobby::new()));

    let lobby_state_recv = Arc::clone(&lobby_state);
    let lobby_state_send = Arc::clone(&lobby_state);

    let bincode_config = config::standard();
    let game_state_recv = Arc::clone(&game_state);
    let game_state_send = Arc::clone(&game_state);

    // Store connected client addresses to broadcast state
    let clients = Arc::new(RwLock::new(HashSet::<SocketAddr>::new()));
    let clients_recv = Arc::clone(&clients);
    let clients_send = Arc::clone(&clients);

    // Bind UDP socket to listen on server port
    let ip = config.serverip();
    let port = config.serverport();
    let socket = Arc::new(UdpSocket::bind(format!("{}:{}", ip, port)).await.unwrap());
    println!("Server listening on {}:{}", ip, port);

    // Handshake with clients
    let mut buf = [0u8; 1500];
    let (len, addr) = socket.recv_from(&mut buf).await?;

    let (msg, _): (ClientMessage, usize) = decode_from_slice(
        &buf[..len],
        bincode_config,
    ).map_err(|e| ggez::GameError::CustomError(e.to_string()))?;

    if let ClientMessage::Hello { name } = msg {
        let mut lobby = lobby_state.write().await;

        // Prevent duplicate connections
        //if lobby.is_name_taken(&name) {
        //    // Modify name or reject
        //    name = format!("{}{}", name, rand::random::<u16>());
        //}

        let (team_id, player_id) = lobby.assign_slot(addr, name.clone());

        // Send welcome to this client
        let welcome = ServerMessage::Welcome {
            team_id,
            player_id,
            name: name.clone(),
        };
        send_to(addr, welcome, &socket, &bincode_config).await;

        // Send lobby status to everyone
        let status = ServerMessage::LobbyStatus {
            players: lobby.players.clone(),
            required: REQUIRED_PLAYERS,
        };
        broadcast(status.clone(), &clients, &socket, &bincode_config).await;

        if lobby.connected_count() == REQUIRED_PLAYERS {
            let start_msg = ServerMessage::StartGame {
                teams: lobby.initial_teams(),
            };
            broadcast(status, &clients, &socket, &bincode_config).await;
        }
    }

    // Task to receive client messages, update GameState, and track clients
    let socket_recv = Arc::clone(&socket);
    let config_recv = bincode_config.clone();
    tokio::spawn(async move {
        let mut buf = [0u8; 2048];
        loop {
            match socket_recv.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    // Remember the client to send snapshots
                    {
                        let mut c = clients_recv.write().await;
                        c.insert(addr);
                    }

                    // Decode incoming ClientMessage
                    if let Ok((msg, _)) = decode_from_slice::<ClientMessage, _>(&buf[..len], config_recv) {
                        match msg {
                            ClientMessage::Input { team_id, player_id, input } => {
                                let mut gs = game_state_recv.lock().await;
                                // Update player input in game state (assuming valid indexing)
                                if let Some(team) = gs.teams.get_mut(team_id) {
                                    if let Some(player) = team.players.get_mut(player_id) {
                                        player.input = input;
                                    }
                                }
                            },
                            ClientMessage::Hello { .. } => {}
                        }
                    }
                }
                Err(e) => eprintln!("Receive error: {}", e),
            }
        }
    });

    // Task to periodically send ServerMessage snapshots to all connected clients
    let socket_send = Arc::clone(&socket);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(16)).await;

            let gs = game_state_send.lock().await;
            let snapshot_msg = ServerMessage::Snapshot(gs.to_net());
            drop(gs);

            let data = match encode_to_vec(&snapshot_msg, bincode_config) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Encoding error: {}", e);
                    continue;
                }
            };

            let client_addrs = {
                let c = clients_send.read().await;
                c.clone()
            };

            // Broadcast snapshot to all clients
            for client in client_addrs.iter() {
                let _ = socket_send.send_to(&data, client).await;
            }
        }
    });

    ggez::event::run(ctx, event_loop, SharedGameState(game_state))
}

struct SharedGameState(Arc<Mutex<GameState>>);

impl ggez::event::EventHandler for SharedGameState {
    fn update(&mut self, ctx: &mut ggez::Context) -> GameResult {
        let gs = self.0.try_lock();
        if let Ok(mut gs) = gs {
            gs.update(ctx)
        } else {
            Ok(())
        }
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> GameResult {
        let gs = self.0.try_lock();
        if let Ok(mut gs) = gs {
            gs.draw(ctx)
        } else {
            Ok(())
        }
    }
}
