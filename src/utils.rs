use crate::{
    constants::PLAYER_SIZE,
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

pub fn handle_dash_collision(player: &mut Player, enemy: &mut Player) {
    if enemy.dashing > 0.0 {
        enemy.vel[0] = player.vel[0].signum() * 100.0 * enemy.knockback_multiplier;
        enemy.dashing = 0.0;
        enemy.stunned = 0.5;
        enemy.knockback_multiplier += 0.01;

        player.stunned = 0.5;
    } else {
        enemy.vel[0] = player.vel[0] * enemy.knockback_multiplier;
        enemy.stunned = 0.1;
    }

    enemy.vel[1] -= 200.0;
    enemy.slow = 0.5;

    player.vel[1] -= 200.0;
    player.vel[0] = player.vel[0] * -0.5;
    player.dashing = 0.0;
    player.slow = 0.5;
}

pub fn handle_slam_collision(player: &mut Player, enemy: &mut Player) {
    let player_bottom = player.pos[1] + PLAYER_SIZE;
    let enemy_top = enemy.pos[1];

    if player_bottom <= enemy_top + 5.0 && player.vel[1] > 0.0 {
        enemy.vel[1] = player.vel[1] * 1.5 * enemy.knockback_multiplier;
        enemy.stunned = 0.1;
        enemy.slow = 0.5;
        enemy.knockback_multiplier += 0.03;

        player.vel[1] = -100.0;
        player.slow = 0.5;
        player.input.slam = false;
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
                handle_dash_collision(player, enemy);
            } else if player.input.slam {
                handle_slam_collision(player, enemy);
            }
        }
    }
}
