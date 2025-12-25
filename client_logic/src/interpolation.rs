use std::collections::VecDeque;
use simulation::{
    attack::Attack,
    game_state::GameState,
    player::Player,
    team::Team,
};

#[derive(Clone)]
pub struct TimedSnapshot {
    pub server_tick: u64,
    pub snapshot: GameState,
}

const SNAPSHOT_HISTORY_SIZE: usize = 128;

pub struct SnapshotHistory {
    buffer: VecDeque<TimedSnapshot>,
    capacity: usize,
}

impl Default for SnapshotHistory {
    fn default() -> Self {
        Self {
            buffer: VecDeque::new(),
            capacity: SNAPSHOT_HISTORY_SIZE,
        }
    }
}

impl SnapshotHistory {
    pub fn push(&mut self, server_tick: u64, snapshot: GameState) {
        if self.buffer.len() == self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(TimedSnapshot { server_tick, snapshot });
    }

    pub fn get(&self, server_tick: u64) -> Option<&GameState> {
        self.buffer.iter()
            .find(|s| s.server_tick == server_tick)
            .map(|s| &s.snapshot)
    }

    pub fn surrounding(&self, tick: f32) -> (&GameState, &GameState, f32) {
        if self.buffer.is_empty() {
            panic!("no snapshots available");
        }

        let floor = self.buffer.iter()
            .rev()
            .find(|s| s.server_tick as f32 <= tick)
            .unwrap();

        let ceil = self.buffer.iter()
            .find(|s| s.server_tick as f32 >= tick)
            .unwrap_or(floor);

        let alpha = if floor.server_tick == ceil.server_tick {
            0.0
        } else {
            (tick - floor.server_tick as f32) / (ceil.server_tick as f32 - floor.server_tick as f32)
        };

        (&floor.snapshot, &ceil.snapshot, alpha)
    }

    pub fn get_interpolated(&self, render_tick: f32) -> GameState {
        let (a, b, alpha) = self.surrounding(render_tick);
        let mut gs = interpolate(a, b, alpha);

        // overwrite local player with the latest state
        let last = &self.buffer.back().unwrap().snapshot;
        let c_team = last.c_team;
        let c_player = last.c_player;
        gs.teams[c_team].players[c_player] = last.teams[c_team].players[c_player].clone();

        gs
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn interpolate(a: &GameState, b: &GameState, alpha: f32) -> GameState {
    GameState {
        c_team: a.c_team,
        c_player: a.c_player,
        winner: a.winner,
        map: a.map.clone(),
        teams: [
            interpolate_team(&a.teams[0], &b.teams[0], alpha),
            interpolate_team(&a.teams[1], &b.teams[1], alpha),
        ],
    }
}

fn interpolate_team(a: &Team, b: &Team, alpha: f32) -> Team {
    let players = a.players.iter().zip(&b.players).map(|(pa, pb)| {
        interpolate_player(pa, pb, alpha)
    }).collect();

    Team { players }
}

fn interpolate_player(a: &Player, b: &Player, alpha: f32) -> Player {
    Player {
        pos: [
            lerp(a.pos[0], b.pos[0], alpha),
            lerp(a.pos[1], b.pos[1], alpha),
        ],
        vel: a.vel,
        lives: a.lives,
        name: a.name.clone(),
        stunned: lerp(a.stunned, b.stunned, alpha),
        invulnerable_timer: lerp(a.invulnerable_timer, b.invulnerable_timer, alpha),
        parry: lerp(a.parry, b.parry, alpha),
        double_jumps: a.double_jumps,
        combo: a.combo,
        combo_timer: lerp(a.combo_timer, b.combo_timer, alpha),
        knockback_multiplier: a.knockback_multiplier,
        attacks: interpolate_attacks(&a.attacks, &b.attacks, alpha),
        trail_squares: a.trail_squares.clone(),
        can_slam: a.can_slam,
        dash_cooldown: lerp(a.dash_cooldown, b.dash_cooldown, alpha),
        normal_cooldown: lerp(a.normal_cooldown, b.normal_cooldown, alpha),
        light_cooldown: lerp(a.light_cooldown, b.light_cooldown, alpha),
        parry_cooldown: lerp(a.parry_cooldown, b.parry_cooldown, alpha),
        respawn_timer: lerp(a.respawn_timer, b.respawn_timer, alpha),
        trail_timer: lerp(a.trail_timer, b.trail_timer, alpha),
        team_idx: a.team_idx,
        facing: a.facing,
        input: a.input.clone(),
        has_jumped: a.has_jumped,
        start_pos: a.start_pos,
        color: a.color.clone(),
    }
}

fn interpolate_attacks(a: &[Attack], b: &[Attack], alpha: f32) -> Vec<Attack> {
    a.iter().zip(b).map(|(aa, ab)| {
        Attack {
            timer: lerp(aa.timer, ab.timer, alpha),
            ..aa.clone() // everything else copied
        }
    }).collect()
}
