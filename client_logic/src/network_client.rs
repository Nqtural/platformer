use crate::{ClientState, runtime::ClientEvent};
use anyhow::Result;
use foundation::GameMode;
use protocol::{net_client::ClientMessage, net_game_state, net_server::ServerMessage};
use simulation::PlayerInput;
use std::{
    net::SocketAddr,
    sync::{Arc, atomic::Ordering},
};
use tokio::net::UdpSocket;
use wincode::{deserialize, serialize};

#[derive(Clone)]
pub struct NetworkClient {
    socket: Arc<UdpSocket>,
    server_addr: SocketAddr,
}

impl NetworkClient {
    pub async fn new(
        client_ip: &str,
        client_port: &str,
        server_ip: &str,
        server_port: &str,
    ) -> Self {
        let server_addr: SocketAddr = format!("{server_ip}:{server_port}")
            .parse()
            .expect("Fatal: Unable to parse server IP from configuration file");
        let socket = Arc::new(
            UdpSocket::bind(format!("{}:{}", client_ip, client_port))
                .await
                .expect("Fatal: Unable to create client socket"),
        );
        socket
            .connect(server_addr)
            .await
            .expect("Fatal: Unable to connect to server");

        Self {
            socket,
            server_addr,
        }
    }

    pub async fn handshake(&self, player_name: &str) -> Result<()> {
        let packet = serialize(&ClientMessage::Hello {
            player_name: player_name.to_string(),
        })?;
        self.socket.send(&packet).await?;

        Ok(())
    }

    pub async fn leave_queue(&self) -> Result<()> {
        self.socket
            .send(&serialize(&ClientMessage::QueueLeave)?)
            .await?;

        Ok(())
    }

    pub async fn enter_queue(&self, mode: GameMode) -> Result<()> {
        self.socket
            .send(&serialize(&ClientMessage::QueueJoin(mode))?)
            .await?;

        Ok(())
    }

    pub async fn poll_queue(&self) -> Result<ServerMessage> {
        let mut buf = [0u8; 2048];
        match self.socket.recv(&mut buf).await {
            Ok(n) => Ok(deserialize(&buf[..n])?),
            Err(e) => Err(anyhow::Error::from(e)),
        }
    }

    pub fn spawn_receive_task(&self, client: Arc<ClientState>) {
        let socket = Arc::clone(&self.socket);
        tokio::spawn(async move {
            let mut buf = [0u8; 2048];

            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((len, _)) => {
                        match deserialize::<ServerMessage>(&buf[..len]) {
                            Ok(ServerMessage::Snapshot {
                                server_tick,
                                server_state,
                            }) => {
                                let mut snapshot_history = client.snapshot_history.lock().await;

                                {
                                    // update render clock
                                    let mut render_clock_locked = client.render_clock.lock().await;
                                    render_clock_locked.update(server_tick);

                                    let mut render_tick = client.render_tick.lock().await;
                                    *render_tick = render_clock_locked.render_tick();
                                }

                                // apply server snapshot
                                let mut core = client.core.lock().await;
                                net_game_state::apply_snapshot(
                                    core.game_state_mut(),
                                    &server_state,
                                );

                                snapshot_history.push(server_tick, core.game_state().clone());
                            }
                            Ok(ServerMessage::EndGame) => {
                                let _ = client.event_tx.send(Some(ClientEvent::EndGame));
                                client.shutdown.store(true, Ordering::Relaxed);
                                return;
                            }
                            Ok(_) => {
                                // ignore other message types
                            }
                            Err(e) => {
                                eprintln!("Decode error: {e}");
                            }
                        }
                    }

                    Err(e) => eprintln!("Receive error: {e}"),
                }
            }
        });
    }

    pub fn spawn_send_task(&self, client: Arc<ClientState>) {
        let socket = Arc::clone(&self.socket);
        let server_addr = self.server_addr;
        tokio::spawn(async move {
            let tick_duration =
                std::time::Duration::from_millis(1000 / simulation::constants::TICK_RATE as u64);

            loop {
                if client.shutdown.load(Ordering::Relaxed) {
                    return;
                }

                let start = std::time::Instant::now();
                let tick = client.tick.load(Ordering::Relaxed);

                // collect input
                let pressed = client.current_input.lock().await.clone();
                let mut input = PlayerInput::default();
                input.update(&pressed);

                let msg = ClientMessage::Input {
                    client_tick: tick,
                    input,
                };

                match serialize(&msg) {
                    Ok(data) => {
                        let _ = socket.send_to(&data, server_addr).await;
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
}
