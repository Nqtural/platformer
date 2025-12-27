use anyhow::Result;
use bincode::{serde::{encode_to_vec, decode_from_slice}, config};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::{Mutex, RwLock};
use protocol::{
    lobby::Lobby,
    net_client::ClientMessage,
    net_game_state,
    net_server::ServerMessage,
    utils::{broadcast, send_to},
};
use simulation::constants::FIXED_DT;
use crate::ServerState;

pub struct NetworkServer {
    socket: Arc<UdpSocket>,
    clients: Arc<RwLock<HashSet<SocketAddr>>>,
    bincode_config: bincode::config::Configuration,
}

impl NetworkServer {
    pub async fn new(bind_ip: &str, port: &str) -> Result<Self> {
        let socket = Arc::new(UdpSocket::bind(format!("{bind_ip}:{port}")).await?);
        Ok(Self {
            socket,
            clients: Arc::new(RwLock::new(HashSet::new())),
            bincode_config: config::standard(),
        })
    }

    pub async fn handshake(&self, lobby: Arc<Mutex<Lobby>>) -> Result<()> {
        println!("Waiting for clients...");
        let mut buf = [0u8; 1500];

        loop {
            let (len, addr) = self.socket.recv_from(&mut buf).await?;
            let (msg, _): (ClientMessage, usize) =
                decode_from_slice(&buf[..len], self.bincode_config)
                    .map_err(|e| ggez::GameError::CustomError(e.to_string()))?;

            if let ClientMessage::Hello { name } = msg {
                self.clients.write().await.insert(addr);
                let mut lobby_locked = lobby.lock().await;
                let (team_id, player_id) = lobby_locked.assign_slot(addr, name.clone());
                println!(
                    "{} joined as {} ({}/{})",
                    addr,
                    name,
                    lobby_locked.connected_count(),
                    lobby_locked.required(),
                );

                // send welcome
                let welcome = ServerMessage::Welcome { team_id, player_id };
                send_to(addr, welcome, &self.socket, &self.bincode_config).await;

                // broadcast lobby status
                let status = ServerMessage::LobbyStatus {
                    players: lobby_locked.players.clone(),
                    required: lobby_locked.required(),
                };
                broadcast(status, &self.clients, &self.socket, &self.bincode_config).await;
            }

            // exit when lobby is full
            if lobby.lock().await.is_full() {
                break;
            }
        }
        
        Ok(())
    }

    pub fn clients(&self) -> Arc<RwLock<HashSet<SocketAddr>>> {
        Arc::clone(&self.clients)
    }

    pub fn socket(&self) -> Arc<UdpSocket> {
        Arc::clone(&self.socket)
    }

    pub fn bincode_config(&self) -> bincode::config::Configuration {
        self.bincode_config
    }

    pub async fn spawn_receive_task(
        &self,
        server_state: Arc<Mutex<ServerState>>,
    ) {
        let socket = Arc::clone(&self.socket);
        let clients = Arc::clone(&self.clients);
        let bincode_config = self.bincode_config;

        tokio::spawn(async move {
            let mut buf = [0u8; 2048];

            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((len, addr)) => {
                        // track client for snapshots
                        clients.write().await.insert(addr);

                        if let Ok((msg, _)) = decode_from_slice::<ClientMessage, _>(&buf[..len], bincode_config) {
                            match msg {
                                ClientMessage::Input { team_id, player_id, input, .. } => {
                                    let mut gs = server_state.lock().await;
                                    if let Some(game_state_arc) = &mut gs.game_state {
                                        let mut gs_locked = game_state_arc.lock().await;
                                        if let Some(team) = gs_locked.teams.get_mut(team_id)
                                        && let Some(player) = team.players.get_mut(player_id) {
                                            player.set_input(input);
                                        }
                                    }
                                }
                                ClientMessage::Hello { .. } => {}
                            }
                        }
                    }
                    Err(e) => eprintln!("Receive error: {e}"),
                }
            }
        });
    }

    pub async fn spawn_send_task(&self, server_state: Arc<Mutex<ServerState>>) {
        let socket = Arc::clone(&self.socket);
        let clients = Arc::clone(&self.clients);
        let bincode_config = self.bincode_config;
        let tick = Arc::clone(&server_state.lock().await.tick);

        tokio::spawn(async move {
            let interval = std::time::Duration::from_secs_f32(FIXED_DT);

            loop {
                tokio::time::sleep(interval).await;

                let server_tick = tick.load(std::sync::atomic::Ordering::Relaxed);

                let gs_clone = {
                    let server_guard = server_state.lock().await;
                    let gs_guard = server_guard.game_state.as_ref().unwrap().lock().await;
                    gs_guard.clone()
                };

                {
                    let mut server_guard = server_state.lock().await;
                    server_guard.render_clock.update(server_tick);
                }

                let snapshot = net_game_state::to_net(&gs_clone);

                let snapshot_msg = ServerMessage::Snapshot {
                    server_tick,
                    server_state: snapshot,
                };

                let data = match encode_to_vec(&snapshot_msg, bincode_config) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("Encoding error: {e}");
                        continue;
                    }
                };

                let client_addrs = clients.read().await.clone();
                for client in &client_addrs {
                    let _ = socket.send_to(&data, client).await;
                }
            }
        });
    }
}
