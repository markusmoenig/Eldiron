//! Independent main-thread foreign hosts. A layer has one presenter, and
//! handles are never recycled, including after every host has been destroyed.
pub(super) struct Owner<T> {
    serial: u64,
    legacy: Option<u64>,
    entries: Vec<(u64, usize, T)>,
}
impl<T> Owner<T> {
    pub const fn new() -> Self {
        Self {
            serial: 0,
            legacy: None,
            entries: Vec::new(),
        }
    }
    pub fn legacy(&self) -> Option<u64> {
        self.legacy
    }
    pub fn contains(&self, id: u64) -> bool {
        self.entries.iter().any(|entry| entry.0 == id)
    }
    pub fn get_mut(&mut self, id: u64) -> Option<&mut T> {
        self.entries
            .iter_mut()
            .find(|entry| entry.0 == id)
            .map(|entry| &mut entry.2)
    }
    pub fn layer_available(&self, layer: usize, except: Option<u64>) -> bool {
        layer != 0
            && !self
                .entries
                .iter()
                .any(|entry| entry.1 == layer && Some(entry.0) != except)
    }
    pub fn install(&mut self, layer: usize, value: T, legacy: bool) -> Option<u64> {
        if !self.layer_available(layer, None) || (legacy && self.legacy.is_some()) {
            return None;
        }
        let id = self.serial.checked_add(1)?;
        self.serial = id;
        self.entries.push((id, layer, value));
        if legacy {
            self.legacy = Some(id);
        }
        Some(id)
    }
    pub fn replace_layer(&mut self, id: u64, layer: usize) -> bool {
        if !self.layer_available(layer, Some(id)) {
            return false;
        }
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.0 == id) else {
            return false;
        };
        entry.1 = layer;
        true
    }
    pub fn take(&mut self, id: u64) -> Option<T> {
        let index = self.entries.iter().position(|entry| entry.0 == id)?;
        if self.legacy == Some(id) {
            self.legacy = None;
        }
        Some(self.entries.remove(index).2)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_hosts_preserve_legacy_routing_layer_exclusivity_and_retirement() {
        let mut hosts = Owner::new();
        let a = hosts.install(10, String::from("A"), false).unwrap();
        let b = hosts.install(20, String::from("B"), false).unwrap();
        assert!(hosts.legacy().is_none());
        assert!(
            hosts
                .install(10, String::from("duplicate"), false)
                .is_none()
        );
        assert!(!hosts.replace_layer(b, 10));
        assert!(hosts.replace_layer(a, 30));
        let legacy = hosts.install(10, String::from("legacy"), true).unwrap();
        assert!(
            hosts
                .install(40, String::from("other legacy"), true)
                .is_none()
        );
        assert_eq!(hosts.legacy(), Some(legacy));
        hosts.get_mut(b).unwrap().push('!');
        assert_eq!(hosts.take(a).unwrap(), "A");
        assert!(!hosts.contains(a));
        assert!(hosts.get_mut(a).is_none());
        assert_eq!(hosts.get_mut(b).unwrap(), "B!");
        assert_eq!(hosts.take(legacy).unwrap(), "legacy");
        assert!(hosts.legacy().is_none());
        assert_eq!(hosts.take(b).unwrap(), "B!");
        assert!(hosts.install(30, String::from("new"), false).unwrap() > legacy);
        assert!(hosts.take(a).is_none());
    }
}
