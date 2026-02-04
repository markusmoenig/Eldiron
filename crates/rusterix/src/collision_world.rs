use rustc_hash::FxHashMap;
use scenevm::GeoId;
use vek::{Vec2, Vec3};

/// Manages collision data across all chunks in the world
pub struct CollisionWorld {
    /// Collision data indexed by chunk coordinates
    chunks: FxHashMap<Vec2<i32>, ChunkCollision>,
    /// Current state of dynamic geometry (doors open/closed, etc.)
    dynamic_states: FxHashMap<GeoId, DynamicState>,
    /// Chunk size (must match rendering chunk size)
    chunk_size: i32,
}

/// Collision data for a single chunk
#[derive(Clone, Debug)]
pub struct ChunkCollision {
    /// Static blocking volumes (walls, extruded surfaces)
    pub static_volumes: Vec<BlockingVolume>,
    /// Dynamic openings (doors, windows) with their GeoIds
    pub dynamic_openings: Vec<DynamicOpening>,
    /// Walkable floor regions
    pub walkable_floors: Vec<WalkableFloor>,
}

/// A static blocking volume (wall, extruded surface, etc.)
#[derive(Clone, Debug)]
pub struct BlockingVolume {
    pub geo_id: GeoId,
    pub min: Vec3<f32>,
    pub max: Vec3<f32>,
}

/// A dynamic opening that can change state (door, window, etc.)
#[derive(Clone, Debug)]
pub struct DynamicOpening {
    /// GeoId for this opening (used to control state)
    pub geo_id: GeoId,
    /// Optional blocking flag derived from the controlling item (if any)
    pub item_blocking: Option<bool>,
    /// 2D boundary polygon in world space (XZ plane)
    pub boundary_2d: Vec<Vec2<f32>>,
    /// Floor height (Y coordinate)
    pub floor_height: f32,
    /// Ceiling height (Y coordinate)
    pub ceiling_height: f32,
    /// Type of opening
    pub opening_type: OpeningType,
}

/// Type of dynamic opening
#[derive(Clone, Debug, PartialEq)]
pub enum OpeningType {
    Door,    // Can open/close
    Window,  // Always blocking at player height
    Passage, // Always passable
}

/// A walkable floor region
#[derive(Clone, Debug)]
pub struct WalkableFloor {
    pub geo_id: GeoId,
    pub height: f32,
    pub polygon_2d: Vec<Vec2<f32>>,
}

#[derive(Clone, Copy, Debug)]
struct CollisionSegment {
    start: Vec2<f32>,
    end: Vec2<f32>,
}

/// State of a dynamic geometry element
#[derive(Clone, Debug)]
pub struct DynamicState {
    /// Whether this geometry is currently passable
    pub is_passable: bool,
    /// Animation progress (0.0 = closed, 1.0 = open)
    pub animation_progress: f32,
}

impl Default for ChunkCollision {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkCollision {
    pub fn new() -> Self {
        Self {
            static_volumes: Vec::new(),
            dynamic_openings: Vec::new(),
            walkable_floors: Vec::new(),
        }
    }
}

impl Default for CollisionWorld {
    fn default() -> Self {
        Self::new(10) // Default chunk size
    }
}

impl CollisionWorld {
    pub fn new(chunk_size: i32) -> Self {
        Self {
            chunks: FxHashMap::default(),
            dynamic_states: FxHashMap::default(),
            chunk_size,
        }
    }

    /// Add/update collision data for a chunk
    pub fn update_chunk(&mut self, chunk_origin: Vec2<i32>, collision: ChunkCollision) {
        self.chunks.insert(chunk_origin, collision);
    }

    /// Remove collision data for a chunk (when unloading)
    pub fn remove_chunk(&mut self, chunk_origin: Vec2<i32>) {
        self.chunks.remove(&chunk_origin);
    }

