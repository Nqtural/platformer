use anyhow::Result;
use ggez::input::keyboard::KeyCode;
use tokio::net::UdpSocket;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::net::SocketAddr;
use game_config::read::Config;
use client_logic::ClientState;
use protocol::{
    net_client::ClientMessage,
    net_game_state,
    net_server::ServerMessage,
};
use simulation::{
    constants::TICK_RATE,
    input::PlayerInput,
};
use bincode::{serde::{encode_to_vec, decode_from_slice}, config};

#[tokio::main]
async fn main() -> Result<()> {
    // get configuration
    let config = Config::get()?;

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

    // handle server responses until StartGame signal
    let mut buf = [0u8; 1500];

    let mut team_id: Option<usize> = None;
    let mut player_id: Option<usize> = None;

    let init_teams = loop {
        let (len, _addr) = socket.recv_from(&mut buf).await?;
        let (msg, _): (ServerMessage, usize) =
        decode_from_slice(&buf[..len], bincode_config)?;

        match msg {
            ServerMessage::Welcome {
                team_id: t,
                player_id: p,
            } => {
                team_id = Some(t);
                player_id = Some(p);
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

    let team_id = team_id.expect("StartGame received before Welcome");
    let player_id = player_id.expect("StartGame received before Welcome");

    let client = ClientState::new(
        team_id,
        player_id,
        init_teams,
    )?;

    // spawn receive task
    let socket_recv = Arc::clone(&socket);
    let config_recv = bincode_config;
    let history_clone_recv = Arc::clone(&client.snapshot_history);
    let render_tick_update = Arc::clone(&client.render_tick);
    let render_clock = Arc::clone(&client.render_clock);
    let core = Arc::clone(&client.core);
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
                            let mut render_clock_locked = render_clock.lock().await;
                            render_clock_locked.update(server_tick);
                            let mut render_tick = render_tick_update.lock().await;
                            *render_tick = render_clock_locked.render_tick();
                            drop(render_tick);

                            // apply server snapshot
                            let mut core = core.lock().await;
                            net_game_state::apply_snapshot(core.game_state_mut(), &server_state);
                            snapshot_history.push(server_tick, core.game_state().clone());
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
    let current_input = Arc::clone(&client.current_input);
    tokio::spawn(async move {
        let tick_duration = std::time::Duration::from_millis(1000 / TICK_RATE as u64);
        loop {
            let start = std::time::Instant::now();
            let tick = tick_send.load(Ordering::Relaxed);

            let pressed = current_input.lock().await.clone();
            let mut input = PlayerInput::default();
            input.update(&pressed);

            let msg = ClientMessage::Input {
                client_tick: tick,
                team_id: client.team_id,
                player_id: client.player_id,
                input,
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
