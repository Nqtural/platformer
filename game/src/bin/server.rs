use anyhow::Result;
use bimap::BiMap;
use foundation::GameMode;
use futures::future::pending;
use game_config::read::Config;
use protocol::init::{InitData, InitPlayerData};
use protocol::net_client::ClientMessage;
use protocol::net_game_state;
use protocol::net_server::ServerMessage;
use server_logic::runtime::{
    ClientSession, ClientState, GameHandle, GameInput, PlayerSlot, Queues,
};
use simulation::PlayerInput;
use simulation::constants::FIXED_DT;
use simulation::game_state::GameState;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, net::SocketAddr};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tokio::time::{Instant, sleep};
use tokio::{
    net::UdpSocket,
    sync::{Mutex, RwLock},
};
use uuid::Uuid;
use wincode::serialize;

pub struct Server {
    pub socket: Arc<UdpSocket>,
    pub sessions: RwLock<HashMap<Uuid, ClientSession>>,
    pub connections: RwLock<BiMap<SocketAddr, Uuid>>,
    pub queues: Mutex<Queues>,
    pub games: RwLock<HashMap<Uuid, GameHandle>>,
}

impl Server {
    pub fn new(socket: Arc<UdpSocket>) -> Arc<Self> {
        Arc::new(Self {
            socket,
            sessions: RwLock::new(HashMap::new()),
            connections: RwLock::new(BiMap::new()),
            queues: Mutex::new(Queues::default()),
            games: RwLock::new(HashMap::new()),
        })
    }

    pub async fn run(self: &Arc<Self>) {
        self.spawn_network_task();

        pending::<()>().await;
    }

    pub fn spawn_network_task(self: &Arc<Self>) {
        let server = Arc::clone(self);

        tokio::spawn(async move {
            let mut buf = [0u8; 2048];

            loop {
                let (len, addr) = match server.socket.recv_from(&mut buf).await {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let msg = match wincode::deserialize::<ClientMessage>(&buf[..len]) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                server.handle_packet(msg, addr).await;
            }
        });
    }

    async fn handle_packet(self: &Arc<Self>, msg: ClientMessage, addr: SocketAddr) {
        if let ClientMessage::Hello { player_name } = &msg {
            let client_id = Uuid::new_v4();

            let session = ClientSession {
                client_id,
                player_name: player_name.clone(),
                state: ClientState::Menu,
                addr,
            };

            self.sessions.write().await.insert(client_id, session);
            self.connections.write().await.insert(addr, client_id);
        }
        let client_id_optional = {
            let connections = self.connections.read().await;
            connections.get_by_left(&addr).copied()
        };
        let client_id = match client_id_optional {
            Some(id) => id,
            None => return,
        };

        match msg {
            ClientMessage::Hello { player_name: _ } => {} // already handled
            ClientMessage::QueueJoin(mode) => {
                self.queue_player(client_id, mode).await;
            }
            ClientMessage::QueueLeave => {
                self.leave_queue(client_id).await;
            }
            ClientMessage::Input { client_tick, input } => {
                self.route_input(client_id, client_tick, input).await;
            }
        }
    }

    async fn queue_player(self: &Arc<Self>, client_id: Uuid, mode: GameMode) {
        self.leave_queue(client_id).await;

        {
            let mut queues = self.queues.lock().await;
            match mode {
                GameMode::Solos => queues.solos.add(client_id),
                GameMode::Duos => queues.duos.add(client_id),
            }
        }

        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(&client_id) {
                session.state = ClientState::Queueing(mode.clone());
            }
        }

        self.try_start_match(mode).await;
    }

