use anyhow::Result;
use bincode::{config, serde::{encode_to_vec, decode_from_slice}};
use protocol::{
    net_client::ClientMessage,
    net_game_state,
    net_server::ServerMessage,
    net_team::InitTeamData,
};
use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::Ordering,
    },
};
use tokio::net::UdpSocket;
use simulation::PlayerInput;
use crate::ClientState;

pub struct NetworkClient {
    socket: Arc<UdpSocket>,
    server_addr: SocketAddr,
    bincode_config: config::Configuration,
}

impl NetworkClient {
    pub async fn new(
        client_ip: &str,
        client_port: &str,
        server_ip: &str,
        server_port: &str
    ) -> Result<Self> {
        let server_addr: SocketAddr = format!("{}:{}", server_ip, server_port).parse()?;
        let socket = Arc::new(UdpSocket::bind(format!("{}:{}", client_ip, client_port)).await?);
        socket.connect(server_addr).await?;

        Ok(Self {
            socket,
            server_addr,
            bincode_config: config::standard(),
        })
    }

    pub async fn handshake(&self, player_name: &str) -> Result<(usize, usize, Vec<InitTeamData>)> {
        // send Hello packet
        let packet = encode_to_vec(
            &ClientMessage::Hello { name: player_name.to_string() },
            self.bincode_config,
        )?;
        self.socket.send(&packet).await?;

        let mut buf = [0u8; 1500];
        let mut team_id: Option<usize> = None;
        let mut player_id: Option<usize> = None;

        let init_teams = loop {
            let (len, _addr) = self.socket.recv_from(&mut buf).await?;
            let (msg, _): (ServerMessage, usize) = decode_from_slice(&buf[..len], self.bincode_config)?;

            match msg {
                ServerMessage::Welcome { team_id: t, player_id: p } => {
                    team_id = Some(t);
                    player_id = Some(p);
                }
                ServerMessage::StartGame { teams } => break teams,
                ServerMessage::LobbyStatus { .. } => {} // can log or update UI later
                _ => {}
            }
        };

        let team_id = team_id.expect("StartGame received before Welcome");
        let player_id = player_id.expect("StartGame received before Welcome");

        Ok((team_id, player_id, init_teams))
    }

    pub fn spawn_receive_task(
        &self,
        client: Arc<ClientState>,
    ) {
        let socket = Arc::clone(&self.socket);
        let config = self.bincode_config;
        tokio::spawn(async move {
            let mut buf = [0u8; 2048];

            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((len, _)) => {
                        if let Ok((ServerMessage::Snapshot { server_tick, server_state }, _)) =
                            decode_from_slice::<ServerMessage, _>(&buf[..len], config)
                        {
                            let mut snapshot_history = client.snapshot_history.lock().await;
                            {
                                // update render clock
                                let mut render_clock_locked = client.render_clock.lock().await;
                                render_clock_locked.update(server_tick);
                                let mut render_tick = client.render_tick.lock().await;
                                *render_tick = render_clock_locked.render_tick();
                                drop(render_tick);

                                // apply server snapshot
                                let mut core = client.core.lock().await;
                                net_game_state::apply_snapshot(core.game_state_mut(), &server_state);
                                snapshot_history.push(server_tick, core.game_state().clone());
                            }
                        }
                    }
                    Err(e) => eprintln!("Receive error: {e}"),
                }
            }
        });
    }

    pub fn spawn_send_task(
        &self,
        client: Arc<ClientState>,
    ) {
        let socket = Arc::clone(&self.socket);
        let config = self.bincode_config;
        let server_addr = self.server_addr;
        tokio::spawn(async move {
            let tick_duration = std::time::Duration::from_millis(1000 / simulation::constants::TICK_RATE as u64);

            loop {
                let start = std::time::Instant::now();
                let tick = client.tick.load(Ordering::Relaxed);

                // collect input
                let pressed = client.current_input.lock().await.clone();
                let mut input = PlayerInput::default();
                input.update(&pressed);

                let msg = ClientMessage::Input {
                    client_tick: tick,
                    team_id: client.team_id,
                    player_id: client.player_id,
                    input,
                };

                match encode_to_vec(&msg, config) {
                    Ok(data) => { let _ = socket.send_to(&data, server_addr).await; }
                    Err(e) => eprintln!("Encoding error: {e}"),
                }

                let elapsed = start.elapsed();
                if elapsed < tick_duration {
                    tokio::time::sleep(tick_duration - elapsed).await;
                }
            }
        });
    }
}
