use bincode::config;
use bincode::serde::encode_to_vec;
use ggez::graphics::Rect as GgezRect;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use std::collections::HashSet;
use std::net::SocketAddr;
use crate::{
    network::ServerMessage,
    rect::Rect,
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

#[must_use]
pub fn get_combo_multiplier(combo: u32) -> f32 {
    (combo * combo) as f32 * 0.01 + 1.0
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

pub fn rect_to_ggez(rect: &Rect) -> GgezRect {
    GgezRect::new(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
    )
}
