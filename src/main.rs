mod attack;
mod constants;
mod game_state;
mod input;
mod map;
mod player;
mod team;
mod trail;
mod traits;
mod utils;

use constants::{
    ENABLE_VSYNC,
    TEAM_ONE_COLOR,
    TEAM_ONE_START_POS,
    TEAM_TWO_COLOR,
    TEAM_TWO_START_POS,
    VIRTUAL_WIDTH,
    VIRTUAL_HEIGHT,
};
use game_state::GameState;
use crate::player::Player;
use crate::team::Team;
use ggez::{
    ContextBuilder,
    GameResult,
    event::run,
};

fn main() -> GameResult {
    let (ctx, event_loop) = ContextBuilder::new("game", "me")
        .window_setup(
            ggez::conf::WindowSetup::default()
                .vsync(ENABLE_VSYNC)
                .title("Game")
        )
        .window_mode(
            ggez::conf::WindowMode::default()
                .dimensions(VIRTUAL_WIDTH, VIRTUAL_HEIGHT) 
                .resizable(true)
        )
        .build()?;

    let game_state = GameState::new([
        Team::new(
            vec![Player::new(TEAM_ONE_START_POS, String::from("Player"))],
            TEAM_ONE_COLOR,
            TEAM_ONE_START_POS,
        ),
        Team::new(
            vec![Player::new(TEAM_TWO_START_POS, String::from("Player"))],
            TEAM_TWO_COLOR,
            TEAM_TWO_START_POS,
        ),
    ]);

    run(ctx, event_loop, game_state)
}