    async fn leave_queue(&self, client_id: Uuid) {
        let mut queues = self.queues.lock().await;

        queues.solos.remove(client_id);
        queues.duos.remove(client_id);

        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&client_id) {
            session.state = ClientState::Menu;
        }
    }

    async fn try_start_match(self: &Arc<Self>, mode: GameMode) {
        let players;
        {
            let mut queues = self.queues.lock().await;
            players = match mode {
                GameMode::Solos => {
                    if queues.solos.len() < 2 {
                        return;
                    }
                    queues.solos.get_and_remove_players(2)
                }

                GameMode::Duos => {
                    if queues.duos.len() < 4 {
                        return;
                    }
                    queues.duos.get_and_remove_players(4)
                }
            };
        }

        match self.start_game_instance(players, mode).await {
            Ok(_) => {}
            Err(e) => eprintln!("Failed to start game: {e}"),
        };
    }

    async fn start_game_instance(
        self: &Arc<Self>,
        player_ids: Vec<Uuid>,
        mode: GameMode,
    ) -> Result<()> {
        let teams = match mode {
            GameMode::Solos => [
                vec![player_ids[0].to_string()],
                vec![player_ids[1].to_string()],
            ],
            GameMode::Duos => [
                vec![player_ids[0].to_string(), player_ids[1].to_string()],
                vec![player_ids[2].to_string(), player_ids[3].to_string()],
            ],
        };

        let mut players = HashMap::new();
        {
            let sessions = self.sessions.read().await;
            for player_id in &player_ids {
                players.insert(
                    player_id.to_string(),
                    InitPlayerData {
                        name: sessions.get(player_id).unwrap().player_name.clone(),
                    },
                );
            }
        }
        let init_data = InitData { players, teams };

        let gs = init_data.to_game_state();

        let (input_tx, input_rx) = unbounded_channel::<GameInput>();
        let game_id = Uuid::new_v4();
        {
            let mut sessions = self.sessions.write().await;

            for player_id in &player_ids {
                if let Some(session) = sessions.get_mut(player_id) {
                    session.state = ClientState::InGame;
                }
            }
        }

        let mut players = HashMap::new();
        match mode {
            GameMode::Solos => {
                players.insert(
                    player_ids[0],
                    PlayerSlot {
                        team_id: 0,
                        player_id: 0,
                        client_id: player_ids[0],
                    },
                );

                players.insert(
                    player_ids[1],
                    PlayerSlot {
                        team_id: 1,
                        player_id: 0,
                        client_id: player_ids[1],
                    },
                );
            }

            GameMode::Duos => {
                players.insert(
                    player_ids[0],
                    PlayerSlot {
                        team_id: 0,
                        player_id: 0,
                        client_id: player_ids[0],
                    },
                );

                players.insert(
                    player_ids[1],
                    PlayerSlot {
                        team_id: 0,
                        player_id: 1,
                        client_id: player_ids[1],
                    },
                );

                players.insert(
                    player_ids[2],
                    PlayerSlot {
                        team_id: 1,
                        player_id: 0,
                        client_id: player_ids[2],
                    },
                );

                players.insert(
                    player_ids[3],
                    PlayerSlot {
                        team_id: 1,
                        player_id: 1,
                        client_id: player_ids[3],
                    },
                );
            }
        }

        let handle = GameHandle {
            game_id,
            players: players.clone(),
            input_tx,
        };
        {
            self.games.write().await.insert(game_id, handle.clone());
        }

        let connections = self.connections.read().await;

        for uuid in players.keys() {
            let addr = match connections.get_by_right(uuid) {
                Some(a) => a,
                None => continue,
            };

            self.socket
                .send_to(
                    &serialize(&ServerMessage::StartGame {
                        c_player: uuid.to_string(),
                        init_data: init_data.clone(),
                    })?,
                    addr,
                )
                .await?;
        }

        let player_addrs: Vec<SocketAddr> = players
            .iter()
            .filter_map(|(id, _)| connections.get_by_right(id).copied())
            .collect();
        let server = Arc::clone(self);
        tokio::spawn(async move {
            println!("Starting game with id '{game_id}'");

            if let Err(e) = server
                .handle_game(gs, input_rx, players, player_addrs, game_id)
                .await
            {
                eprintln!("Game ' {game_id}' crashed: {e}");
            }
        });

        Ok(())
    }

    async fn route_input(&self, client_id: Uuid, client_tick: u64, input: PlayerInput) {
        let games = self.games.read().await;

        let game = games.values().find(|g| g.players.contains_key(&client_id));

        let Some(game) = game else {
            return;
        };

        let _ = game.input_tx.send(GameInput {
            client_id,
            client_tick,
            input,
        });
    }

    async fn handle_game(
        self: Arc<Self>,
        mut gs: GameState,
        mut input_rx: UnboundedReceiver<GameInput>,
        players: HashMap<Uuid, PlayerSlot>,
        player_addrs: Vec<SocketAddr>,
        game_id: Uuid,
    ) -> Result<()> {
        let mut tick: u64 = 0;

        loop {
            let frame_start = Instant::now();

            while let Ok(input) = input_rx.try_recv() {
                let Some(player) = gs.players.get_mut(&input.client_id) else {
                    continue;
                };
                player.input = input.input;
            }

            gs.update(FIXED_DT);

            let snapshot = net_game_state::to_net(&gs);
            let msg = ServerMessage::Snapshot {
                server_tick: tick,
                server_state: snapshot,
            };
            let bytes = serialize(&msg)?;
            for addr in &player_addrs {
                let _ = self.socket.send_to(&bytes, addr).await;
            }

            tick += 1;

            sleep_until_next_tick(frame_start).await;

            if gs.is_game_over() {
                break;
            }
        }

        self.games.write().await.remove(&game_id);

        let mut sessions = self.sessions.write().await;

        let msg = ServerMessage::EndGame;
        let bytes = serialize(&msg)?;
        for addr in &player_addrs {
            let _ = self.socket.send_to(&bytes, addr).await;
        }

        for client_id in players.keys() {
            if let Some(session) = sessions.get_mut(client_id) {
                session.state = ClientState::Menu;
            }
        }

        Ok(())
    }
}

async fn sleep_until_next_tick(frame_start: Instant) {
    let tick_duration = Duration::from_secs_f32(FIXED_DT);

    let elapsed = frame_start.elapsed();

    if elapsed < tick_duration {
        sleep(tick_duration - elapsed).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::get()?;

    let server = Server::new(Arc::new(
        UdpSocket::bind(format!("{}:{}", config.serverip(), config.serverport())).await?,
    ));

    tokio::select! {
        _ = server.run() => {}
        _ = tokio::signal::ctrl_c() => {}
    }

    println!("\nStopping server...");

    Ok(())
}
