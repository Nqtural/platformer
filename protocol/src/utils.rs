use crate::net_server::ServerMessage;
use std::collections::HashSet;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use wincode::serialize;

pub fn condense_name(mut name: &str) -> String {
    // Skip leading "The"
    if name.len() >= 3 && name[..3].eq_ignore_ascii_case("the") {
        name = &name[3..];
    }

    name.chars()
        .filter(|c| c.is_ascii_alphanumeric()) // excludes spaces automatically
        .take(3)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

pub async fn send_to(addr: SocketAddr, msg: ServerMessage, socket: &UdpSocket) {
    if let Ok(data) = serialize(&msg) {
        let _ = socket.send_to(&data, addr).await;
    }
}

pub async fn broadcast(
    msg: ServerMessage,
    clients: &RwLock<HashSet<SocketAddr>>,
    socket: &UdpSocket,
) {
    if let Ok(data) = serialize(&msg) {
        let list = clients.read().await;
        for client in list.iter() {
            let _ = socket.send_to(&data, client).await;
        }
    }
}
