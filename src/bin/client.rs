use ggez::{
    Context,
    ContextBuilder,
    GameError,
    GameResult,
};
use tokio::net::UdpSocket;
use tokio::sync::Mutex as TokioMutex;
use std::collections::VecDeque;
use std::sync::Arc;
use std::net::SocketAddr;
use std::time::Instant;
use platform::{
    constants::{
        C_TEAM,
        C_PLAYER,
        ENABLE_VSYNC,
        TEAM_ONE_START_POS,
        TEAM_TWO_START_POS,
        VIRTUAL_HEIGHT,
        VIRTUAL_WIDTH,
    },
    game_state::GameState,
    network::{
        ClientMessage,
        ServerMessage,
        InitTeamData,
        NetPlayer,
        NetSnapshot,
    },
    read_config::Config,
    player::Player,
    team::Team,
};
use bincode::{serde::{encode_to_vec, decode_from_slice}, config};

pub struct ClientState {
    pub team_id: usize,
    pub player_id: usize,
    pub ready: bool,
    pub game_state: Option<GameState>,
    pub buffer: SnapshotBuffer,
}

impl ClientState {
    pub fn apply_initial_data(
        &mut self,
        teams: Vec<InitTeamData>,
        ctx: &mut Context,
    ) -> GameResult<()> {
        let team_list: Vec<Team> = teams
            .into_iter()
            .map(Team::from_init)
            .collect();

        let teams_array: [Team; 2] = team_list.try_into()
            .map_err(|_| GameError::ResourceLoadError("Exactly 2 teams required".to_string()))?;

        let gs = GameState::new(teams_array, ctx)?;
        self.game_state = Some(gs);
        Ok(())
    }
}

pub struct SnapshotEntry {
    pub tick: u64,
    pub timestamp: f64,
    pub state: NetSnapshot,
}

pub struct SnapshotBuffer {
    pub delay: f64,
    pub snapshots: VecDeque<SnapshotEntry>,
}

impl SnapshotBuffer {
    pub fn new(delay: f64) -> Self {
        Self {
            delay,
            snapshots: VecDeque::with_capacity(256),
        }
    }

    pub fn push_snapshot(&mut self, tick: u64, state: NetSnapshot, now: f64) {
        self.snapshots.push_back(SnapshotEntry {
            tick,
            timestamp: now,
            state,
        });

        while self.snapshots.len() > 256 {
            self.snapshots.pop_front();
        }
    }

    // finds the two snapshots around a target time
    pub fn interpolate(&self, now: f64) -> Option<NetSnapshot> {
        let target = now - self.delay;

        // need at least 2 snapshots to interpolate
        if self.snapshots.len() < 2 {
            return None;
        }

        // find older/newer entries
        let mut older = &self.snapshots[0];
        let mut newer = &self.snapshots[1];

        for i in 1..self.snapshots.len() {
            if self.snapshots[i].timestamp >= target {
                older = &self.snapshots[i - 1];
                newer = &self.snapshots[i];
                break;
            }
        }

        let dt = newer.timestamp - older.timestamp;
        if dt <= 0.0 {
            return Some(older.state.clone());
        }

        let t = ((target - older.timestamp) / dt).clamp(0.0, 1.0) as f32;

        Some(Self::lerp_snapshot(&older.state, &newer.state, t))
    }

    fn lerp_snapshot(a: &NetSnapshot, b: &NetSnapshot, t: f32) -> NetSnapshot {
        NetSnapshot {
            winner: b.winner,
            players: a.players.iter().zip(&b.players).map(|(pa, pb)| {
                NetPlayer {
                    team_id: pa.team_id,
                    player_id: pa.player_id,
                    pos: [
                        pa.pos[0] + (pb.pos[0] - pa.pos[0]) * t,
                        pa.pos[1] + (pb.pos[1] - pa.pos[1]) * t,
                    ],
                    vel: [
                        pa.vel[0] + (pb.vel[0] - pa.vel[0]) * t,
                        pa.vel[1] + (pb.vel[1] - pa.vel[1]) * t,
                    ],
                    stunned: pb.stunned,
                    invulnerable: pb.invulnerable,
                    lives: pb.lives,
                }
            }).collect(),
            attacks: b.attacks.clone(),
        }
    }
}

