//! Generational root ownership for the foreign Creator facade. Until the
//! remaining editor globals retire, a second active Creator is rejected.
pub(super) struct Owner<T> {
    serial: u64,
    active: Option<(u64, T)>,
}
impl<T> Owner<T> {
    pub const fn new() -> Self {
        Self {
            serial: 0,
            active: None,
        }
    }
    pub fn is_some(&self) -> bool {
        self.active.is_some()
    }
    pub fn id(&self) -> u64 {
        self.active.as_ref().map_or(0, |entry| entry.0)
    }
    pub fn as_mut(&mut self) -> Option<&mut T> {
        self.active.as_mut().map(|entry| &mut entry.1)
    }
    pub fn install(&mut self, value: T) -> Option<u64> {
        if self.active.is_some() {
            return None;
        }
        let id = self.serial.checked_add(1)?;
        self.serial = id;
        self.active = Some((id, value));
        Some(id)
    }
    pub fn take(&mut self) -> Option<T> {
        self.active.take().map(|entry| entry.1)
    }
}
