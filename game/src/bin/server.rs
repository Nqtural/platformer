use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::collections::HashSet;
use tokio::sync::Mutex;
use game_config::read::Config;
use protocol::{
    constants::TEAM_SIZE,
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
use ggez::input::keyboard::KeyCode;

#[tokio::main]
async fn main() -> Result<()> {
    let mut server = ServerState::default();
    let config = Config::get()?;

    // initialize lobby state
    println!("Initializing lobby state (team size: {TEAM_SIZE})...");
    let lobby_state = Arc::new(Mutex::new(Lobby::new()));
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

    if config.render_server() {
        // needed parameter for client input, unused here
        let (input_tx, _) = tokio::sync::mpsc::unbounded_channel::<HashSet<KeyCode>>();

        // setup game window
        let snapshot_history_render = Arc::clone(&server.snapshot_history);
        let render_tick_clone = Arc::clone(&server.render_tick);
        display::game_window::run(input_tx, snapshot_history_render, render_tick_clone, "server")?;
    }

    let server_shared = Arc::new(Mutex::new(server));
    network.spawn_receive_task(Arc::clone(&server_shared)).await;
    network.spawn_send_task(Arc::clone(&server_shared)).await;

    println!("Game started");

    tokio::signal::ctrl_c().await?;

    println!("\nStopping server...");

    Ok(())
}
