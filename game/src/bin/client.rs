use anyhow::Result;
use ggez::input::keyboard::KeyCode;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::net::SocketAddr;
use client_logic::{
    interpolation::SnapshotHistory,
    render_clock::RenderClock,
};
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

const INPUT_HISTORY_SIZE: usize = 512;

#[derive(Clone)]
pub struct TimedInput {
    pub tick: u64,
    pub input: HashSet<KeyCode>,
}

pub struct InputHistory {
    buffer: [Option<TimedInput>; INPUT_HISTORY_SIZE],
}

impl InputHistory {
    fn new() -> Self {
        Self {
            buffer: std::array::from_fn(|_| None),
        }
    }

    fn push(&mut self, tick: u64, input: HashSet<KeyCode>) {
        let index = (tick as usize) % INPUT_HISTORY_SIZE;
        self.buffer[index] = Some(TimedInput { tick, input });
    }

    fn _get(&self, tick: u64) -> Option<&HashSet<KeyCode>> {
        let index = (tick as usize) % INPUT_HISTORY_SIZE;
        self.buffer[index]
            .as_ref()
            .filter(|entry| entry.tick == tick)
            .map(|entry| &entry.input)
    }
}

struct ClientPrediction {
    _tick: AtomicU64,
    input_history: InputHistory,
}

pub struct ClientState {
    pub team_id: usize,
    pub player_id: usize,
    pub current_input: Arc<Mutex<HashSet<KeyCode>>>,
    pub snapshot_history: Arc<Mutex<SnapshotHistory>>,
    render_clock: RenderClock,
    render_tick: Arc<Mutex<f32>>,
    pub game_state: Option<Arc<Mutex<GameState>>>,
    pub tick: Arc<AtomicU64>,
}

impl ClientState {
    pub fn apply_initial_data(
        &mut self,
        teams: Vec<InitTeamData>,
    ) -> Result<()> {
        let gs = net_game_state::new_from_initial(self.team_id, self.player_id, teams)?;
        self.game_state = Some(Arc::new(Mutex::new(gs)));
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = ClientState {
        team_id: 0,
        player_id: 0,
        current_input: Arc::new(Mutex::new(HashSet::new())),
        snapshot_history: Arc::new(Mutex::new(SnapshotHistory::default())),
        render_clock: RenderClock::default(),
        render_tick: Arc::new(Mutex::new(0.0)),
        game_state: None,
        tick: Arc::new(AtomicU64::new(0)),
    };

    let prediction = Arc::new(Mutex::new(ClientPrediction {
        _tick: AtomicU64::new(0),
        input_history: InputHistory::new(),
    }));

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
        )?;
        socket.send(&packet).await?;

        let mut buf = [0u8; 1500];
        let init_teams = loop {
            let (len, _addr) = socket.recv_from(&mut buf).await?;
            let (msg, _): (ServerMessage, usize) =
            decode_from_slice(&buf[..len], bincode_config)?;

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
        let history_clone_recv = Arc::clone(&client.snapshot_history);
        let render_tick_update = Arc::clone(&client.render_tick);
        tokio::spawn(async move {
            let mut buf = [0u8; 2048];

            loop {
                match socket_recv.recv_from(&mut buf).await {
                    Ok((len, _)) => {
                        // decode snapshot from server
                        if let Ok((ServerMessage::Snapshot{ server_tick, server_state }, _)) =
                        decode_from_slice::<ServerMessage, _>(&buf[..len], config_recv) {
                            let mut snapshot_history = history_clone_recv.lock().await;
                            {
                                // update render clock
                                client.render_clock.update(server_tick);
                                let mut render_tick = render_tick_update.lock().await;
                                *render_tick = client.render_clock.render_tick();
                                drop(render_tick);

                                // apply server snapshot
                                let mut gs = gs_clone_recv.lock().await;
                                net_game_state::apply_snapshot(&mut gs, &server_state);
                                snapshot_history.push(server_tick, gs.clone());
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Receive error: {e}");
                    }
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

    // simulation
    let game_state_tick = Arc::clone(client.game_state.as_ref().unwrap());
    let client_tick = Arc::clone(&client.tick);
    let prediction_clone = Arc::clone(&prediction);
    let current_input_read = Arc::clone(&client.current_input);
    tokio::spawn(async move {
        let tick_duration = std::time::Duration::from_secs_f32(1.0 / TICK_RATE as f32);

        loop {
            let start = std::time::Instant::now();
            let tick = client_tick.load(Ordering::Relaxed);

            let current_input = current_input_read.lock().await.clone();

            {
                let mut prediction = prediction_clone.lock().await;

                {
                    let mut gs = game_state_tick.lock().await;

                    // apply current input to GameState
                    gs.teams[client.team_id].players[client.player_id].input.update(&current_input);

                    // simulate
                    gs.fixed_update(FIXED_DT);
                }

                // store input for current tick
                prediction.input_history.push(
                    tick,
                    current_input,
                );
            }

            client_tick.fetch_add(1, Ordering::Relaxed);

            let elapsed = start.elapsed();
            if elapsed < tick_duration {
                tokio::time::sleep(tick_duration - elapsed).await;
            }
        }
    });

    // input
    let current_input_write = Arc::clone(&client.current_input);
    let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<HashSet<KeyCode>>();
    tokio::spawn(async move {
        while let Some(input) = input_rx.recv().await {
            let mut current = current_input_write.lock().await;
            *current = input;
        }
    });

    // setup game window
    let history_clone_render = Arc::clone(&client.snapshot_history);
    let render_tick_clone = Arc::clone(&client.render_tick);
    let _ = display::game_window::run(input_tx, history_clone_render, render_tick_clone, "client");
    
    Ok(())
}
