use ggez::{
    Context,
    ContextBuilder,
    GameError,
    GameResult,
};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::net::SocketAddr;
use platform::{
    constants::{
        C_TEAM,
        C_PLAYER,
        ENABLE_VSYNC,
        TEAM_ONE_START_POS,
        TEAM_SIZE,
        TEAM_TWO_START_POS,
        VIRTUAL_HEIGHT,
        VIRTUAL_WIDTH,
    },
    game_state::GameState,
    network::{
        ClientMessage,
        ServerMessage,
        InitTeamData,
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

#[tokio::main]
async fn main() -> GameResult {
    let mut client = ClientState {
        team_id: 0,
        player_id: 0,
        ready: false,
        game_state: None,
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

    let game_state = Arc::new(Mutex::new(GameState::new(
        [
            Team::new(
                (0..TEAM_SIZE)
                    .map(|_| Player::new(TEAM_ONE_START_POS, "Player".into(), config.team_one_color()))
                    .collect()
            ),
            Team::new(
                (0..TEAM_SIZE)
                    .map(|_| Player::new(TEAM_TWO_START_POS, "Player".into(), config.team_two_color()))
                    .collect()
            ),
        ],
        &mut ctx
    )?));

    let bincode_config = config::standard();
    let gs_clone_send = Arc::clone(&game_state);
    let gs_clone_recv = Arc::clone(&game_state);

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

    if let ServerMessage::StartGame { teams } = msg {
        client.apply_initial_data(teams, &mut ctx)?;
        client.ready = true;
    }

    // spawn receive task
    let socket_recv = Arc::clone(&socket);
    let config_recv = bincode_config;

    tokio::spawn(async move {
        let mut buf = [0u8; 2048];

        loop {
            match socket_recv.recv_from(&mut buf).await {
                Ok((len, _)) => {
                    // decode snapshot from server
                    if let Ok((ServerMessage::Snapshot(server_state), _)) =
                    decode_from_slice::<ServerMessage, _>(&buf[..len], config_recv)
                    {
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

                        // apply server snapshot
                        gs.apply_snapshot(server_state);

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
                    eprintln!("Receive error: {e}");
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
                Err(e) => eprintln!("Encoding error: {e}"),
            }

            tokio::time::sleep(std::time::Duration::from_millis(16)).await;
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

    fn key_down_event(
        &mut self,
        ctx: &mut ggez::Context,
        input: ggez::input::keyboard::KeyInput,
        repeated: bool,
    ) -> GameResult {
        if let Ok(mut gs) = self.0.try_lock() {
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
        if let Ok(mut gs) = self.0.try_lock() {
            gs.key_up_event(ctx, input)
        } else {
            Ok(())
        }
    }
}
