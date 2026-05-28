use foundation::GameMode;
use simulation::PlayerInput;
use simulation::game_state::GameState;
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use uuid::Uuid;

#[derive(Default)]
pub struct Queues {
    pub solos: Queue,
    pub duos: Queue,
}

#[derive(Default)]
pub struct Queue {
    players: VecDeque<Uuid>,
}

impl Queue {
    pub fn add(&mut self, session_id: Uuid) {
        self.players.push_back(session_id)
    }

    pub fn remove(&mut self, session_id: Uuid) {
        self.players.retain(|&p| p != session_id)
    }

    pub fn len(&self) -> usize {
        self.players.len()
    }

    pub fn get_and_remove_players(&mut self, player_count: usize) -> Vec<Uuid> {
        self.players.drain(..player_count).collect()
    }
}

#[derive(Debug)]
pub struct ClientSession {
    pub client_id: Uuid,
    pub player_name: String,
    pub state: ClientState,
    pub addr: SocketAddr,
}

#[derive(Debug)]
pub enum ClientState {
    Menu,
    Queueing(GameMode),
    InGame,
}

pub struct GameSession {
    pub game_id: Uuid,
    pub players: Vec<Uuid>,
    pub game_state: GameState,
    pub input_rx: UnboundedReceiver<GameInput>,
}

#[derive(Clone)]
pub struct GameHandle {
    pub game_id: Uuid,
    pub players: HashMap<Uuid, PlayerSlot>,
    pub input_tx: UnboundedSender<GameInput>,
}

#[derive(Clone)]
pub struct PlayerSlot {
    pub team_id: usize,
    pub player_id: usize,
    pub client_id: Uuid,
}

pub struct GameInput {
    pub client_id: Uuid,
    pub client_tick: u64,
    pub input: PlayerInput,
}
