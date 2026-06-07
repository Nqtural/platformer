use simulation::{
    Player, PlayerCombat, PlayerCooldowns, PlayerPhysics, PlayerStatus, attack::Attack,
    game_state::GameState,
};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

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
        self.buffer.push_back(TimedSnapshot {
            server_tick,
            snapshot,
        });
    }

    pub fn get(&self, server_tick: u64) -> Option<&GameState> {
        self.buffer
            .iter()
            .find(|s| s.server_tick == server_tick)
            .map(|s| &s.snapshot)
    }

    pub fn surrounding(&self, tick: f32) -> Option<(&GameState, &GameState, f32)> {
        if self.buffer.is_empty() {
            return None;
        }

        let floor = self
            .buffer
            .iter()
            .rev()
            .find(|s| s.server_tick as f32 <= tick)?;

        let ceil = self
            .buffer
            .iter()
            .find(|s| s.server_tick as f32 >= tick)
            .unwrap_or(floor);

        let alpha = if floor.server_tick == ceil.server_tick {
            0.0
        } else {
            (tick - floor.server_tick as f32) / (ceil.server_tick as f32 - floor.server_tick as f32)
        };

        Some((&floor.snapshot, &ceil.snapshot, alpha))
    }

    pub fn get_interpolated(&self, render_tick: f32, c_player: Uuid) -> Option<GameState> {
        let (a, b, alpha) = self.surrounding(render_tick)?;
        let mut gs = interpolate(a, b, alpha);

        // overwrite local player with the latest state
        let last = &self.buffer.back()?.snapshot;
        gs.players
            .insert(c_player, last.players.get(&c_player).unwrap().clone());

        Some(gs)
    }

    pub fn latest(&self) -> Option<&GameState> {
        self.buffer.back().map(|s| &s.snapshot)
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn interpolate(a: &GameState, b: &GameState, alpha: f32) -> GameState {
    let mut players = HashMap::new();
    for (player_id, player) in &a.players {
        players.insert(
            *player_id,
            interpolate_player(player, b.players.get(player_id).unwrap(), alpha),
        );
    }
    GameState {
        winner: a.winner,
        map: a.map.clone(),
        players,
        teams: a.teams.clone(),
        post_game_timer: a.post_game_timer,
    }
}

fn interpolate_player(a: &Player, b: &Player, alpha: f32) -> Player {
    Player {
        combat: interpolate_combat(&a.combat, &b.combat, alpha),
        cooldowns: interpolate_cooldowns(&a.cooldowns, &b.cooldowns, alpha),
        physics: interpolate_physics(&a.physics, &b.physics, alpha),
        status: interpolate_status(&a.status, &b.status, alpha),
        input: a.input.clone(),
    }
}

fn interpolate_combat(a: &PlayerCombat, b: &PlayerCombat, alpha: f32) -> PlayerCombat {
    PlayerCombat {
        lives: a.lives,
        combo: a.combo,
        combo_timer: lerp(a.combo_timer, b.combo_timer, alpha),
        knockback_multiplier: a.knockback_multiplier,
        attacks: interpolate_attacks(&a.attacks, &b.attacks, alpha),
    }
}

fn interpolate_cooldowns(a: &PlayerCooldowns, b: &PlayerCooldowns, alpha: f32) -> PlayerCooldowns {
    PlayerCooldowns {
        dash: lerp(a.dash, b.dash, alpha),
        normal: lerp(a.normal, b.normal, alpha),
        light: lerp(a.light, b.light, alpha),
        parry: lerp(a.parry, b.parry, alpha),
    }
}

fn interpolate_physics(a: &PlayerPhysics, b: &PlayerPhysics, alpha: f32) -> PlayerPhysics {
    PlayerPhysics {
        start_pos: a.start_pos,
        pos: a.pos.lerp(b.pos, alpha),
        vel: a.vel,
        facing: a.facing,
        team_idx: a.team_idx,
        double_jumps: a.double_jumps,
        has_jumped: a.has_jumped,
    }
}

fn interpolate_status(a: &PlayerStatus, b: &PlayerStatus, alpha: f32) -> PlayerStatus {
    PlayerStatus {
        stunned: lerp(a.stunned, b.stunned, alpha),
        respawn_timer: lerp(a.respawn_timer, b.respawn_timer, alpha),
        invulnerable_timer: lerp(a.invulnerable_timer, b.invulnerable_timer, alpha),
        parry: lerp(a.parry, b.parry, alpha),
        can_slam: a.can_slam,
    }
}

fn interpolate_attacks(a: &[Attack], b: &[Attack], alpha: f32) -> Vec<Attack> {
    a.iter()
        .zip(b)
        .map(|(aa, ab)| {
            Attack {
                timer: lerp(aa.timer, ab.timer, alpha),
                ..aa.clone() // everything else copied
            }
        })
        .collect()
}
