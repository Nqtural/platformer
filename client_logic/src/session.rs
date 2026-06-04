use crate::interpolation::SnapshotHistory;
use crate::replay::recorder::ReplayRecorder;
use display::render::RenderState;
use ggez::input::keyboard::KeyCode;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc::UnboundedSender};

pub struct GameSession {
    pub input_tx: UnboundedSender<HashSet<KeyCode>>,
    pub input_state: HashSet<KeyCode>,
    pub snapshot_history: Arc<Mutex<SnapshotHistory>>,
    pub render_tick: Arc<Mutex<f32>>,
    pub render_state: RenderState,
    pub replay_recorder: ReplayRecorder,
}

impl GameSession {
    pub fn new(
        input_tx: UnboundedSender<HashSet<KeyCode>>,
        snapshot_history: Arc<Mutex<SnapshotHistory>>,
        render_tick: Arc<Mutex<f32>>,
        render_state: RenderState,
        replay_recorder: ReplayRecorder,
    ) -> Self {
        Self {
            input_tx,
            input_state: HashSet::new(),
            snapshot_history,
            render_tick,
            render_state,
            replay_recorder,
        }
    }

    pub fn press(&mut self, keycode: KeyCode) {
        self.input_state.insert(keycode);
        let _ = self.input_tx.send(self.input_state.clone());
    }

    pub fn release(&mut self, keycode: &KeyCode) {
        self.input_state.remove(keycode);
        let _ = self.input_tx.send(self.input_state.clone());
    }

    pub fn update_replay(&mut self) {
        let _ = self
            .snapshot_history
            .try_lock()
            .map(|h| h.latest().map(|s| self.replay_recorder.update(s.clone())));
    }

    pub fn save_replay(&self) {
        self.replay_recorder.save();
    }
}
