use ggez::{
    Context,
    ContextBuilder,
    event::EventHandler,
    GameResult,
    input::keyboard::{KeyCode, KeyInput},
};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::collections::HashSet;
use std::net::SocketAddr;
use platform::{
    constants::{
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
};
use bincode::{serde::{encode_to_vec, decode_from_slice}, config};

pub struct ClientState {
    pub team_id: usize,
    pub player_id: usize,
    pub game_state: Option<Arc<Mutex<GameState>>>,
}

impl ClientState {
    pub fn apply_initial_data(
        &mut self,
        teams: Vec<InitTeamData>,
        ctx: &mut Context,
    ) -> GameResult<()> {
        let gs = GameState::new_from_initial(self.team_id, self.player_id, teams, ctx)?;
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

        client.apply_initial_data(init_teams, &mut ctx)?;

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
                        if let Ok((ServerMessage::Snapshot(server_state), _)) =
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
                            gs.apply_snapshot(server_state);

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

        // spawn send task
        let socket_send = Arc::clone(&socket);
        tokio::spawn(async move {
            loop {
                let input = {
                    let gs = gs_clone_send.lock().await;
                    gs.teams[client.team_id].players[client.player_id].get_input().clone()
                };

                let msg = ClientMessage::Input {
                    tick: 0,
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

                tokio::time::sleep(std::time::Duration::from_millis(16)).await;
            }
        });
    } else {
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
            &mut ctx
        )?;
    }

    ggez::event::run(
        ctx,
        event_loop,
        SharedGameState {
            input: InputState::default(),
            game_state: client.game_state.unwrap(),
        }
    )
}

#[derive(Default)]
struct InputState {
    pressed: HashSet<KeyCode>,
}

struct SharedGameState{
    input: InputState,
    game_state: Arc<Mutex<GameState>>,
}

impl EventHandler for SharedGameState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        if let Ok(mut gs) = self.game_state.try_lock() {
            gs.update_input(&self.input.pressed.clone());
            return gs.render_update(ctx);
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        if let Ok(mut gs) = self.game_state.try_lock() {
            return gs.draw(ctx);
        }

        Ok(())
    }

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        key: KeyInput,
        _repeated: bool,
    ) -> GameResult {
        if let Some(code) = key.keycode {
            self.input.pressed.insert(code);
        }

        Ok(())
    }

    fn key_up_event(
        &mut self,
        _ctx: &mut Context,
        key: KeyInput
    ) -> GameResult {
        if let Some(code) = key.keycode {
            self.input.pressed.remove(&code);
        }

        Ok(())
    }
}
