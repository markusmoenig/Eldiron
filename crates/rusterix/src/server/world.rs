//! Ephemeral simulation-wide state, shared by one Server's regions only.
//! Document data and wire entity IDs retain their existing formats.
use crate::{Value, ValueContainer};
use std::sync::{
    RwLock,
    atomic::{AtomicU32, Ordering},
};

pub struct SimulationWorld {
    ids: AtomicU32,
    state: RwLock<ValueContainer>,
}
impl Default for SimulationWorld {
    fn default() -> Self {
        Self {
            ids: AtomicU32::new(1),
            state: Default::default(),
        }
    }
}
impl SimulationWorld {
    /// Zero is the existing NO_ID sentinel. Exhaustion must never reuse IDs.
    pub fn next_id(&self) -> u32 {
        self.ids
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .expect("simulation exhausted its runtime entity/item IDs")
    }
    pub fn get(&self, key: &str) -> Option<Value> {
        self.state
            .read()
            .ok()
            .and_then(|state| state.get(key).cloned())
    }
    pub fn set(&self, key: &str, value: Value) {
        if let Ok(mut state) = self.state.write() {
            state.set(key, value);
        }
    }
}
