//! Region-local world X/Z cells, independent of renderer/collision chunks.
//! These are trusted host subscriptions, never a client authorization API.
use super::ReplicationError;
use rusterix::{Value, ValueContainer};
use std::collections::{BTreeSet, HashMap, HashSet};
use theframework_core::Uuid;

pub const CELL_SIZE: f32 = 32.;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpatialPolicy {
    pub radius: f32,
    pub margin: f32,
}
impl Default for SpatialPolicy {
    fn default() -> Self {
        Self {
            radius: 64.,
            margin: 16.,
        }
    }
}
impl SpatialPolicy {
    pub fn valid(self) -> bool {
        self.radius.is_finite()
            && (4. ..=512.).contains(&self.radius)
            && self.margin.is_finite()
            && (0. ..=128.).contains(&self.margin)
    }
    /// floor, rather than truncation, preserves negative world coordinates.
    pub fn cell(self, position: [f32; 2]) -> [i32; 2] {
        position.map(|v| (v / CELL_SIZE).floor() as i32)
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpatialSubscriptions {
    pub policy: SpatialPolicy,
    /// Trusted full-region observers (spectators, game-specific remote views).
    pub regions: BTreeSet<Uuid>,
    /// Trusted target pins; ordinary public/private projection still applies.
    pub entities: BTreeSet<u32>,
    /// Static/collision preloads only; grants no live region subscription.
    pub prefetch: BTreeSet<Uuid>,
}
impl SpatialSubscriptions {
    pub(crate) fn validate(&self) -> Result<(), ReplicationError> {
        if !self.policy.valid()
            || self.regions.len() > 8
            || self.entities.len() > 64
            || self.prefetch.len() > 4
            || self.regions.iter().chain(&self.prefetch).any(Uuid::is_nil)
            || self.entities.contains(&0)
        {
            return Err(ReplicationError::InvalidFrame(
                "Invalid spatial subscription".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum Object {
    Entity(u32),
    Item(u32),
    Prop(Uuid),
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Bounds {
    pub min: [f32; 2],
    pub max: [f32; 2],
}
impl Bounds {
    pub fn point(p: [f32; 2]) -> Self {
        Self { min: p, max: p }
    }
    pub fn around(p: [f32; 2], radius: f32) -> Self {
        Self {
            min: p.map(|v| v - radius),
            max: p.map(|v| v + radius),
        }
    }
    pub fn intersects(self, other: Self) -> bool {
        (0..2).all(|i| self.min[i] <= other.max[i] && self.max[i] >= other.min[i])
    }
    pub fn union(self, other: Self) -> Self {
        Self {
            min: std::array::from_fn(|i| self.min[i].min(other.min[i])),
            max: std::array::from_fn(|i| self.max[i].max(other.max[i])),
        }
    }
    pub fn valid(self) -> bool {
        (0..2).all(|i| {
            self.min[i].is_finite() && self.max[i].is_finite() && self.min[i] <= self.max[i]
        })
    }
}
pub(crate) fn actor_bounds(p: [f32; 2], attributes: &ValueContainer) -> Bounds {
    let mut radius = attributes.get_float_default("radius", 0.5).abs().max(1.0);
    for key in ["size", "size_2d", "source_size"] {
        let size = match attributes.get(key) {
            Some(Value::Vec2(v)) => v[0].abs().max(v[1].abs()),
            Some(Value::Vec3(v)) => v[0].abs().max(v[2].abs()),
            Some(Value::Float(v)) => v.abs(),
            Some(Value::Int(v)) => (*v as f32).abs(),
            _ => 0.,
        };
        radius = radius.max(size * 0.5);
    }
    Bounds::around(p, radius)
}
/// Large extents use a bounded overflow list instead of allocating millions of
/// cells. Queries still perform exact AABB tests and return each object once.
#[derive(Default)]
pub(crate) struct SpatialIndex {
    cell_size: f32,
    cells: HashMap<[i32; 2], Vec<Object>>,
    bounds: HashMap<Object, Bounds>,
    oversized: Vec<Object>,
}
impl SpatialIndex {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            ..Default::default()
        }
    }
    fn cell(&self, p: [f32; 2]) -> [i32; 2] {
        p.map(|v| (v / self.cell_size).floor() as i32)
    }
    pub fn insert(&mut self, id: Object, bounds: Bounds) {
        self.bounds.insert(id, bounds);
        let min = self.cell(bounds.min);
        let max = self.cell(bounds.max);
        let width = i64::from(max[0]) - i64::from(min[0]) + 1;
        let height = i64::from(max[1]) - i64::from(min[1]) + 1;
        if !bounds.valid() || width <= 0 || height <= 0 || width.saturating_mul(height) > 256 {
            self.oversized.push(id);
            return;
        }
        for x in min[0]..=max[0] {
            for z in min[1]..=max[1] {
                self.cells.entry([x, z]).or_default().push(id);
            }
        }
    }
    pub fn query(&self, window: Bounds) -> HashSet<Object> {
        let min = self.cell(window.min);
        let max = self.cell(window.max);
        let mut found: HashSet<_> = self.oversized.iter().copied().collect();
        for x in min[0]..=max[0] {
            for z in min[1]..=max[1] {
                if let Some(ids) = self.cells.get(&[x, z]) {
                    found.extend(ids);
                }
            }
        }
        found.retain(|id| self.intersects(*id, window));
        found
    }
    pub fn intersects(&self, id: Object, window: Bounds) -> bool {
        self.bounds
            .get(&id)
            .is_some_and(|b| !b.valid() || b.intersects(window))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cells_cover_negative_edges_large_extents_and_deduplicate() {
        let policy = SpatialPolicy::default();
        assert_eq!(policy.cell([-0.1, -32.1]), [-1, -2]);
        let mut index = SpatialIndex::new(32.);
        index.insert(Object::Entity(1), Bounds::around([-32., 0.], 0.5));
        index.insert(
            Object::Item(2),
            Bounds {
                min: [-1_000_000., -1_000_000.],
                max: [1_000_000., 1_000_000.],
            },
        );
        index.insert(Object::Entity(3), Bounds::point([100., 100.]));
        assert_eq!(index.query(Bounds::around([-32., 0.], 1.)).len(), 2);
        assert_eq!(index.oversized.len(), 1);
        assert_eq!(
            index.query(Bounds::around([700., 700.], 1.)),
            HashSet::from([Object::Item(2)])
        );
    }
    #[test]
    fn policies_are_bounded_and_finite() {
        let mut config = SpatialSubscriptions::default();
        config.policy.radius = 0.;
        assert!(config.validate().is_err());
        config.policy = Default::default();
        config.policy.margin = f32::INFINITY;
        assert!(config.validate().is_err());
    }
}
