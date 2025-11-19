use crate::{
    attack::AttackKind,
    team::Team,
    player::Player,
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
    others: impl Iterator<Item = &'a mut Player>,
) {
    for other in others {
        if player.get_rect().overlaps(&other.get_rect()) {
            if player.dashing > 0.0 {
                AttackKind::Dash.attack(other, player);
            } else if player.slamming {
                AttackKind::Slam.attack(other, player);
            }
        }
    }
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
