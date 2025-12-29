use serde::{
    Serialize,
    Deserialize,
};
use foundation::color::Color;
use simulation::Player;
use simulation::team::Team;

#[derive(Serialize, Deserialize, Clone)]
pub struct InitTeamData {
    pub color: Color,
    pub player_names: Vec<String>,
    pub start_position: [f32; 2],
    pub index: usize,
}

impl InitTeamData {
    pub fn new(
        color: Color,
        start_position: [f32; 2],
        index: usize,
        team_size: usize
    ) -> Self {
        Self {
            color,
            player_names: vec![String::new(); team_size],
            start_position,
            index,
        }
    }
}

#[must_use]
pub fn from_init(
    init: InitTeamData,
    trail_delay: f32,
    trail_opacity: f32,
    trail_lifetime: f32,
) -> Team {
    let mut players = Vec::new();

    for name in init.player_names.iter() {
        players.push(Player::new(
            init.start_position,
            name.clone(),
            init.color.clone(),
            init.index,
            trail_delay,
            trail_opacity,
            trail_lifetime,
        ));
    }

    Team::new(players)
}
