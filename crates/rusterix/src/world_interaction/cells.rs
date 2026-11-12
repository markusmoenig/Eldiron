//! Immutable flat-view targets. Capture positions/order once for the frame;
//! input must not re-scan a newer simulation map beneath an older image.
use super::*;
use rustc_hash::FxHashMap;
#[derive(Clone, Copy)]
struct Target {
    id: u32,
    position: Vec2<f32>,
}
#[derive(Default)]
struct Cell {
    living: Option<Target>,
    item: Option<Target>,
    dead: Option<Target>,
}
pub struct CellTargets {
    region: Uuid,
    leader: Vec2<f32>,
    cells: FxHashMap<(u32, u32), Cell>,
}
fn key(position: Vec2<f32>) -> Option<(u32, u32)> {
    if !position.iter().all(|v| v.is_finite()) {
        return None;
    }
    // Float equality identifies -0 with +0; raw bit keys must do the same.
    let position = position.map(|v| {
        let v = v.floor();
        if v == 0. { 0 } else { v.to_bits() }
    });
    Some((position.x, position.y))
}
impl CellTargets {
    /// Native flat producers use the runtime `visible` flag (default false).
    /// Reference click policy remains available separately for compatibility.
    pub fn capture_visible(map: &Map) -> Self {
        let mut result = Self {
            region: map.id,
            leader: leader_position(map),
            cells: FxHashMap::default(),
        };
        for entity in &map.entities {
            if !entity.attributes.get_bool_default("visible", false) {
                continue;
            }
            let position = entity.get_pos_xz();
            let Some(key) = key(position) else {
                continue;
            };
            let cell = result.cells.entry(key).or_default();
            let target = Target {
                id: entity.id,
                position,
            };
            let slot = if entity.attributes.get_str("mode") == Some("dead") {
                &mut cell.dead
            } else {
                &mut cell.living
            };
            slot.get_or_insert(target);
        }
        for item in &map.items {
            if !item.attributes.get_bool_default("visible", false) {
                continue;
            }
            let position = item.get_pos_xz();
            let Some(key) = key(position) else {
                continue;
            };
            result.cells.entry(key).or_default().item = Some(Target {
                id: item.id,
                position,
            });
        }
        result
    }
    pub fn hit(&self, ground: Vec2<f32>, prefer_item: bool) -> Option<WorldHit> {
        let key = key(ground)?;
        let mut hit = WorldHit {
            region: self.region,
            cell: Some(ground.map(f32::floor)),
            world: Some(Vec3::new(ground.x, 0., ground.y)),
            ..Default::default()
        };
        let Some(cell) = self.cells.get(&key) else {
            return Some(hit);
        };
        if let Some(item) = cell.item.filter(|_| prefer_item || cell.living.is_none()) {
            hit.item = Some(item.id);
            hit.distance = self.leader.distance(item.position);
        } else if let Some(entity) = cell.living.or(cell.dead) {
            hit.entity = Some(entity.id);
            hit.distance = self.leader.distance(entity.position);
        }
        Some(hit)
    }
}
