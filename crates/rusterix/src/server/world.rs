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
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .expect("simulation exhausted its runtime entity/item IDs")
    }
    pub fn high_water(&self) -> u32 {
        self.ids.load(Ordering::Relaxed)
    }
    pub fn snapshot(&self) -> ValueContainer {
        self.state.read().unwrap().clone()
    }
    pub fn restore(&self, next_id: u32, state: ValueContainer) {
        self.ids.store(next_id, Ordering::Relaxed);
        *self.state.write().unwrap() = state;
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
    /// Serialize stock and destination commitments across all region workers.
    pub(crate) fn with_state<T>(&self, operation: impl FnOnce(&mut ValueContainer) -> T) -> T {
        operation(&mut self.state.write().unwrap())
    }
    pub fn advance_resource_time(&self, elapsed: std::time::Duration) {
        self.with_state(|state| {
            let key = "__world_resource_clock_ms";
            let current = match state.get(key) { Some(Value::Int64(n)) => *n, _ => 0 };
            let elapsed = i64::try_from(elapsed.as_millis()).expect("Resource clock duration overflow");
            state.set(key, Value::Int64(current.checked_add(elapsed).expect("Resource clock exhausted")));
        });
    }

}