#[tokio::main]
async fn main() -> GameResult {
    // start time shared between tasks and ggez loop
    let start = Arc::new(Instant::now());

    // ClientState is used only for handshake/initialization here
    let mut client = ClientState {
        team_id: 0,
        player_id: 0,
        ready: false,
        game_state: None,
        buffer: SnapshotBuffer::new(0.1),
    };

    let config = Config::get()?;

    // setup game window
    let (mut ctx, event_loop) = ContextBuilder::new("client", "platform")
        .window_setup(
            ggez::conf::WindowSetup::default()
                .vsync(ENABLE_VSYNC)
                .title("Game")
        )
        .window_mode(
            ggez::conf::WindowMode::default()
                .dimensions(VIRTUAL_WIDTH, VIRTUAL_HEIGHT)
                .resizable(true)
        )
        .build()?;

    // shared GameState (wrapped in a tokio Mutex so tasks can use .lock().await)
    let game_state = Arc::new(TokioMutex::new(GameState::new(
        [
            Team::new(
                vec![Player::new(TEAM_ONE_START_POS, "Player1".into())],
                config.team_one_color(),
            ),
            Team::new(
                vec![Player::new(TEAM_TWO_START_POS, "Player2".into())],
                config.team_two_color(),
            ),
        ],
        &mut ctx
    )?));

    // shared SnapshotBuffer (tokio Mutex so receive task can push)
    let shared_buffer = Arc::new(TokioMutex::new(SnapshotBuffer::new(0.1)));

    let bincode_config = config::standard();
    let gs_clone_send = Arc::clone(&game_state);
    let buffer_clone_for_task = Arc::clone(&shared_buffer);
    let start_clone_for_task = Arc::clone(&start);

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
    let (len, _addr) = socket.recv_from(&mut buf).await?;

    let (msg, _): (ServerMessage, usize) = decode_from_slice(
        &buf[..len],
        bincode_config,
    ).map_err(|e| ggez::GameError::CustomError(e.to_string()))?;
    if let ServerMessage::Welcome { team_id, player_id } = msg {
        client.team_id = team_id;
        client.player_id = player_id;
    }

    if let ServerMessage::StartGame { ref teams } = msg {
        client.apply_initial_data(teams.to_vec(), &mut ctx)?;
        client.ready = true;
    }

    // spawn receive task
    let socket_recv = Arc::clone(&socket);
    tokio::spawn(async move {
        let mut buf = [0u8; 2048];

        loop {
            match socket_recv.recv_from(&mut buf).await {
                Ok((len, _)) => {
                    // decode snapshot from server
                    if let Ok((ServerMessage::Snapshot{ tick, state }, _)) =
                    decode_from_slice::<ServerMessage, _>(&buf[..len], config_recv)
                    {
                        // timestamp using the shared Instant
                        let now = start_clone_for_task.elapsed().as_secs_f64();

                        // push into shared buffer
                        let mut buffer = buffer_clone_for_task.lock().await;
                        buffer.push_snapshot(tick, state, now);

                        // lock game state
                        let mut gs = gs_clone_recv.lock().await;

                        // preserve local inputs
                        let local_inputs: Vec<Vec<_>> = gs
                            .teams
                            .iter()
                            .map(|team| {
                                team.players
                                    .iter()
                                    .map(|p| p.input.clone())
                                    .collect::<Vec<_>>()
                            })
                            .collect();

                        // restore local inputs to prevent input lag
                        for (team_idx, team) in gs.teams.iter_mut().enumerate() {
                            for (player_idx, player) in team.players.iter_mut().enumerate() {
                                if let Some(input) = local_inputs
                                    .get(team_idx)
                                    .and_then(|team_inputs| team_inputs.get(player_idx))
                                {
                                    player.input = input.clone();
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Receive error: {}", e);
                }
            }
        }
    });

    // spawn send task
    let socket_send = Arc::clone(&socket);
    tokio::spawn(async move {
        loop {
            let input = {
                let gs = gs_clone_send.lock().await;
                gs.teams[C_TEAM].players[C_PLAYER].input.clone()
            };

            let msg = ClientMessage::Input {
                team_id: C_TEAM,
                player_id: C_PLAYER,
                input: input.clone(),
            };
            match encode_to_vec(&msg, bincode_config) {
                Ok(data) => {
                    let _ = socket_send.send_to(&data, server_addr).await;
                }
                Err(e) => eprintln!("Encoding error: {}", e),
            }

            tokio::time::sleep(std::time::Duration::from_millis(16)).await;
        }
    });

    // Run ggez event loop with a handler that has access to the shared buffer and start Instant
    ggez::event::run(ctx, event_loop, SharedGameState {
        game_state,
        buffer: shared_buffer,
        start,
    })
}

struct SharedGameState {
    game_state: Arc<TokioMutex<GameState>>,
    buffer: Arc<TokioMutex<SnapshotBuffer>>,
    start: Arc<Instant>,
}

impl ggez::event::EventHandler for SharedGameState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // compute shared time relative to start
        let now = self.start.elapsed().as_secs_f64();

        // try to get an interpolated snapshot (non-blocking)
        if let Ok(buffer_guard) = self.buffer.try_lock()
            && let Some(snapshot) = buffer_guard.interpolate(now) {
            // apply interpolated snapshot to the game state (non-blocking)
            if let Ok(mut gs) = self.game_state.try_lock() {
                gs.apply_interpolated_snapshot(snapshot);
            }
        }

        // now run the usual update (non-blocking)
        if let Ok(mut gs) = self.game_state.try_lock() {
            gs.update(ctx)?;
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> GameResult {
        if let Ok(mut gs) = self.game_state.try_lock() {
            gs.draw(ctx)
        } else {
            Ok(())
        }
    }

    fn key_down_event(
        &mut self,
        ctx: &mut ggez::Context,
        input: ggez::input::keyboard::KeyInput,
        repeated: bool,
    ) -> GameResult {
        if let Ok(mut gs) = self.game_state.try_lock() {
            gs.key_down_event(ctx, input, repeated)
        } else {
            Ok(())
        }
    }

    fn key_up_event(
        &mut self,
        ctx: &mut ggez::Context,
        input: ggez::input::keyboard::KeyInput,
    ) -> GameResult {
        if let Ok(mut gs) = self.game_state.try_lock() {
            gs.key_up_event(ctx, input)
        } else {
            Ok(())
        }
    }
}
