use crate::{net_player, net_server::NetSnapshot};
use simulation::game_state::GameState;
use uuid::Uuid;

#[must_use]
pub fn to_net(gs: &GameState) -> NetSnapshot {
    NetSnapshot {
        tick: 0,
        winner: gs.winner,
        players: gs.players.iter().map(net_player::to_net).collect(),
    }
}

pub fn apply_snapshot(gs: &mut GameState, snapshot: &NetSnapshot) {
    gs.winner = snapshot.winner;

    for net_player in &snapshot.players {
        if let Some(player) = gs
            .players
            .get_mut(&Uuid::parse_str(&net_player.player_id).expect("Invalid UUID string"))
        {
            net_player::from_net(player, net_player);
        }
    }
}

#[must_use]
pub fn to_snapshot(gs: &GameState) -> NetSnapshot {
    let mut net_players = Vec::new();

    for player in gs.players.iter() {
        net_players.push(net_player::to_net(player));
    }

    NetSnapshot {
        tick: 0,
        winner: gs.winner,
        players: net_players,
    }
}
