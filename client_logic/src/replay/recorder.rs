use super::core::Replay;
use crate::{interpolation::TimedSnapshot, replay::constants::REPLAY_DIRECTORY};
use bincode::serde::encode_to_vec;
use chrono::Local;
use protocol::net_team::InitTeamData;
use std::{fs, path::Path};

pub struct ReplayRecorder {
    last_tick: u64,
    replay: Replay,
}

impl ReplayRecorder {
    pub fn new(teams: [InitTeamData; 2]) -> Self {
        Self {
            last_tick: 0,
            replay: Replay::new(teams),
        }
    }

    pub fn update(&mut self, timed_snapshot: TimedSnapshot) {
        if self.last_tick >= timed_snapshot.server_tick {
            return;
        }

        for (i, team) in timed_snapshot.snapshot.teams.iter().enumerate() {
            for player in &team.players {
                self.replay.store(i, player.get_input().clone());
            }
        }

        self.last_tick = timed_snapshot.server_tick;
    }

    pub fn save(&self) {
        let bytes = match encode_to_vec(&self.replay, bincode::config::standard()) {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("Failed to encode replay: {e}");
                return;
            }
        };

        let path = format!(
            "{}{}.prp",
            REPLAY_DIRECTORY,
            Local::now().format("%Y-%m-%d-%H-%M-%S")
        );

        if let Some(parent) = Path::new(&path).parent() {
            let _ = fs::create_dir_all(parent)
                .map_err(|e| eprintln!("Failed to create directory {:?}: {e}", parent));
        }

        let _ = fs::write(path, bytes).map_err(|e| eprintln!("Failed to save replay: {e}"));
    }
}
