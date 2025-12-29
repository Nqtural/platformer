use anyhow::Result;
use ggez::input::keyboard::KeyCode;
use std::collections::HashSet;
use std::sync::Arc;
use game_config::read::Config;
use client_logic::{
    ClientState,
    NetworkClient,
};

#[tokio::main]
async fn main() -> Result<()> {
    // get configuration
    let config = Config::get()?;
    let network = NetworkClient::new(
        config.clientip(),
        config.clientport(),
        config.serverip(),
        config.serverport(),
    ).await?;

    let (team_id, player_id, init_teams) = network.handshake(config.playername()).await?;

    let client = Arc::new(ClientState::new(
        team_id,
        player_id,
        init_teams,
        config.trail_delay(),
        config.trail_opacity(),
        config.trail_lifetime(),
    )?);

    network.spawn_receive_task(Arc::clone(&client));
    network.spawn_send_task(Arc::clone(&client));

    // input
    let current_input_write = Arc::clone(&client.current_input);
    let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<HashSet<KeyCode>>();
    tokio::spawn(async move {
        while let Some(input) = input_rx.recv().await {
            let mut current = current_input_write.lock().await;
            *current = input;
        }
    });

    // setup game window
    let history_clone_render = Arc::clone(&client.snapshot_history);
    let render_tick_clone = Arc::clone(&client.render_tick);
    display::game_window::run(
        input_tx,
        history_clone_render,
        render_tick_clone,
        Arc::clone(&client),
        "client",
        config.vsync(),
    )?;
    
    Ok(())
}
