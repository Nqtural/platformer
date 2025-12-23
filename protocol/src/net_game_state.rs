use anyhow::{anyhow, Result};
use simulation::{
    game_state::GameState,
    team::Team,
};
use crate::{
    net_player,
    net_server::NetSnapshot,
    net_team,
};

pub fn new_from_initial(
    c_team: usize,
    c_player: usize,
    init: Vec<net_team::InitTeamData>,
) -> Result<GameState> {
    let teams_vec: Vec<Team> = init
        .into_iter()
        .map(net_team::from_init)
        .collect();

    let teams: [Team; 2] = teams_vec
        .try_into()
        .map_err(|v: Vec<Team>| {
            anyhow!("Expected exactly 2 teams, got {}", v.len())
        })?;

    Ok(GameState::new(c_team, c_player, teams))
}

#[must_use]
pub fn to_net(gs: &GameState) -> NetSnapshot {
    NetSnapshot {
        tick: 0,
        winner: gs.winner,
        players: gs.teams.iter().flat_map(|team| {
            team.players.iter().enumerate().map(move |(player_idx, player)| {
                net_player::to_net(player, player_idx)
            })
        }).collect(),
    }
}

pub fn apply_snapshot(gs: &mut GameState, snapshot: &NetSnapshot) {
    gs.winner = snapshot.winner;

    for net_player in &snapshot.players {
        if let Some(team) = gs.teams.get_mut(net_player.team_idx)
        && let Some(player) = team.players.get_mut(net_player.player_idx) {
            net_player::from_net(player, net_player);
        }
    }
}

#[must_use]
pub fn to_snapshot(gs: &GameState) -> NetSnapshot {
    let mut net_players = Vec::new();

    for team in &gs.teams {
        for (player_idx, player) in team.players.iter().enumerate() {
            net_players.push(net_player::to_net(player, player_idx));
        }
    }

    NetSnapshot {
        tick: 0,
        winner: gs.winner,
        players: net_players,
    }
}
