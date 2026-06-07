use crate::{Player, PlayerInput, constants::POST_GAME_TIMER, map::Map, player::HitResult};
use foundation::rect::Rect;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone)]
pub struct GameState {
    pub players: HashMap<Uuid, Player>,
    pub teams: [Vec<Uuid>; 2],
    pub map: Map,
    pub winner: usize,
    pub post_game_timer: f32,
}

impl GameState {
    pub fn new(players: HashMap<Uuid, Player>, teams: [Vec<Uuid>; 2]) -> Self {
        Self {
            players,
            teams,
            map: Map::new(),
            winner: 0,
            post_game_timer: POST_GAME_TIMER,
        }
    }

    pub fn update(&mut self, mut dt: f32) {
        self.check_for_win();

        self.update_post_game_timer(dt);

        if self.winner > 0 {
            dt /= 2.0;
        }

        let player_ids: Vec<_> = self.players.keys().cloned().collect();

        let mut hits = Vec::new();

        for attacker_id in &player_ids {
            let attacker = match self.players.get(attacker_id) {
                Some(p) if p.combat.is_alive() => p,
                _ => continue,
            };

            let attacks = attacker.combat.attacks.clone();

            for attack in attacks {
                let atk_rect = attack.get_rect(attacker.physics.pos);

                for enemy_id in self.get_enemy_ids(attacker_id) {
                    let enemy = match self.players.get(&enemy_id) {
                        Some(e) => e,
                        None => continue,
                    };

                    if atk_rect.overlaps(&enemy.physics.get_rect()) {
                        hits.push((attacker_id, enemy_id, attack.clone()));
                    }
                }
            }
        }

        for (_, target_id, attack) in &hits {
            let attacker_id = attack.owner();

            let (attacker_pos, attacker_vel) = match self.players.get(&attacker_id) {
                Some(attacker) => (attacker.physics.pos, attacker.physics.vel),
                None => continue,
            };

            let result = {
                let target = match self.players.get_mut(target_id) {
                    Some(p) => p,
                    None => continue,
                };

                target.apply_hit(attack, attacker_pos, attacker_vel)
            };

            match result {
                HitResult::Hit => {
                    if let Some(attacker) = self.players.get_mut(&attacker_id) {
                        attacker.apply_hit_effects(attack);
                    }
                }

                HitResult::DashClash => {
                    if let Some(attacker) = self.players.get_mut(&attacker_id) {
                        attacker.apply_dash_clash_effects(attack);
                    }
                }

                HitResult::Parried => {
                    if let Some(attacker) = self.players.get_mut(&attacker_id) {
                        attacker.apply_parry_penalty(attack);
                    }
                }

                HitResult::Ignored => {}
            }
        }

        for player_id in &player_ids {
            let enemy_ids = self.get_enemy_ids(player_id);
            let enemies: Vec<(Rect, bool)> = enemy_ids
                .iter()
                .filter_map(|enemy_id| self.players.get(enemy_id))
                .map(|enemy| (enemy.physics.get_rect(), enemy.status.invulnerable()))
                .collect();

            let player = match self.players.get_mut(player_id) {
                Some(p) if p.combat.is_alive() => p,
                _ => continue,
            };

            player.update(self.map.get_rect(), *player_id, &enemies, dt);
        }
    }

    fn get_enemy_ids(&self, player_id: &Uuid) -> Vec<Uuid> {
        if self.teams[0].contains(player_id) {
            self.teams[1].clone()
        } else {
            self.teams[0].clone()
        }
    }

    fn update_post_game_timer(&mut self, dt: f32) {
        if self.winner != 0 {
            self.post_game_timer -= dt;
        }
    }

    pub fn check_for_win(&mut self) {
        if self.winner != 0 {
            return;
        }

        let alive = [
            self.teams[0]
                .iter()
                .any(|id| self.players[id].combat.is_alive()),
            self.teams[1]
                .iter()
                .any(|id| self.players[id].combat.is_alive()),
        ];

        self.winner = match alive {
            [true, false] => 1,
            [false, true] => 2,
            _ => 0,
        };
    }

    pub fn apply_input(&mut self, player: &Uuid, input: PlayerInput) {
        self.players.get_mut(player).unwrap().input = input;
    }

    pub fn is_game_over(&self) -> bool {
        self.post_game_timer <= 0.0
    }
}
