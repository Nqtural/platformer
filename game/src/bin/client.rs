use ggez::{
    GameResult,
    input::keyboard::KeyCode,
};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::net::SocketAddr;
use game_config::read::Config;
use protocol::{
    constants::{
        TEAM_ONE_START_POS,
        TEAM_SIZE,
        TEAM_TWO_START_POS,
    },
    net_client::ClientMessage,
    net_game_state,
    net_server::ServerMessage,
    net_team::InitTeamData,
};
use simulation::{
    constants::{
        TICK_RATE,
        FIXED_DT,
    },
    game_state::GameState,
};
use bincode::{serde::{encode_to_vec, decode_from_slice}, config};

pub struct ClientState {
    pub team_id: usize,
    pub player_id: usize,
    pub game_state: Option<Arc<Mutex<GameState>>>,
    pub tick: Arc<AtomicU64>,
}

impl ClientState {
    pub fn apply_initial_data(
        &mut self,
        teams: Vec<InitTeamData>,
    ) -> GameResult<()> {
        let gs = net_game_state::new_from_initial(self.team_id, self.player_id, teams)?;
        self.game_state = Some(Arc::new(Mutex::new(gs)));
        Ok(())
    }
}

#[tokio::main]
async fn main() -> GameResult {
    let mut client = ClientState {
        team_id: 0,
        player_id: 0,
        game_state: None,
        tick: Arc::new(AtomicU64::new(0)),
    };

    // get configuration
    let config = Config::get()?;

    // spawn dummy team
    client.apply_initial_data(
        vec![
            InitTeamData {
                color: config.team_one_color(),
                player_names: vec![config.playername().to_string(); TEAM_SIZE],
                start_position: TEAM_ONE_START_POS,
                index: 0,
            },
            InitTeamData {
                color: config.team_two_color(),
                player_names: vec![String::from("Dummy"); TEAM_SIZE],
                start_position: TEAM_TWO_START_POS,
                index: 1,
            },
        ],
    )?;

    if !config.practice_mode() {
        let bincode_config = config::standard();

        let server_addr: SocketAddr = format!(
            "{}:{}",
            config.serverip(),
            config.serverport(),
        ).parse().unwrap();
        let socket = Arc::new(UdpSocket::bind(format!(
            "{}:{}",
            config.clientip(),
            config.clientport(),
        )).await.unwrap());
        socket.connect(server_addr).await.unwrap();

        // handshake with server
        let packet = encode_to_vec(
            ClientMessage::Hello {
                name: config.playername().to_string(),
            },
            bincode_config,
        ).map_err(|e| ggez::GameError::CustomError(e.to_string()))?;
        socket.send(&packet).await?;

        let mut buf = [0u8; 1500];
        let init_teams = loop {
            let (len, _addr) = socket.recv_from(&mut buf).await?;
            let (msg, _): (ServerMessage, usize) =
            decode_from_slice(&buf[..len], bincode_config)
                .map_err(|e| ggez::GameError::CustomError(e.to_string()))?;

            match msg {
                ServerMessage::Welcome { team_id, player_id } => {
                    client.team_id = team_id;
                    client.player_id = player_id;
                }
                ServerMessage::StartGame { teams } => {
                    break teams;
                }
                ServerMessage::LobbyStatus { .. } => {
                    // TODO: show in UI
                }
                _ => {}
            }
        };

        client.apply_initial_data(init_teams)?;

        // spawn receive task
        let socket_recv = Arc::clone(&socket);
        let config_recv = bincode_config;
        let gs_clone_send = Arc::clone(client.game_state.as_ref().unwrap());
        let gs_clone_recv = Arc::clone(client.game_state.as_ref().unwrap());

        tokio::spawn(async move {
            let mut buf = [0u8; 2048];

            loop {
                match socket_recv.recv_from(&mut buf).await {
                    Ok((len, _)) => {
                        // decode snapshot from server
                        if let Ok((ServerMessage::Snapshot{ server_tick: _, server_state }, _)) =
                        decode_from_slice::<ServerMessage, _>(&buf[..len], config_recv) {
                            // lock game state
                            let mut gs = gs_clone_recv.lock().await;

                            // preserve local inputs
                            let local_inputs: Vec<Vec<_>> = gs
                                .teams
                                .iter()
                                .map(|team| {
                                    team.players
                                        .iter()
                                        .map(|p| p.get_input().clone())
                                        .collect::<Vec<_>>()
                                })
                                .collect();

                            // apply server snapshot
                            net_game_state::apply_snapshot(&mut gs, server_state);

                            // restore local inputs to prevent input lag
                            for (team_idx, team) in gs.teams.iter_mut().enumerate() {
                                for (player_idx, player) in team.players.iter_mut().enumerate() {
                                    if let Some(input) = local_inputs
                                        .get(team_idx)
                                        .and_then(|team_inputs| team_inputs.get(player_idx))
                                    {
                                        player.set_input(input.clone());
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Receive error: {e}");
                    }
                }
            }
        });

        // simulation
        let game_state_tick = Arc::clone(client.game_state.as_ref().unwrap());
        let client_tick = Arc::clone(&client.tick);
        tokio::spawn(async move {
            let tick_duration = std::time::Duration::from_secs_f32(1.0 / TICK_RATE as f32);

            loop {
                let start = std::time::Instant::now();

                {
                    let mut gs = game_state_tick.lock().await;
                    gs.fixed_update(FIXED_DT);
                }

                client_tick.fetch_add(1, Ordering::Relaxed);

                let elapsed = start.elapsed();
                if elapsed < tick_duration {
                    tokio::time::sleep(tick_duration - elapsed).await;
                }
            }
        });

        // spawn send task
        let socket_send = Arc::clone(&socket);
        let tick_send = Arc::clone(&client.tick);
        tokio::spawn(async move {
            let tick_duration = std::time::Duration::from_millis(1000 / TICK_RATE as u64);
            loop {
                let start = std::time::Instant::now();

                let input = {
                    let gs = gs_clone_send.lock().await;
                    gs.teams[client.team_id].players[client.player_id].get_input().clone()
                };

                let tick = tick_send.load(Ordering::Relaxed);

                let msg = ClientMessage::Input {
                    client_tick: tick,
                    team_id: client.team_id,
                    player_id: client.player_id,
                    input: input.clone(),
                };
                match encode_to_vec(&msg, bincode_config) {
                    Ok(data) => {
                        let _ = socket_send.send_to(&data, server_addr).await;
                    }
                    Err(e) => eprintln!("Encoding error: {e}"),
                }

                let elapsed = start.elapsed();
                if elapsed < tick_duration {
                    tokio::time::sleep(tick_duration - elapsed).await;
                }
            }
        });
    }

    // input
    let gs_clone_input = Arc::clone(client.game_state.as_ref().unwrap());
    let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<HashSet<KeyCode>>();
    tokio::spawn(async move {
        while let Some(input) = input_rx.recv().await {
            let mut gs = gs_clone_input.lock().await;
            gs.teams[client.team_id].players[client.player_id].input.update(&input);
        }
    });

    // setup game window
    let gs_clone_window = Arc::clone(client.game_state.as_ref().unwrap());
    let _ = display::game_window::run(input_tx, gs_clone_window, "client");
    
    Ok(())
}
