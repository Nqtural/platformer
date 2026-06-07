use crate::interpolation::SnapshotHistory;
use display::render::RenderState;
use ggez::input::keyboard::KeyCode;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc::UnboundedSender};
use uuid::Uuid;

pub struct GameSession {
    pub c_player: Uuid,
    pub input_tx: UnboundedSender<HashSet<KeyCode>>,
    pub input_state: HashSet<KeyCode>,
    pub snapshot_history: Arc<Mutex<SnapshotHistory>>,
    pub render_tick: Arc<Mutex<f32>>,
    pub render_state: RenderState,
}

impl GameSession {
    pub fn new(
        c_player: Uuid,
        input_tx: UnboundedSender<HashSet<KeyCode>>,
        snapshot_history: Arc<Mutex<SnapshotHistory>>,
        render_tick: Arc<Mutex<f32>>,
        render_state: RenderState,
    ) -> Self {
        Self {
            c_player,
            input_tx,
            input_state: HashSet::new(),
            snapshot_history,
            render_tick,
            render_state,
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
}