    /// Check if a position is blocked (for player movement)
    pub fn is_blocked(&self, position: Vec3<f32>, radius: f32) -> bool {
        // Find which chunk(s) the position overlaps
        let chunk_coords = self.world_to_chunk(Vec2::new(position.x, position.z));

        // Check current chunk and neighbors (player might be on edge)
        for dx in -1..=1 {
            for dy in -1..=1 {
                let check_chunk = Vec2::new(chunk_coords.x + dx, chunk_coords.y + dy);
                if let Some(chunk_collision) = self.chunks.get(&check_chunk) {
                    if self.check_chunk_collision(position, radius, chunk_collision) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Move in the XZ plane with collision sliding, returning the new position and whether a collision occurred.
    pub fn move_distance(
        &self,
        start_pos: Vec3<f32>,
        move_vector: Vec3<f32>,
        radius: f32,
    ) -> (Vec3<f32>, bool) {
        const MAX_ITERATIONS: usize = 3;
        const EPSILON: f32 = 0.001;

        // Shortcut: if the destination lies in a passable opening we can skip further checks
        let target_pos = Vec3::new(
            start_pos.x + move_vector.x,
            start_pos.y + move_vector.y,
            start_pos.z + move_vector.z,
        );
        if self.is_in_passable_opening(target_pos, radius) {
            return (target_pos, false);
        }

        let mut current_pos = start_pos;
        current_pos.y = target_pos.y; // Allow vertical movement directly; collisions are horizontal.

        let mut current_2d = Vec2::new(start_pos.x, start_pos.z);
        let mut remaining = Vec2::new(move_vector.x, move_vector.z);
        let mut blocked = false;
        let mut iterations = 0;

        let segments = self.collect_blocking_segments(start_pos, radius);

        while remaining.magnitude_squared() > EPSILON * EPSILON && iterations < MAX_ITERATIONS {
            iterations += 1;

            // Find earliest collision in remaining path
            let mut closest_collision = None;
            for seg in &segments {
                if let Some((distance, normal)) = self.check_intersection(
                    current_2d,
                    current_2d + remaining,
                    seg.start,
                    seg.end,
                    radius,
                ) {
                    if closest_collision.map_or(true, |(d, _)| distance < d) {
                        closest_collision = Some((distance, normal));
                    }
                }
            }

            match closest_collision {
                Some((distance, normal)) => {
                    blocked = true;

                    // Move up to (just before) collision point
                    let move_dir = remaining.normalized();
                    let allowed_move = move_dir * (distance - EPSILON);
                    current_2d += allowed_move;

                    // Project leftover movement onto the wall's tangent
                    let leftover = remaining.magnitude() - distance;
                    if leftover > EPSILON {
                        let normal_component = normal.dot(remaining) * normal;
                        let slide_vec = remaining - normal_component;
                        let slide_len = slide_vec.magnitude();

                        if slide_len > EPSILON {
                            let friction = 0.5;
                            remaining = slide_vec.normalized() * leftover * friction;
                        } else {
                            remaining = Vec2::zero();
                        }
                    } else {
                        remaining = Vec2::zero();
                    }

                    // Nudge outward from wall to avoid corner clipping
                    current_2d += normal * EPSILON;
                }
                None => {
                    current_2d += remaining;
                    remaining = Vec2::zero();
                }
            }
        }

        // Final "push out" pass so we are never left overlapping geometry
        for seg in &segments {
            if let Some((dist, normal)) =
                self.check_point_against_segment(current_2d, seg.start, seg.end, radius)
            {
                let penetration = radius - dist;
                if penetration > 0.0 {
                    blocked = true;
                    current_2d += normal * (penetration + EPSILON);
                }
            }
        }

        current_pos.x = current_2d.x;
        current_pos.z = current_2d.y;

        (current_pos, blocked)
    }

    fn check_chunk_collision(
        &self,
        position: Vec3<f32>,
        radius: f32,
        chunk: &ChunkCollision,
    ) -> bool {
        // First check if player is inside a passable opening
        // If so, don't check static volumes (openings cut through walls)
        for opening in &chunk.dynamic_openings {
            let is_passable = match opening.opening_type {
                OpeningType::Passage => true, // Always passable
                OpeningType::Window => false, // Always blocking
                OpeningType::Door => {
                    // Check dynamic state - default to passable for doors (open by default)
                    self.dynamic_states
                        .get(&opening.geo_id)
                        .map(|state| state.is_passable)
                        .unwrap_or(true) // Default to passable (open) if no state set
                }
            };

            if is_passable {
                // Check if player is within this passable opening
                if position.y + radius >= opening.floor_height
                    && position.y - radius <= opening.ceiling_height
                {
                    let in_polygon = self.point_in_polygon_2d(
                        Vec2::new(position.x, position.z),
                        &opening.boundary_2d,
                        radius,
                    );
                    if in_polygon {
                        // Player is in a passable opening - don't check static volumes
                        return false;
                    }
                }
            }
        }

        // Check static volumes
        for volume in &chunk.static_volumes {
            if self.collides_with_aabb(position, radius, volume.min, volume.max) {
                return true;
            }
        }

        // Check dynamic openings that are blocking
        for opening in &chunk.dynamic_openings {
            let is_blocking = match opening.opening_type {
                OpeningType::Passage => false, // Always passable
                OpeningType::Window => true,   // Always blocking
                OpeningType::Door => {
                    // Check dynamic state - default to passable for doors
                    self.dynamic_states
                        .get(&opening.geo_id)
                        .map(|state| !state.is_passable)
                        .unwrap_or(false) // Default to passable (not blocking) if no state set
                }
            };

            if is_blocking {
                // Check if player is in height range
                if position.y + radius >= opening.floor_height
                    && position.y - radius <= opening.ceiling_height
                {
                    // Check if player is within 2D polygon
                    if self.point_in_polygon_2d(
                        Vec2::new(position.x, position.z),
                        &opening.boundary_2d,
                        radius,
                    ) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Set the state of a dynamic opening (door open/close)
    pub fn set_opening_state(&mut self, geo_id: GeoId, is_passable: bool) {
        self.dynamic_states
            .entry(geo_id)
            .or_insert(DynamicState {
                is_passable: false,
                animation_progress: 0.0,
            })
            .is_passable = is_passable;

        // Keep per-opening blocking flag in sync for doors that rely on item_blocking.
        let item_blocking = Some(!is_passable);
        for chunk in self.chunks.values_mut() {
            for opening in &mut chunk.dynamic_openings {
                if opening.geo_id == geo_id {
                    opening.item_blocking = item_blocking;
                }
            }
        }
    }

    /// Get the state of a dynamic opening
    pub fn get_opening_state(&self, geo_id: &GeoId) -> Option<&DynamicState> {
        self.dynamic_states.get(geo_id)
    }

    /// Find floor height at position (for gravity/walking)
    pub fn get_floor_height(&self, position: Vec2<f32>) -> Option<f32> {
        let chunk_coords = self.world_to_chunk(position);

        if let Some(chunk_collision) = self.chunks.get(&chunk_coords) {
            for floor in &chunk_collision.walkable_floors {
                if self.point_in_polygon_2d(position, &floor.polygon_2d, 0.0) {
                    return Some(floor.height);
                }
            }
        }

        None
    }

    fn is_in_passable_opening(&self, position: Vec3<f32>, radius: f32) -> bool {
        let chunk_coords = self.world_to_chunk(Vec2::new(position.x, position.z));

        for dx in -1..=1 {
            for dy in -1..=1 {
                let check_chunk = Vec2::new(chunk_coords.x + dx, chunk_coords.y + dy);
                if let Some(chunk_collision) = self.chunks.get(&check_chunk) {
                    for opening in &chunk_collision.dynamic_openings {
                        if self.opening_is_passable(opening)
                            && position.y + radius >= opening.floor_height
                            && position.y - radius <= opening.ceiling_height
                            && self.point_in_polygon_2d(
                                Vec2::new(position.x, position.z),
                                &opening.boundary_2d,
                                radius,
                            )
                        {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    fn collect_blocking_segments(&self, position: Vec3<f32>, radius: f32) -> Vec<CollisionSegment> {
        let chunk_coords = self.world_to_chunk(Vec2::new(position.x, position.z));
        let mut segments = Vec::new();

        for dx in -1..=1 {
            for dy in -1..=1 {
                let check_chunk = Vec2::new(chunk_coords.x + dx, chunk_coords.y + dy);
                if let Some(chunk_collision) = self.chunks.get(&check_chunk) {
                    for volume in &chunk_collision.static_volumes {
                        self.add_volume_segments(volume, &mut segments);
                    }

                    for opening in &chunk_collision.dynamic_openings {
                        if self.opening_is_blocking(opening)
                            && position.y + radius >= opening.floor_height
                            && position.y - radius <= opening.ceiling_height
                        {
                            self.add_polygon_segments(&opening.boundary_2d, &mut segments);
                        }
                    }
                }
            }
        }

        segments
    }

    fn add_volume_segments(&self, volume: &BlockingVolume, segments: &mut Vec<CollisionSegment>) {
        let min = volume.min;
        let max = volume.max;
        let corners = [
            Vec2::new(min.x, min.z),
            Vec2::new(max.x, min.z),
            Vec2::new(max.x, max.z),
            Vec2::new(min.x, max.z),
        ];

        segments.push(CollisionSegment {
            start: corners[0],
            end: corners[1],
        });
        segments.push(CollisionSegment {
            start: corners[1],
            end: corners[2],
        });
        segments.push(CollisionSegment {
            start: corners[2],
            end: corners[3],
        });
        segments.push(CollisionSegment {
            start: corners[3],
            end: corners[0],
        });
    }

    fn add_polygon_segments(&self, polygon: &[Vec2<f32>], segments: &mut Vec<CollisionSegment>) {
        if polygon.len() < 2 {
            return;
        }

        for i in 0..polygon.len() {
            let start = polygon[i];
            let end = polygon[(i + 1) % polygon.len()];
            segments.push(CollisionSegment { start, end });
        }
    }

    fn opening_is_passable(&self, opening: &DynamicOpening) -> bool {
        match opening.opening_type {
            OpeningType::Passage => true,
            OpeningType::Window => false,
            OpeningType::Door => opening.item_blocking.map(|b| !b).unwrap_or(true),
        }
    }

    fn opening_is_blocking(&self, opening: &DynamicOpening) -> bool {
        match opening.opening_type {
            OpeningType::Passage => false,
            OpeningType::Window => true,
            OpeningType::Door => opening.item_blocking.unwrap_or(false),
        }
    }

    fn world_to_chunk(&self, world_pos: Vec2<f32>) -> Vec2<i32> {
        Vec2::new(
            (world_pos.x / self.chunk_size as f32).floor() as i32,
            (world_pos.y / self.chunk_size as f32).floor() as i32,
        )
    }

    fn collides_with_aabb(
        &self,
        pos: Vec3<f32>,
        radius: f32,
        min: Vec3<f32>,
        max: Vec3<f32>,
    ) -> bool {
        // For walls, we only check XZ collision (horizontal plane)
        // The Y check would prevent collision if the player is at a different height
        // We assume the player's height is handled elsewhere (they're always on the ground)

        // Expand AABB by radius in XZ plane only
        let expanded_min_x = min.x - radius;
        let expanded_max_x = max.x + radius;
        let expanded_min_z = min.z - radius;
        let expanded_max_z = max.z + radius;

        // Only check XZ collision for movement blocking
        pos.x >= expanded_min_x
            && pos.x <= expanded_max_x
            && pos.z >= expanded_min_z
            && pos.z <= expanded_max_z
    }

    fn check_intersection(
        &self,
        start: Vec2<f32>,
        end: Vec2<f32>,
        line_start: Vec2<f32>,
        line_end: Vec2<f32>,
        radius: f32,
    ) -> Option<(f32, Vec2<f32>)> {
        let line_vec = line_end - line_start;
        let line_len = line_vec.magnitude();
        if line_len < f32::EPSILON {
            return None;
        }

        let line_dir = line_vec / line_len;
        let normal = Vec2::new(-line_dir.y, line_dir.x);

        let start_dist = (start - line_start).dot(normal);
        let end_dist = (end - line_start).dot(normal);

        if start_dist > radius && end_dist > radius {
            return None;
        }
        if start_dist < -radius && end_dist < -radius {
            return None;
        }

        let dist_diff = end_dist - start_dist;
        let t = if dist_diff.abs() < f32::EPSILON {
            if start_dist.abs() <= radius {
                0.0
            } else {
                return None;
            }
        } else {
            let desired_dist = if start_dist < 0.0 { -radius } else { radius };
            (desired_dist - start_dist) / dist_diff
        };

        if !(0.0..=1.0).contains(&t) {
            return None;
        }

        let intersection = start + (end - start) * t;
        let line_proj = (intersection - line_start).dot(line_dir);

        if line_proj < 0.0 || line_proj > line_len {
            if line_proj < 0.0 {
                return self.check_point_collision(intersection, line_start, radius, start);
            } else {
                return self.check_point_collision(intersection, line_end, radius, start);
            }
        }

        let collision_dist = (intersection - start).magnitude();
        let final_normal = if start_dist < 0.0 { -normal } else { normal };

        Some((collision_dist, final_normal))
    }

    fn check_point_collision(
        &self,
        collision_point: Vec2<f32>,
        corner: Vec2<f32>,
        radius: f32,
        start: Vec2<f32>,
    ) -> Option<(f32, Vec2<f32>)> {
        let to_corner = collision_point - corner;
        let dist_sq = to_corner.magnitude_squared();

        if dist_sq > radius * radius {
            return None;
        }

        let dist_corner = dist_sq.sqrt();
        let normal = if dist_corner > f32::EPSILON {
            to_corner / dist_corner
        } else {
            Vec2::unit_x()
        };

        let collision_dist = (collision_point - start).magnitude();

        Some((collision_dist, normal))
    }

    fn check_point_against_segment(
        &self,
        point: Vec2<f32>,
        seg_start: Vec2<f32>,
        seg_end: Vec2<f32>,
        radius: f32,
    ) -> Option<(f32, Vec2<f32>)> {
        let seg_vec = seg_end - seg_start;
        let seg_len = seg_vec.magnitude();
        if seg_len < f32::EPSILON {
            let d_sq = (point - seg_start).magnitude_squared();
            if d_sq > radius * radius {
                return None;
            }
            let d = d_sq.sqrt();
            let normal = if d > f32::EPSILON {
                (point - seg_start) / d
            } else {
                Vec2::unit_x()
            };
            return Some((d, normal));
        }

        let seg_dir = seg_vec / seg_len;
        let diff = point - seg_start;
        let t = diff.dot(seg_dir).clamp(0.0, seg_len);
        let closest_point = seg_start + seg_dir * t;

        let delta = point - closest_point;
        let dist_sq = delta.magnitude_squared();
        if dist_sq > radius * radius {
            return None;
        }

        let dist = dist_sq.sqrt();
        let normal = if dist > f32::EPSILON {
            delta / dist
        } else {
            Vec2::unit_x()
        };

        Some((dist, normal))
    }

    fn point_in_polygon_2d(&self, point: Vec2<f32>, polygon: &[Vec2<f32>], padding: f32) -> bool {
        if polygon.len() < 3 {
            return false;
        }

        // Simple ray casting algorithm for point-in-polygon test
        let mut inside = false;
        let mut j = polygon.len() - 1;

        for i in 0..polygon.len() {
            let vi = polygon[i];
            let vj = polygon[j];

            // Apply padding (expand polygon)
            let test_point = if padding > 0.0 {
                // For now, simple implementation without padding
                // TODO: Properly expand polygon by padding distance
                point
            } else {
                point
            };

            if ((vi.y > test_point.y) != (vj.y > test_point.y))
                && (test_point.x < (vj.x - vi.x) * (test_point.y - vi.y) / (vj.y - vi.y) + vi.x)
            {
                inside = !inside;
            }

            j = i;
        }

        // If padding is set, also check distance to edges
        if padding > 0.0 && !inside {
            // Check if point is within padding distance of any edge
            for i in 0..polygon.len() {
                let p1 = polygon[i];
                let p2 = polygon[(i + 1) % polygon.len()];

                if self.point_to_segment_distance_2d(point, p1, p2) <= padding {
                    return true;
                }
            }
        }

        inside
    }

    fn point_to_segment_distance_2d(&self, point: Vec2<f32>, a: Vec2<f32>, b: Vec2<f32>) -> f32 {
        let ab = b - a;
        let ap = point - a;
        let ab_len_sq = ab.magnitude_squared();

        if ab_len_sq < 1e-6 {
            return ap.magnitude();
        }

        let t = (ap.dot(ab) / ab_len_sq).clamp(0.0, 1.0);
        let closest = a + ab * t;
        (point - closest).magnitude()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_collision() {
        let world = CollisionWorld::new(10);
        let pos = Vec3::new(5.0, 1.0, 5.0);
        let min = Vec3::new(4.0, 0.0, 4.0);
        let max = Vec3::new(6.0, 2.0, 6.0);

        assert!(world.collides_with_aabb(pos, 0.5, min, max));
        assert!(!world.collides_with_aabb(Vec3::new(10.0, 1.0, 5.0), 0.5, min, max));
    }

    #[test]
    fn test_point_in_polygon() {
        let world = CollisionWorld::new(10);
        let polygon = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];

        assert!(world.point_in_polygon_2d(Vec2::new(5.0, 5.0), &polygon, 0.0));
        assert!(!world.point_in_polygon_2d(Vec2::new(15.0, 5.0), &polygon, 0.0));
    }

    #[test]
    fn test_door_state() {
        let mut world = CollisionWorld::new(10);
        let door_id = GeoId::Sector(1);

        // Door starts closed (blocking)
        world.set_opening_state(door_id, false);
        assert!(!world.get_opening_state(&door_id).unwrap().is_passable);

        // Open door
        world.set_opening_state(door_id, true);
        assert!(world.get_opening_state(&door_id).unwrap().is_passable);
    }

    #[test]
    fn test_move_distance_slides_along_wall() {
        let mut world = CollisionWorld::new(10);
        let mut chunk = ChunkCollision::new();
        chunk.static_volumes.push(BlockingVolume {
            geo_id: GeoId::Sector(1),
            min: Vec3::new(1.0, 0.0, -2.0),
            max: Vec3::new(1.1, 2.0, 2.0),
        });
        world.update_chunk(Vec2::new(0, 0), chunk);

        let start = Vec3::new(0.0, 0.0, 0.0);
        let move_vec = Vec3::new(2.0, 0.0, 1.0);
        let (end, blocked) = world.move_distance(start, move_vec, 0.5);

        assert!(blocked);
        assert!(end.x < 0.6);
        assert!(end.z > 0.7);
    }
}
