use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::Mutex;
use game_config::read::Config;
use protocol::{
    net_game_state,
    net_server::ServerMessage,
    lobby::Lobby,
    utils::broadcast,
};
use server_logic::{
    NetworkServer,
    ServerState,
};
use simulation::constants::{
    TICK_RATE,
    FIXED_DT,
};

#[tokio::main]
async fn main() -> Result<()> {
    let mut server = ServerState::default();
    let config = Config::get()?;

    // initialize lobby state
    println!(
        "Initializing lobby state (team size: {})...",
        config.team_size(),
    );
    let lobby_state = Arc::new(Mutex::new(Lobby::new(config.team_size())));
    let network = NetworkServer::new(config.serverip(), config.serverport()).await?;

    network.handshake(Arc::clone(&lobby_state)).await?;

    // generate InitTeamData from lobby
    let init_teams = lobby_state.lock().await.initial_teams(
        config.team_one_color(),
        config.team_two_color(),
    );

    // create actual GameState from InitTeamData
    server.game_state = Some(Arc::new(Mutex::new(net_game_state::new_from_initial(0, 0, init_teams.clone())?)));

    // broadcast to clients
    broadcast(
        ServerMessage::StartGame { teams: init_teams },
        &network.clients(),
        &network.socket(),
        &network.bincode_config(),
    ).await;

    println!("Starting game...");
    // simulation loop
    let game_state_tick = Arc::clone(server.game_state.as_ref().unwrap());
    let tick = Arc::clone(&server.tick);
    tokio::spawn(async move {
        let tick_duration = std::time::Duration::from_millis(1000 / TICK_RATE as u64);
        loop {
            let start = std::time::Instant::now();

            {
                let mut gs = game_state_tick.lock().await;
                gs.fixed_update(FIXED_DT);
            }

            tick.fetch_add(1, Ordering::Relaxed);

            let elapsed = start.elapsed();
            if elapsed < tick_duration {
                tokio::time::sleep(tick_duration - elapsed).await;
            }
        }
    });

    let server_shared = Arc::new(Mutex::new(server));
    network.spawn_receive_task(Arc::clone(&server_shared)).await;
    network.spawn_send_task(Arc::clone(&server_shared)).await;

    println!("Game started");

    tokio::signal::ctrl_c().await?;

    println!("\nStopping server...");

    Ok(())
}
