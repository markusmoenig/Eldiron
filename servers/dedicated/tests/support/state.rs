use eldiron_runtime::persistence::{self, SavedItem, WorldSave};
use std::{collections::BTreeMap, path::Path, time::Duration};
use theframework_core::Uuid;
pub async fn committed(path: &Path, stream: Uuid, sequence: u64) -> WorldSave {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(saved) = persistence::read_snapshot(path) {
            if saved
                .action_streams
                .get(&stream)
                .is_some_and(|j| j.last >= sequence)
            {
                return saved;
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "durable receipt not published"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
pub fn items(saved: &WorldSave) -> BTreeMap<u32, usize> {
    fn visit(i: &SavedItem, out: &mut BTreeMap<u32, usize>) {
        *out.entry(i.id).or_default() += 1;
        if let Some(v) = &i.contents {
            for i in v {
                visit(i, out);
            }
        }
    }
    let mut out = BTreeMap::new();
    for r in &saved.world_state.regions {
        for i in &r.items {
            visit(i, &mut out);
        }
        for e in r.entities.iter().chain(&r.stashed) {
            for i in e
                .inventory
                .iter()
                .flatten()
                .chain(e.equipped.iter().map(|(_, i)| i))
            {
                visit(i, &mut out);
            }
        }
    }
    out
}
pub fn owns(saved: &WorldSave, body: u32, item: u32) -> bool {
    saved
        .world_state
        .regions
        .iter()
        .flat_map(|r| r.entities.iter().chain(&r.stashed))
        .find(|e| e.id == body)
        .unwrap()
        .inventory
        .iter()
        .flatten()
        .any(|i| i.id == item)
}
