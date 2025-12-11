use bincode::config;
use bincode::serde::encode_to_vec;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use std::collections::HashSet;
use std::net::SocketAddr;
use crate::net_server::ServerMessage;

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
