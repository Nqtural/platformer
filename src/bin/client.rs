use ggez::{
    ContextBuilder,
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
        TEAM_ONE_COLOR,
        TEAM_ONE_START_POS,
        TEAM_TWO_COLOR,
        TEAM_TWO_START_POS,
        VIRTUAL_HEIGHT,
        VIRTUAL_WIDTH,
    },
    game_state::GameState,
    network::{
        ClientMessage,
        ServerMessage,
    },
    read_config::Config,
    player::Player,
    team::Team,
};
use bincode::{serde::{encode_to_vec, decode_from_slice}, config};

#[tokio::main]
async fn main() -> GameResult {
    let config = Config::get()?;
    // Setup game window and run event loop as usual
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
                vec![Player::new(TEAM_ONE_START_POS, "Player1".into())],
                TEAM_ONE_COLOR,
                TEAM_ONE_START_POS,
            ),
            Team::new(
                vec![Player::new(TEAM_TWO_START_POS, "Player2".into())],
                TEAM_TWO_COLOR,
                TEAM_TWO_START_POS,
            ),
        ],
        &mut ctx
    )?));

    let bincode_config = config::standard();
    let gs_clone_send = Arc::clone(&game_state);
    let gs_clone_recv = Arc::clone(&game_state);

    let ip = config.serverip();
    let port = config.serverport();
    let server_addr: SocketAddr = format!("{}:{}", ip, port).parse().unwrap();
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());

    // Spawn receive task
    let socket_recv = Arc::clone(&socket);
    let config_recv = bincode_config.clone();
    tokio::spawn(async move {
        let mut buf = [0u8; 2048];
        loop {
            match socket_recv.recv_from(&mut buf).await {
                Ok((len, _)) => {
                    if let Ok((ServerMessage::Snapshot(server_state), _)) =
    decode_from_slice::<ServerMessage, _>(&buf[..len], config_recv)
                    {
                        let mut gs = gs_clone_recv.lock().await;

                        // Preserve local input(s)
                        let local_inputs: Vec<_> = gs
                            .teams
                            .iter()
                            .map(|team| {
                                team.players
                                    .iter()
                                    .map(|p| p.input.clone())
                                    .collect::<Vec<_>>()
                            })
                            .collect();

                        // Merge server state
                        *gs = server_state;

                        // Restore player inputs
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
                    // You may want to consider a delay or termination condition here
                }
            }
        }
    });

    // Spawn send task
    let socket_send = Arc::clone(&socket);
    tokio::spawn(async move {
        //let mut last_input = PlayerInput::default();
        loop {
            let input = {
                let gs = gs_clone_send.lock().await;
                gs.teams[C_TEAM].players[C_PLAYER].input.clone()
            };

            //if input != last_input {
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
                //last_input = input;
            //}

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
