use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::net::SocketAddr;
use std::collections::HashSet;
use tokio::net::UdpSocket;
use tokio::sync::{Mutex, RwLock};
use game_config::read::Config;
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
        TICK_RATE,
        FIXED_DT,
    },
    game_state::GameState,
};
use bincode::{serde::{encode_to_vec, decode_from_slice}, config};
use ggez::{
    GameResult,
    input::keyboard::KeyCode,
};

struct ServerState {
    pub game_state: Option<Arc<Mutex<GameState>>>,
    pub tick: Arc<AtomicU64>,
}

#[tokio::main]
async fn main() -> GameResult {
    let mut server = ServerState {
        game_state: None,
        tick: Arc::new(AtomicU64::new(0)),
    };

    let config = Config::get()?;

    println!("Initializing lobby state (team size: {TEAM_SIZE})...");
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
    println!("Waiting for players...");

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

            println!(
                "({}/{}): {} connected as {}",
                lobby.connected_count(),
                TEAM_SIZE * 2,
                addr,
                name,
            );
        }
    }

    println!("Starting game...");

    // generate InitTeamData from lobby
    let init_teams = lobby.initial_teams(
        config.team_one_color(),
        config.team_two_color()
    );

    // create actual GameState from InitTeamData
    server.game_state = Some(Arc::new(Mutex::new(net_game_state::new_from_initial(0, 0, init_teams.clone())?)));

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
    let game_state_recv = Arc::clone(server.game_state.as_ref().unwrap());
    let game_state_send = Arc::clone(server.game_state.as_ref().unwrap());
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
                            ClientMessage::Input { client_tick, team_id, player_id, input } => {
                                println!("tick: {}", client_tick);
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
    let game_state_tick = Arc::clone(server.game_state.as_ref().unwrap());
    let tick = Arc::clone(&server.tick);
    tokio::spawn(async move {
        let tick_duration = std::time::Duration::from_millis(1000 / TICK_RATE as u64);
        loop {
            let start = std::time::Instant::now();

            {
                let mut gs = game_state_tick.lock().await;
                gs.fixed_update(FIXED_DT);
            }

            tick.fetch_add(1, Ordering::Relaxed);

            let elapsed = start.elapsed();
            if elapsed < tick_duration {
                tokio::time::sleep(tick_duration - elapsed).await;
            }
        }
    });

    // task to periodically send ServerMessage snapshots to all connected clients
    let socket_send = Arc::clone(&socket);
    let tick_send = Arc::clone(&server.tick);
    tokio::spawn(async move {
        let interval = std::time::Duration::from_secs_f32(FIXED_DT);

        loop {
            tokio::time::sleep(interval).await;

            let tick = tick_send.load(Ordering::Relaxed);

            let gs = game_state_send.lock().await;
            let snapshot = net_game_state::to_net(&gs);
            drop(gs);

            let msg = ServerMessage::Snapshot {
                server_tick: tick,
                server_state: snapshot,
            };

            let data = match encode_to_vec(&msg, bincode_config) {
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

    println!("Game started");

    if config.render_server() {
        // needed parameter for client input, unused here
        let (input_tx, _) = tokio::sync::mpsc::unbounded_channel::<HashSet<KeyCode>>();

        // setup game window
        let gs_clone_window = Arc::clone(server.game_state.as_ref().unwrap());
        let _ = display::game_window::run(input_tx, gs_clone_window, "server");
    }

    tokio::signal::ctrl_c().await
        .map_err(|e| ggez::GameError::CustomError(e.to_string()))?;

    println!("\nStopping server...");

    Ok(())
}
