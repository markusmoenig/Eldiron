use crate::{BBox, Map, ValueContainer};
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Linedef {
    pub id: u32,

    // For editors
    pub creator_id: Uuid,

    pub name: String,
    pub start_vertex: u32,
    pub end_vertex: u32,

    // List of sector IDs this linedef belongs to
    #[serde(default)]
    pub sector_ids: Vec<u32>,

    pub properties: ValueContainer,
}

impl Linedef {
    pub fn new(id: u32, start_vertex: u32, end_vertex: u32) -> Self {
        let properties = ValueContainer::default();
        Self {
            id,
            creator_id: Uuid::new_v4(),
            name: String::new(),
            start_vertex,
            end_vertex,
            sector_ids: Vec::new(),

            properties,
        }
    }

    /// Signed distance from a point to this linedef.
    /// Negative if the point is on the "front" (normal-facing) side.
    pub fn signed_distance(&self, map: &Map, point: Vec2<f32>) -> Option<f32> {
        let v0 = map.get_vertex(self.start_vertex)?;
        let v1 = map.get_vertex(self.end_vertex)?;

        let edge = v1 - v0;
        let to_point = point - v0;

        let t = to_point.dot(edge) / edge.dot(edge);
        let t_clamped = t.clamp(0.0, 1.0);
        let closest = v0 + edge * t_clamped;

        let dist = (point - closest).magnitude();

        // Use perpendicular normal to define the "front" side
        let normal = Vec2::new(-edge.y, edge.x).normalized();
        let side = (point - closest).dot(normal);

        Some(if side < 0.0 { -dist } else { dist })
    }

    /// Calculate the length of the linedef, applying animation states
    pub fn length(&self, map: &Map) -> Option<f32> {
        let start = map.get_vertex(self.start_vertex)?;
        let end = map.get_vertex(self.end_vertex)?;

        Some((end - start).magnitude())
    }

    /// Generate a bounding box for the linedef
    pub fn bounding_box(&self, map: &Map) -> BBox {
        let start = map
            .get_vertex(self.start_vertex)
            .unwrap_or(Vec2::broadcast(0.0));
        let end = map
            .get_vertex(self.end_vertex)
            .unwrap_or(Vec2::broadcast(0.0));

        let min = Vec2::new(start.x.min(end.x), start.y.min(end.y));
        let max = Vec2::new(start.x.max(end.x), start.y.max(end.y));

        BBox::new(min, max)
    }

    /// Returns the vertical span (min_y, max_y) of this linedef in world space (Y-up).
    pub fn y_span_world(&self, map: &Map) -> Option<(f32, f32)> {
        let a = map.get_vertex_3d(self.start_vertex)?;
        let b = map.get_vertex_3d(self.end_vertex)?;
        let min_y = a.y.min(b.y);
        let max_y = a.y.max(b.y);
        Some((min_y, max_y))
    }

    /// Checks whether this linedef intersects a vertical slice centered at `slice_y` with thickness `thickness`.
    pub fn intersects_vertical_slice(&self, map: &Map, slice_y: f32, thickness: f32) -> bool {
        if thickness <= 0.0 {
            return false;
        }
        if let Some((min_y, max_y)) = self.y_span_world(map) {
            let half = thickness * 0.5;
            let y0 = slice_y - half;
            let y1 = slice_y + half;
            max_y >= y0 && min_y <= y1
        } else {
            false
        }
    }
}

impl PartialEq for Linedef {
    fn eq(&self, other: &Self) -> bool {
        (self.start_vertex == other.start_vertex && self.end_vertex == other.end_vertex)
            || (self.start_vertex == other.end_vertex && self.end_vertex == other.start_vertex)
    }
}
impl Eq for Linedef {}

/// A "compiled" version which is used in MapMini for lighting, navigation etc
#[derive(Clone)]
pub struct CompiledLinedef {
    pub start: Vec2<f32>,
    pub end: Vec2<f32>,

    pub wall_width: f32,
    pub wall_height: f32,
}

impl CompiledLinedef {
    pub fn new(start: Vec2<f32>, end: Vec2<f32>, wall_width: f32, wall_height: f32) -> Self {
        Self {
            start,
            end,
            wall_width,
            wall_height,
        }
    }
}
