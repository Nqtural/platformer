use anyhow::Result;
use bimap::BiMap;
use foundation::GameMode;
use foundation::color::Color;
use futures::future::pending;
use game_config::read::Config;
use protocol::constants::{DUO_OFFSET, TEAM_ONE_START_POS, TEAM_TWO_START_POS};
use protocol::net_client::ClientMessage;
use protocol::net_game_state;
use protocol::net_server::ServerMessage;
use server_logic::runtime::{
    ClientSession, GameHandle, GameInput, PlayerSlot, Queues, SessionState,
};
use simulation::constants::FIXED_DT;
use simulation::game_state::GameState;
use simulation::team::Team;
use simulation::{Player, PlayerInput};
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
    pub fn new(socket: Arc<UdpSocket>) -> Self {
        Self {
            socket,
            sessions: RwLock::new(HashMap::new()),
            connections: RwLock::new(BiMap::new()),
            queues: Mutex::new(Queues::default()),
            games: RwLock::new(HashMap::new()),
        }
    }
    pub async fn run(self: Arc<Self>) {
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

    async fn handle_packet(&self, msg: ClientMessage, addr: SocketAddr) {
        if let ClientMessage::Hello { player_name } = &msg {
            let client_id = Uuid::new_v4();

            let session = ClientSession {
                client_id,
                player_name: player_name.clone(),
                state: SessionState::Menu,
                addr,
                current_game: None,
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

    async fn queue_player(&self, client_id: Uuid, mode: GameMode) {
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
                session.state = SessionState::Queueing(mode.clone());
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
            session.state = SessionState::Menu;
        }
    }

    async fn try_start_match(&self, mode: GameMode) {
        let mut queues = self.queues.lock().await;
        let players = match mode {
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

        match self.start_game_instance(players, mode).await {
            Ok(_) => {}
            Err(e) => eprintln!("Failed to start game: {e}"),
        };
    }

    async fn start_game_instance(&self, player_ids: Vec<Uuid>, mode: GameMode) -> Result<()> {
        let players;
        {
            let sessions = self.sessions.read().await;

            players = player_ids
                .iter()
                .filter_map(|id| sessions.get(id).map(|s| (*id, s.player_name.clone())))
                .collect::<Vec<(Uuid, String)>>();
        }

        let teams: [Vec<String>; 2] = match mode {
            GameMode::Solos => {
                vec![vec![players[0].1.clone()], vec![players[1].1.clone()]]
            }
            GameMode::Duos => {
                vec![
                    vec![players[0].1.clone(), players[1].1.clone()],
                    vec![players[2].1.clone(), players[3].1.clone()],
                ]
            }
        }
        .try_into()
        .unwrap();

        let built_teams: Vec<Team> = teams
            .iter()
            .enumerate()
            .map(|(team_index, team_players)| {
                Team::new(
                    team_players
                        .iter()
                        .enumerate()
                        .map(|(player_index, player_name)| {
                            Player::new(
                                spawn_position(team_index, player_index),
                                player_name.clone(),
                                if team_index == 0 {
                                    Color::new(0.0, 0.0, 1.0, 1.0)
                                } else {
                                    Color::new(1.0, 0.0, 0.0, 1.0)
                                },
                                team_index,
                                0.0,
                                0.0,
                                0.0,
                            )
                        })
                        .collect(),
                )
            })
            .collect();

        // TODO: can just unwrap instead of match once Team implements Debug
        let built_teams: [Team; 2] = match built_teams.try_into() {
            Ok(teams) => teams,
            Err(_) => panic!("Expected exactly 2 teams"),
        };
        let gs = GameState::new(0, 0, built_teams);

        let (input_tx, input_rx) = unbounded_channel::<GameInput>();
        let game_id = Uuid::new_v4();
        {
            let mut sessions = self.sessions.write().await;

            for player_id in &player_ids {
                if let Some(session) = sessions.get_mut(player_id) {
                    session.current_game = Some(game_id);
                    session.state = SessionState::InGame(game_id);
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

        let player_names: [Vec<String>; 2] = teams.clone();
        let connections = self.connections.read().await;

        for uuid in players.keys() {
            let addr = match connections.get_by_right(uuid) {
                Some(a) => a,
                None => continue,
            };

            let slot = handle.players.get(uuid).unwrap();
            self.socket
                .send_to(
                    &serialize(&ServerMessage::StartGame {
                        c_team_id: slot.team_id,
                        c_player_id: slot.player_id,
                        player_names: player_names.clone(),
                    })?,
                    addr,
                )
                .await?;
        }

        let player_addrs: Vec<SocketAddr> = players
            .iter()
            .filter_map(|(id, _)| connections.get_by_right(id).copied())
            .collect();
        let socket = Arc::clone(&self.socket);
        tokio::spawn(async move {
            println!("Starting game with id '{game_id}'");

            if let Err(e) = handle_game(gs, input_rx, socket, players, player_addrs).await {
                eprintln!("Game ' {game_id}' crashed: {e}");
            }
        });

        Ok(())
    }

    async fn route_input(&self, client_id: Uuid, client_tick: u64, input: PlayerInput) {
        let game_id = {
            let sessions = self.sessions.read().await;

            sessions.get(&client_id).and_then(|s| s.current_game)
        };

        let Some(game_id) = game_id else {
            return;
        };

        let games = self.games.read().await;

        let Some(game) = games.get(&game_id) else {
            return;
        };

        let _ = game.input_tx.send(GameInput {
            client_id,
            client_tick,
            input,
        });
    }
}

async fn handle_game(
    mut gs: GameState,
    mut input_rx: UnboundedReceiver<GameInput>,
    socket: Arc<UdpSocket>,
    players: HashMap<Uuid, PlayerSlot>,
    player_addrs: Vec<SocketAddr>,
) -> Result<()> {
    let mut tick: u64 = 0;

    loop {
        let frame_start = Instant::now();

        while let Ok(input) = input_rx.try_recv() {
            let Some(slot) = players.get(&input.client_id) else {
                continue;
            };

            if let Some(team) = gs.teams.get_mut(slot.team_id)
                && let Some(player) = team.players.get_mut(slot.player_id)
            {
                player.input = input.input;
            }
        }

        gs.fixed_update(FIXED_DT);

        let snapshot = net_game_state::to_net(&gs);

        let msg = ServerMessage::Snapshot {
            server_tick: tick,
            server_state: snapshot,
        };

        let bytes = serialize(&msg)?;

        println!("Snapshot size: {}", bytes.len());

        for addr in &player_addrs {
            let _ = socket.send_to(&bytes, addr).await;
        }

        tick += 1;

        sleep_until_next_tick(frame_start).await;
    }
}

async fn sleep_until_next_tick(frame_start: Instant) {
    let tick_duration = Duration::from_secs_f32(FIXED_DT);

    let elapsed = frame_start.elapsed();

    if elapsed < tick_duration {
        sleep(tick_duration - elapsed).await;
    }
}

fn spawn_position(team_id: usize, player_id: usize) -> [f32; 2] {
    match (team_id, player_id) {
        (0, 0) => TEAM_ONE_START_POS,
        (0, 1) => [TEAM_ONE_START_POS[0] + DUO_OFFSET, TEAM_ONE_START_POS[1]],
        (1, 0) => TEAM_TWO_START_POS,
        (1, 1) => [TEAM_TWO_START_POS[0] - DUO_OFFSET, TEAM_TWO_START_POS[1]],
        _ => unreachable!(),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::get()?;

    let server = Arc::new(Server::new(Arc::new(
        UdpSocket::bind(format!("{}:{}", config.serverip(), config.serverport())).await?,
    )));

    tokio::select! {
        _ = server.run() => {}
        _ = tokio::signal::ctrl_c() => {}
    }

    println!("\nStopping server...");

    Ok(())
}
