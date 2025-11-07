use crate::{
    attack::AttackKind,
    player::Player,
    team::Team,
};

pub fn approach_zero(value: f32, step: f32) -> f32 {
    if value > 0.0 {
        (value - step).max(0.0)
    } else if value < 0.0 {
        (value + step).min(0.0)
    } else {
        0.0
    }
}

pub fn handle_collisions<'a>(
    player: &mut Player,
    teams: impl Iterator<Item = &'a mut Team>,
) {
    let player_rect = player.get_rect();

    for enemy in teams.flat_map(|team| team.players.iter_mut()) {
        if player_rect.overlaps(&enemy.get_rect()) && enemy.invulnerable_timer == 0.0 {
            if player.dashing > 0.0 {
                AttackKind::Dash.attack(enemy, player);
            } else if player.input.slam() {
                AttackKind::Slam.attack(enemy, player);
            }
        }
    }
}
