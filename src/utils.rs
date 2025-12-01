use bincode::config;
use bincode::serde::encode_to_vec;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use std::collections::HashSet;
use std::net::SocketAddr;
use crate::{
    network::ServerMessage,
    team::Team,
};

#[must_use]
pub fn approach_zero(value: f32, step: f32) -> f32 {
    if value > 0.0 {
        (value - step).max(0.0)
    } else if value < 0.0 {
        (value + step).min(0.0)
    } else {
        0.0
    }
}

pub fn current_and_enemy<const N: usize>(teams: &mut [Team; N], i: usize) -> (&mut Team, &mut Team) {
    assert!(N == 2 && (i == 0 || i == 1));
    let (left, right) = teams.split_at_mut(1);
    if i == 0 {
        (&mut left[0], &mut right[0])
    } else {
        (&mut right[0], &mut left[0])
    }
}

pub async fn send_to(addr: SocketAddr, msg: ServerMessage, socket: &UdpSocket, cfg: &config::Configuration) {
    if let Ok(data) = encode_to_vec(&msg, *cfg) {
        let _ = socket.send_to(&data, addr).await;
    }
}

pub async fn broadcast(msg: ServerMessage, clients: &RwLock<HashSet<SocketAddr>>, socket: &UdpSocket, cfg: &config::Configuration) {
    if let Ok(data) = encode_to_vec(&msg, *cfg) {
        let list = clients.read().await;
        for client in list.iter() {
            let _ = socket.send_to(&data, client).await;
        }
    }
}
