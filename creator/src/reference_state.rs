//! Browser-only compatibility owner for the remaining reference facade. WebGPU
//! objects are not Send/Sync. They stay in this WASM instance's thread-local
//! owner; owned lock guards retain the source without leaking or unsafe traits.
//! This adapter is temporary: M5 still must retire the facade's caller globals.
use parking_lot::{ArcRwLockReadGuard, ArcRwLockWriteGuard, RawRwLock, RwLock};
use rusterix::Rusterix;
use std::{cell::OnceCell, convert::Infallible, sync::Arc};
thread_local! {
    static STATE: OnceCell<Arc<RwLock<Rusterix>>> = const { OnceCell::new() };
}
pub struct BrowserReference;
pub static RUSTERIX: BrowserReference = BrowserReference;
impl BrowserReference {
    fn source(&self) -> Arc<RwLock<Rusterix>> {
        STATE.with(|state| {
            state
                .get_or_init(|| Arc::new(RwLock::new(Rusterix::new_without_audio())))
                .clone()
        })
    }
    pub fn get(&'static self) -> Option<&'static Self> {
        STATE.with(|state| state.get().map(|_| self))
    }
    pub fn read(&self) -> Result<ArcRwLockReadGuard<RawRwLock, Rusterix>, Infallible> {
        Ok(self.source().read_arc())
    }
    pub fn write(&self) -> Result<ArcRwLockWriteGuard<RawRwLock, Rusterix>, Infallible> {
        Ok(self.source().write_arc())
    }
}
