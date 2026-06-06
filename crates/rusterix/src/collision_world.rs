use pathfinding::prelude::astar;
use rustc_hash::FxHashMap;
use scenevm::GeoId;
use std::cell::RefCell;
use vek::{Vec2, Vec3};

const PASSABLE_OPENING_EXTRA_TOLERANCE: f32 = 0.12;

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
    /// Exact static wall barriers for thin/diagonal walls
    pub static_barriers: Vec<StaticBarrier>,
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

#[derive(Clone, Debug)]
pub struct StaticBarrier {
    pub geo_id: GeoId,
    pub start: Vec2<f32>,
    pub end: Vec2<f32>,
    pub min_y: f32,
    pub max_y: f32,
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
    pub plane_normal: Vec3<f32>,
    pub plane_d: f32,
}

#[derive(Clone, Copy, Debug)]
struct CollisionSegment {
    geo_id: GeoId,
    start: Vec2<f32>,
    end: Vec2<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CollisionProbeStepKind {
    Walk,
    StepUp,
    StepDown,
    Contact,
    Blocked,
    NoFloor,
}

#[derive(Clone, Copy, Debug)]
pub struct CollisionProbeBlocker {
    pub geo_id: GeoId,
    pub start: Vec2<f32>,
    pub end: Vec2<f32>,
}

#[derive(Clone, Copy, Debug)]
pub struct CollisionProbeFloorSample {
    pub position: Vec2<f32>,
    pub height: f32,
    pub reachable: bool,
}

#[derive(Clone, Debug)]
pub struct CollisionProbeStep {
    pub from: Vec3<f32>,
    pub to: Vec3<f32>,
    pub target: Vec2<f32>,
    pub current_floor: Option<f32>,
    pub next_floor: Option<f32>,
    pub kind: CollisionProbeStepKind,
    pub blocker: Option<CollisionProbeBlocker>,
    pub support_samples: Vec<CollisionProbeFloorSample>,
}

#[derive(Clone, Debug, Default)]
pub struct CollisionProbeResult {
    pub start: Vec3<f32>,
    pub target: Vec2<f32>,
    pub radius: f32,
    pub max_step_height: f32,
    pub arrived: bool,
    pub goto_path_found: bool,
    pub goto_path_unstable: bool,
    pub goto_path: Vec<Vec3<f32>>,
    pub steps: Vec<CollisionProbeStep>,
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
            static_barriers: Vec::new(),
            dynamic_openings: Vec::new(),
            walkable_floors: Vec::new(),
        }
    }

    pub fn extend(&mut self, other: ChunkCollision) {
        self.static_volumes.extend(other.static_volumes);
        self.static_barriers.extend(other.static_barriers);
        self.dynamic_openings.extend(other.dynamic_openings);
        self.walkable_floors.extend(other.walkable_floors);
    }
}

impl WalkableFloor {
    pub fn flat(geo_id: GeoId, height: f32, polygon_2d: Vec<Vec2<f32>>) -> Self {
        Self {
            geo_id,
            height,
            polygon_2d,
            plane_normal: Vec3::unit_y(),
            plane_d: height,
        }
    }

    pub fn planar(
        geo_id: GeoId,
        height: f32,
        polygon_2d: Vec<Vec2<f32>>,
        plane_normal: Vec3<f32>,
        plane_point: Vec3<f32>,
    ) -> Self {
        let normal = plane_normal.try_normalized().unwrap_or_else(Vec3::unit_y);
        Self {
            geo_id,
            height,
            polygon_2d,
            plane_normal: normal,
            plane_d: normal.dot(plane_point),
        }
    }

    pub fn height_at(&self, position: Vec2<f32>) -> f32 {
        if self.plane_normal.y.abs() <= 1e-5 {
            return self.height;
        }
        (self.plane_d - self.plane_normal.x * position.x - self.plane_normal.z * position.y)
            / self.plane_normal.y
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

    pub fn has_collision_data(&self) -> bool {
        !self.chunks.is_empty()
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
        self.move_distance_with_step_clearance(start_pos, move_vector, radius, None)
    }

    fn move_distance_with_step_clearance(
        &self,
        start_pos: Vec3<f32>,
        move_vector: Vec3<f32>,
        radius: f32,
        step_clearance_y: Option<f32>,
    ) -> (Vec3<f32>, bool) {
        let (pos, blocked, _) = self.move_distance_with_step_clearance_trace(
            start_pos,
            move_vector,
            radius,
            step_clearance_y,
        );
        (pos, blocked)
    }

    fn move_distance_with_step_clearance_trace(
        &self,
        start_pos: Vec3<f32>,
        move_vector: Vec3<f32>,
        radius: f32,
        step_clearance_y: Option<f32>,
    ) -> (Vec3<f32>, bool, Option<CollisionProbeBlocker>) {
        const MAX_ITERATIONS: usize = 3;
        const EPSILON: f32 = 0.001;

        let mut current_pos = start_pos;
        current_pos.y = start_pos.y + move_vector.y; // Allow vertical movement directly; collisions are horizontal.

        let mut current_2d = Vec2::new(start_pos.x, start_pos.z);
        let mut remaining = Vec2::new(move_vector.x, move_vector.z);
        let mut blocked = false;
        let mut iterations = 0;

        let segments = self.collect_blocking_segments(start_pos, radius, step_clearance_y);
        let mut blocker = None;

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
                    if closest_collision.map_or(true, |(d, _, _)| distance < d) {
                        closest_collision = Some((distance, normal, seg.geo_id));
                    }
                }
            }

            match closest_collision {
                Some((distance, normal, geo_id)) => {
                    blocked = true;
                    blocker = segments.iter().find(|seg| seg.geo_id == geo_id).map(|seg| {
                        CollisionProbeBlocker {
                            geo_id: seg.geo_id,
                            start: seg.start,
                            end: seg.end,
                        }
                    });

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
                    blocker.get_or_insert(CollisionProbeBlocker {
                        geo_id: seg.geo_id,
                        start: seg.start,
                        end: seg.end,
                    });
                    current_2d += normal * (penetration + EPSILON);
                }
            }
        }

        current_pos.x = current_2d.x;
        current_pos.z = current_2d.y;

        (current_pos, blocked, blocker)
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
                    let in_polygon = self.footprint_intersects_polygon_2d(
                        Vec2::new(position.x, position.z),
                        &opening.boundary_2d,
                        radius + PASSABLE_OPENING_EXTRA_TOLERANCE,
                        radius,
                    );
                    if in_polygon {
                        // Player is in a passable opening - don't check static volumes
                        return false;
                    }
                }
            }
        }

        for barrier in &chunk.static_barriers {
            if self.barrier_blocks_horizontal_motion(position, barrier)
                && self.point_to_segment_distance_2d(
                    Vec2::new(position.x, position.z),
                    barrier.start,
                    barrier.end,
                ) <= radius
            {
                return true;
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
        let mut best_height: Option<f32> = None;

        for chunk_collision in self.neighbor_chunks(chunk_coords) {
            for floor in &chunk_collision.walkable_floors {
                if self.point_in_polygon_2d(position, &floor.polygon_2d, 0.0) {
                    let height = floor.height_at(position);
                    best_height = Some(match best_height {
                        Some(curr) => curr.max(height),
                        None => height,
                    });
                }
            }
        }

        best_height
    }

    /// Find floor height at position, preferring floors at or below `reference_y`.
    /// This avoids snapping actors to ceilings/upper decks in overlapping XZ areas.
    /// If no floor exists at/below reference, falls back to the nearest above.
    pub fn get_floor_height_nearest(&self, position: Vec2<f32>, reference_y: f32) -> Option<f32> {
        let chunk_coords = self.world_to_chunk(position);
        const FLOOR_EPS: f32 = 0.05;
        let mut best_below: Option<f32> = None;
        let mut best_above: Option<f32> = None;
        let mut best_above_dist = f32::INFINITY;

        for chunk_collision in self.neighbor_chunks(chunk_coords) {
            for floor in &chunk_collision.walkable_floors {
                if self.point_in_polygon_2d(position, &floor.polygon_2d, 0.0) {
                    let height = floor.height_at(position);
                    if height <= reference_y + FLOOR_EPS {
                        best_below = Some(match best_below {
                            Some(curr) => curr.max(height),
                            None => height,
                        });
                    } else {
                        let d = height - reference_y;
                        if d < best_above_dist {
                            best_above_dist = d;
                            best_above = Some(height);
                        } else if (d - best_above_dist).abs() < 1e-4
                            && height < best_above.unwrap_or(f32::INFINITY)
                        {
                            // Stable tie-breaker for above floors: prefer lower one.
                            best_above = Some(height);
                        }
                    }
                }
            }
        }

        best_below.or(best_above)
    }

    /// Find the best floor reachable from `reference_y` within `max_step_height`.
    /// Prefer the closest higher floor when climbing, otherwise the closest lower floor.
    pub fn get_floor_height_reachable(
        &self,
        position: Vec2<f32>,
        reference_y: f32,
        max_step_height: f32,
    ) -> Option<f32> {
        let chunk_coords = self.world_to_chunk(position);
        const SAME_LEVEL_EPS: f32 = 0.05;
        let mut best_up: Option<f32> = None;
        let mut best_up_delta = f32::INFINITY;
        let mut best_same: Option<f32> = None;
        let mut best_down: Option<f32> = None;
        let mut best_down_delta = f32::INFINITY;

        for chunk_collision in self.neighbor_chunks(chunk_coords) {
            for floor in &chunk_collision.walkable_floors {
                if !self.point_in_polygon_2d(position, &floor.polygon_2d, 0.0) {
                    continue;
                }
                let height = floor.height_at(position);
                let delta = height - reference_y;
                if delta > SAME_LEVEL_EPS && delta <= max_step_height + 1e-3 {
                    if delta < best_up_delta {
                        best_up_delta = delta;
                        best_up = Some(height);
                    } else if (delta - best_up_delta).abs() < 1e-4
                        && height > best_up.unwrap_or(f32::NEG_INFINITY)
                    {
                        best_up = Some(height);
                    }
                } else if delta >= -SAME_LEVEL_EPS && delta <= SAME_LEVEL_EPS {
                    best_same = Some(height);
                } else if delta < -SAME_LEVEL_EPS && delta.abs() <= max_step_height + 1e-3 {
                    let down_delta = delta.abs();
                    if down_delta < best_down_delta {
                        best_down_delta = down_delta;
                        best_down = Some(height);
                    } else if (down_delta - best_down_delta).abs() < 1e-4
                        && height > best_down.unwrap_or(f32::NEG_INFINITY)
                    {
                        best_down = Some(height);
                    }
                }
            }
        }

        best_up.or(best_same).or(best_down)
    }

    fn get_center_floor_height_down(
        &self,
        position: Vec2<f32>,
        reference_y: f32,
        max_step_height: f32,
    ) -> Option<f32> {
        const SAME_LEVEL_EPS: f32 = 0.05;
        let mut best_down: Option<f32> = None;
        let mut best_down_delta = f32::INFINITY;

        let chunk_coords = self.world_to_chunk(position);
        for chunk_collision in self.neighbor_chunks(chunk_coords) {
            for floor in &chunk_collision.walkable_floors {
                if !self.point_in_polygon_2d(position, &floor.polygon_2d, 0.0) {
                    continue;
                }
                let height = floor.height_at(position);
                let delta = reference_y - height;
                if delta > SAME_LEVEL_EPS
                    && delta <= max_step_height + 1e-3
                    && delta < best_down_delta
                {
                    best_down_delta = delta;
                    best_down = Some(height);
                }
            }
        }

        best_down
    }

    /// 3D-aware movement on walkable floors.
    /// Returns `None` if no valid floor/path context exists so caller can fall back.
    pub fn move_towards_on_floors(
        &self,
        from: Vec2<f32>,
        to: Vec2<f32>,
        speed: f32,
        radius: f32,
        max_step_height: f32,
        reference_y: f32,
    ) -> Option<(Vec3<f32>, bool)> {
        self.move_towards_on_floors_inner(
            from,
            to,
            None,
            speed,
            radius,
            max_step_height,
            reference_y,
        )
    }

    pub fn move_towards_on_floors_to_height(
        &self,
        from: Vec2<f32>,
        to: Vec2<f32>,
        target_y: f32,
        speed: f32,
        radius: f32,
        max_step_height: f32,
        reference_y: f32,
    ) -> Option<(Vec3<f32>, bool)> {
        self.move_towards_on_floors_inner(
            from,
            to,
            Some(target_y),
            speed,
            radius,
            max_step_height,
            reference_y,
        )
    }

    fn move_towards_on_floors_inner(
        &self,
        from: Vec2<f32>,
        to: Vec2<f32>,
        target_y: Option<f32>,
        speed: f32,
        radius: f32,
        max_step_height: f32,
        reference_y: f32,
    ) -> Option<(Vec3<f32>, bool)> {
        let base_height = self
            .sample_reachable_floor_height(from, radius * 0.5, reference_y, max_step_height)
            .or_else(|| self.get_floor_height_reachable(from, reference_y, max_step_height))
            .or_else(|| {
                self.sample_reachable_floor_height(to, radius * 0.5, reference_y, max_step_height)
            })
            .or_else(|| self.get_floor_height_reachable(to, reference_y, max_step_height))?;

        let target_height = target_y.or_else(|| {
            self.get_floor_height_nearest(to, base_height)
                .or_else(|| self.sample_floor_height(to, radius * 0.5))
        });

        let distance_before = (to - from).magnitude();
        if distance_before <= 0.05 {
            let arrived = target_height
                .map(|height| (base_height - height).abs() <= max_step_height + 0.05)
                .unwrap_or(true);
            if arrived {
                return Some((Vec3::new(from.x, base_height, from.y), true));
            }
        }

        let target_on_different_level = target_height
            .map(|height| (height - base_height).abs() > max_step_height + 0.05)
            .unwrap_or(false);

        if !target_on_different_level {
            let (direct_position, direct_arrived) = self.step_towards_point(
                from,
                to,
                speed,
                radius,
                base_height,
                max_step_height,
                0.05,
                false,
            );
            let direct_2d = Vec2::new(direct_position.x, direct_position.z);
            let moved = (direct_2d - from).magnitude();
            let distance_after = (to - direct_2d).magnitude();
            let requested_step = speed.min(distance_before);
            let useful_direct =
                moved >= requested_step * 0.5 && distance_after < distance_before - 0.001;
            let height_arrived = target_height
                .map(|height| (direct_position.y - height).abs() <= max_step_height + 0.05)
                .unwrap_or(true);
            if useful_direct || direct_arrived || distance_after <= 0.05 {
                return Some((direct_position, distance_after <= 0.05 && height_arrived));
            }
        }

        let waypoint = if target_on_different_level {
            self.navgrid_next_waypoint_with_target_height(
                from,
                to,
                radius,
                base_height,
                max_step_height,
                0.05,
                target_height,
            )?
        } else if self.segment_is_clear(
            Vec3::new(from.x, base_height, from.y),
            Vec3::new(to.x, base_height, to.y),
            radius,
        ) {
            to
        } else {
            self.navgrid_next_waypoint_with_target_height(
                from,
                to,
                radius,
                base_height,
                max_step_height,
                0.05,
                target_height,
            )?
        };

        let waypoint_height = self
            .get_floor_height_nearest(waypoint, base_height)
            .or_else(|| self.sample_floor_height(waypoint, radius * 0.5));
        let prefer_center_down = waypoint_height
            .map(|height| height < base_height - 0.05)
            .unwrap_or(false);

        let (new_position, _) = self.step_towards_point(
            from,
            waypoint,
            speed,
            radius,
            base_height,
            max_step_height,
            0.05,
            prefer_center_down,
        );
        let arrived = (to - Vec2::new(new_position.x, new_position.z)).magnitude() <= 0.05
            && target_height
                .map(|height| (new_position.y - height).abs() <= max_step_height + 0.05)
                .unwrap_or(true);
        Some((new_position, arrived))
    }

    /// Direct local movement on floors without navgrid/path waypoint expansion.
    /// Intended for short player input steps where pathfinding causes overshoot on stairs.
    pub fn move_towards_on_floors_direct(
        &self,
        from: Vec2<f32>,
        to: Vec2<f32>,
        speed: f32,
        radius: f32,
        max_step_height: f32,
        reference_y: f32,
    ) -> Option<(Vec3<f32>, bool)> {
        let base_height = self
            .sample_reachable_floor_height(from, radius * 0.5, reference_y, max_step_height)
            .or_else(|| self.get_floor_height_reachable(from, reference_y, max_step_height))
            .or_else(|| {
                self.sample_reachable_floor_height(to, radius * 0.5, reference_y, max_step_height)
            })
            .or_else(|| self.get_floor_height_reachable(to, reference_y, max_step_height));
        let base_height = base_height?;

        // Direct player input can be much smaller than the pathing arrival radius.
        // Do not swallow tiny per-frame movement as "already arrived".
        let (new_position, _) = self.step_towards_point(
            from,
            to,
            speed,
            radius,
            base_height,
            max_step_height,
            0.0001,
            false,
        );
        let arrived = (to - Vec2::new(new_position.x, new_position.z)).magnitude() <= 0.05;
        Some((new_position, arrived))
    }

    pub fn probe_path_direct(
        &self,
        from: Vec2<f32>,
        to: Vec2<f32>,
        radius: f32,
        max_step_height: f32,
        reference_y: f32,
    ) -> CollisionProbeResult {
        let mut result = CollisionProbeResult {
            start: Vec3::new(from.x, reference_y, from.y),
            target: to,
            radius,
            max_step_height,
            arrived: false,
            goto_path_found: false,
            goto_path_unstable: false,
            goto_path: Vec::new(),
            steps: Vec::new(),
        };

        let Some(base_height) = self
            .sample_reachable_floor_height(from, radius * 0.5, reference_y, max_step_height)
            .or_else(|| self.get_floor_height_reachable(from, reference_y, max_step_height))
            .or_else(|| {
                self.sample_reachable_floor_height(to, radius * 0.5, reference_y, max_step_height)
            })
            .or_else(|| self.get_floor_height_reachable(to, reference_y, max_step_height))
        else {
            result.steps.push(CollisionProbeStep {
                from: Vec3::new(from.x, reference_y, from.y),
                to: Vec3::new(from.x, reference_y, from.y),
                target: to,
                current_floor: None,
                next_floor: None,
                kind: CollisionProbeStepKind::NoFloor,
                blocker: None,
                support_samples: self.floor_support_samples(
                    from,
                    radius * 0.5,
                    reference_y,
                    max_step_height,
                ),
            });
            return result;
        };

        result.start.y = base_height;

        let to_vector = to - from;
        let dist = to_vector.magnitude();
        if dist <= 0.0001 {
            result.arrived = true;
            return result;
        }

        let dir = to_vector / dist;
        let sub_step = (radius * 0.5).max(0.1);
        let mut current = Vec3::new(from.x, base_height, from.y);
        let mut remaining = dist;

        while remaining > 0.0001 && result.steps.len() < 512 {
            let len = remaining.min(sub_step);
            let delta = Vec3::new(dir.x * len, 0.0, dir.y * len);
            let current_2d = Vec2::new(current.x, current.z);
            let current_floor = self
                .sample_reachable_floor_height(current_2d, radius * 0.5, current.y, max_step_height)
                .or_else(|| {
                    self.get_floor_height_reachable(current_2d, current.y, max_step_height)
                });
            let Some(current_floor) = current_floor else {
                result.steps.push(CollisionProbeStep {
                    from: current,
                    to: current,
                    target: current_2d,
                    current_floor: None,
                    next_floor: None,
                    kind: CollisionProbeStepKind::NoFloor,
                    blocker: None,
                    support_samples: self.floor_support_samples(
                        current_2d,
                        radius * 0.5,
                        current.y,
                        max_step_height,
                    ),
                });
                break;
            };

            let probe_2d = current_2d + Vec2::new(delta.x, delta.z);
            let support_samples =
                self.floor_support_samples(probe_2d, radius * 0.5, current_floor, max_step_height);
            let Some(mut move_height) = self
                .sample_reachable_floor_height(
                    probe_2d,
                    radius * 0.5,
                    current_floor,
                    max_step_height,
                )
                .or_else(|| {
                    self.get_floor_height_reachable(probe_2d, current_floor, max_step_height)
                })
            else {
                result.steps.push(CollisionProbeStep {
                    from: current,
                    to: current,
                    target: probe_2d,
                    current_floor: Some(current_floor),
                    next_floor: None,
                    kind: CollisionProbeStepKind::NoFloor,
                    blocker: None,
                    support_samples,
                });
                break;
            };
            if (move_height - current_floor).abs() > max_step_height + 1e-3 {
                move_height = current_floor;
            }

            let step_start = Vec3::new(current.x, move_height, current.z);
            let (mut next, blocked, blocker) = self.move_distance_with_step_clearance_trace(
                step_start,
                delta,
                radius,
                Some(current_floor + max_step_height + 1e-3),
            );
            let next_2d = Vec2::new(next.x, next.z);
            let next_floor = self
                .sample_reachable_floor_height(next_2d, radius * 0.5, next.y, max_step_height)
                .or_else(|| self.get_floor_height_reachable(next_2d, next.y, max_step_height));

            if let Some(next_floor) = next_floor
                && (next_floor - current_floor).abs() <= max_step_height + 1e-3
            {
                next.y = next_floor;
            }

            let moved = Vec2::new(next.x - current.x, next.z - current.z).magnitude();
            let height_delta = next_floor.unwrap_or(move_height) - current_floor;
            let mut kind = if height_delta > 0.05 {
                CollisionProbeStepKind::StepUp
            } else if height_delta < -0.05 {
                CollisionProbeStepKind::StepDown
            } else {
                CollisionProbeStepKind::Walk
            };
            if blocked {
                kind = if moved >= len * 0.5 {
                    CollisionProbeStepKind::Contact
                } else {
                    CollisionProbeStepKind::Blocked
                };
            }

            result.steps.push(CollisionProbeStep {
                from: current,
                to: next,
                target: probe_2d,
                current_floor: Some(current_floor),
                next_floor,
                kind,
                blocker,
                support_samples,
            });

            current = next;
            remaining -= len;
            if kind == CollisionProbeStepKind::Blocked || moved < 0.0001 {
                break;
            }
        }

        let end_2d = Vec2::new(current.x, current.z);
        result.arrived = (to - end_2d).magnitude() <= 0.05;
        result.goto_path_found = result.arrived;
        result.goto_path = if result.arrived {
            let mut path = Vec::with_capacity(result.steps.len() + 1);
            path.push(result.start);
            for step in &result.steps {
                if step.to.x.is_finite()
                    && step.to.y.is_finite()
                    && step.to.z.is_finite()
                    && path
                        .last()
                        .map(|last| (*last - step.to).magnitude_squared() > 1e-6)
                        .unwrap_or(true)
                {
                    path.push(step.to);
                }
            }
            if path.len() < 2 {
                path.push(current);
            }
            path
        } else {
            self.navgrid_path_3d(from, to, radius, base_height, max_step_height, 0.05)
                .unwrap_or_default()
        };
        if !result.goto_path.is_empty() {
            result.goto_path_found = true;
        }
        result
    }

    /// Like `move_towards_on_floors`, but stops once the agent is within `dest_radius`.
    pub fn close_in_on_floors(
        &self,
        from: Vec2<f32>,
        target: Vec2<f32>,
        dest_radius: f32,
        speed: f32,
        agent_radius: f32,
        max_step_height: f32,
        reference_y: f32,
    ) -> Option<(Vec3<f32>, bool)> {
        if (target - from).magnitude() <= dest_radius {
            return Some((Vec3::new(from.x, reference_y, from.y), true));
        }

        let base_height = self
            .get_floor_height_reachable(from, reference_y, max_step_height)
            .or_else(|| self.get_floor_height_reachable(target, reference_y, max_step_height))?;

        let arrival_radius = dest_radius.max(0.05);
        let distance_before = (target - from).magnitude();
        let (direct_position, direct_arrived) = self.step_towards_point(
            from,
            target,
            speed,
            agent_radius,
            base_height,
            max_step_height,
            arrival_radius,
            false,
        );
        let direct_2d = Vec2::new(direct_position.x, direct_position.z);
        let moved = (direct_2d - from).magnitude();
        let distance_after = (target - direct_2d).magnitude();
        let requested_step = speed.min(distance_before);
        let useful_direct =
            moved >= requested_step * 0.5 && distance_after < distance_before - 0.001;
        if useful_direct || direct_arrived || distance_after <= arrival_radius {
            return Some((direct_position, distance_after <= arrival_radius));
        }

        let waypoint = if self.segment_is_clear(
            Vec3::new(from.x, base_height, from.y),
            Vec3::new(target.x, base_height, target.y),
            agent_radius,
        ) {
            target
        } else {
            self.navgrid_next_waypoint(
                from,
                target,
                agent_radius,
                base_height,
                max_step_height,
                arrival_radius,
            )?
        };

        let waypoint_height = self
            .get_floor_height_nearest(waypoint, base_height)
            .or_else(|| self.sample_floor_height(waypoint, agent_radius * 0.5));
        let prefer_center_down = waypoint_height
            .map(|height| height < base_height - 0.05)
            .unwrap_or(false);

        let (new_position, _) = self.step_towards_point(
            from,
            waypoint,
            speed,
            agent_radius,
            base_height,
            max_step_height,
            arrival_radius,
            prefer_center_down,
        );
        let arrived =
            (target - Vec2::new(new_position.x, new_position.z)).magnitude() <= arrival_radius;
        Some((new_position, arrived))
    }

    fn step_towards_point(
        &self,
        from: Vec2<f32>,
        target: Vec2<f32>,
        speed: f32,
        radius: f32,
        base_height: f32,
        max_step_height: f32,
        arrival_radius: f32,
        prefer_center_down: bool,
    ) -> (Vec3<f32>, bool) {
        let to_vector = target - from;
        let dist = to_vector.magnitude();
        if dist <= arrival_radius {
            return (Vec3::new(from.x, base_height, from.y), true);
        }

        let total_step = speed.min(dist);
        let dir = to_vector.normalized();
        let mut current = Vec3::new(from.x, base_height, from.y);
        let mut remaining = total_step;
        let sub_step = (radius * 0.5).max(0.1);

        while remaining > 0.0001 {
            let len = remaining.min(sub_step);
            let delta = Vec3::new(dir.x * len, 0.0, dir.y * len);
            let current_2d = Vec2::new(current.x, current.z);
            let current_floor = self
                .sample_reachable_floor_height(current_2d, radius * 0.5, current.y, max_step_height)
                .or_else(|| self.get_floor_height_reachable(current_2d, current.y, max_step_height))
                .unwrap_or(current.y);
            let probe_2d = current_2d + Vec2::new(delta.x, delta.z);
            let Some(mut move_height) = self
                .sample_reachable_floor_height(
                    probe_2d,
                    radius * 0.5,
                    current_floor,
                    max_step_height,
                )
                .or_else(|| {
                    self.get_floor_height_reachable(probe_2d, current_floor, max_step_height)
                })
            else {
                break;
            };
            if prefer_center_down {
                if let Some(center_down) =
                    self.get_center_floor_height_down(probe_2d, current_floor, max_step_height)
                {
                    move_height = center_down;
                }
            }
            if (move_height - current_floor).abs() > max_step_height + 1e-3 {
                move_height = current_floor;
            }
            current.y = move_height;

            let (mut next, blocked) = self.move_distance_with_step_clearance(
                current,
                delta,
                radius,
                Some(current_floor + max_step_height + 1e-3),
            );
            let next_2d = Vec2::new(next.x, next.z);
            let Some(next_floor) = self
                .sample_reachable_floor_height(next_2d, radius * 0.5, current.y, max_step_height)
                .or_else(|| self.get_floor_height_reachable(next_2d, current.y, max_step_height))
            else {
                break;
            };
            if (next_floor - current_floor).abs() <= max_step_height + 1e-3 {
                next.y = next_floor;
            }
            let moved = Vec2::new(next.x - current.x, next.z - current.z).magnitude();
            current = next;
            remaining -= len;

            // Stop after first blocking contact in this tick. This prevents tunneling
            // while still allowing local sliding from `move_distance`.
            if blocked || moved < 0.0001 {
                break;
            }
        }

        let end_2d = Vec2::new(current.x, current.z);
        let arrived = (target - end_2d).magnitude() <= arrival_radius.max(0.05);
        (current, arrived)
    }

    fn collect_blocking_segments(
        &self,
        position: Vec3<f32>,
        radius: f32,
        step_clearance_y: Option<f32>,
    ) -> Vec<CollisionSegment> {
        let chunk_coords = self.world_to_chunk(Vec2::new(position.x, position.z));
        let mut segments = Vec::new();

        for dx in -1..=1 {
            for dy in -1..=1 {
                let check_chunk = Vec2::new(chunk_coords.x + dx, chunk_coords.y + dy);
                if let Some(chunk_collision) = self.chunks.get(&check_chunk) {
                    for barrier in &chunk_collision.static_barriers {
                        if step_clearance_y.is_some_and(|clearance| barrier.max_y <= clearance) {
                            continue;
                        }
                        if self.barrier_blocks_horizontal_motion(position, barrier) {
                            segments.push(CollisionSegment {
                                geo_id: barrier.geo_id,
                                start: barrier.start,
                                end: barrier.end,
                            });
                        }
                    }
                    for volume in &chunk_collision.static_volumes {
                        if self.volume_blocks_horizontal_motion(position, radius, volume) {
                            self.add_volume_segments(volume, &mut segments);
                        }
                    }

                    for opening in &chunk_collision.dynamic_openings {
                        if self.opening_is_blocking(opening)
                            && position.y + radius >= opening.floor_height
                            && position.y - radius <= opening.ceiling_height
                        {
                            self.add_polygon_segments(
                                opening.geo_id,
                                &opening.boundary_2d,
                                &mut segments,
                            );
                        }
                    }
                }
            }
        }

        segments
    }

    fn volume_blocks_horizontal_motion(
        &self,
        position: Vec3<f32>,
        _radius: f32,
        volume: &BlockingVolume,
    ) -> bool {
        const FEET_EPS: f32 = 0.02;
        const ACTOR_HEIGHT: f32 = 1.7;
        let body_top = position.y + ACTOR_HEIGHT;

        if position.y >= volume.max.y - FEET_EPS {
            return false;
        }
        if body_top <= volume.min.y + FEET_EPS {
            return false;
        }
        true
    }

    fn barrier_blocks_horizontal_motion(
        &self,
        position: Vec3<f32>,
        barrier: &StaticBarrier,
    ) -> bool {
        const FEET_EPS: f32 = 0.02;
        const ACTOR_HEIGHT: f32 = 1.7;
        let body_top = position.y + ACTOR_HEIGHT;

        if position.y >= barrier.max_y - FEET_EPS {
            return false;
        }
        if body_top <= barrier.min_y + FEET_EPS {
            return false;
        }
        true
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
            geo_id: volume.geo_id,
            start: corners[0],
            end: corners[1],
        });
        segments.push(CollisionSegment {
            geo_id: volume.geo_id,
            start: corners[1],
            end: corners[2],
        });
        segments.push(CollisionSegment {
            geo_id: volume.geo_id,
            start: corners[2],
            end: corners[3],
        });
        segments.push(CollisionSegment {
            geo_id: volume.geo_id,
            start: corners[3],
            end: corners[0],
        });
    }

    fn add_polygon_segments(
        &self,
        geo_id: GeoId,
        polygon: &[Vec2<f32>],
        segments: &mut Vec<CollisionSegment>,
    ) {
        if polygon.len() < 2 {
            return;
        }

        for i in 0..polygon.len() {
            let start = polygon[i];
            let end = polygon[(i + 1) % polygon.len()];
            segments.push(CollisionSegment { geo_id, start, end });
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

    fn neighbor_chunks(&self, chunk_coords: Vec2<i32>) -> Vec<&ChunkCollision> {
        let mut chunks = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                let check_chunk = Vec2::new(chunk_coords.x + dx, chunk_coords.y + dy);
                if let Some(chunk_collision) = self.chunks.get(&check_chunk) {
                    chunks.push(chunk_collision);
                }
            }
        }
        chunks
    }

    fn collides_with_aabb(
        &self,
        pos: Vec3<f32>,
        radius: f32,
        min: Vec3<f32>,
        max: Vec3<f32>,
    ) -> bool {
        // First check vertical relation using feet height (pos.y).
        // This allows stepping onto low geometry (e.g. stair tops near short walls)
        // instead of being blocked forever by XZ overlap alone.
        const FEET_EPS: f32 = 0.02;
        if pos.y >= max.y - FEET_EPS {
            return false;
        }
        if pos.y < min.y - radius {
            return false;
        }

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

    fn footprint_intersects_polygon_2d(
        &self,
        center: Vec2<f32>,
        polygon: &[Vec2<f32>],
        padding: f32,
        radius: f32,
    ) -> bool {
        let sample = (radius * 0.7).max(0.12);
        let offsets = [
            Vec2::zero(),
            Vec2::new(sample, 0.0),
            Vec2::new(-sample, 0.0),
            Vec2::new(0.0, sample),
            Vec2::new(0.0, -sample),
            Vec2::new(sample * 0.7, sample * 0.7),
            Vec2::new(sample * 0.7, -sample * 0.7),
            Vec2::new(-sample * 0.7, sample * 0.7),
            Vec2::new(-sample * 0.7, -sample * 0.7),
        ];

        offsets
            .iter()
            .any(|offset| self.point_in_polygon_2d(center + *offset, polygon, padding))
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

    fn segment_is_clear(&self, start: Vec3<f32>, end: Vec3<f32>, radius: f32) -> bool {
        let delta = end - start;
        let distance = Vec2::new(delta.x, delta.z).magnitude();
        let sample_step = (radius * 0.5).max(0.2);
        let steps = (distance / sample_step).ceil().max(1.0) as i32;

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let p = start + delta * t;
            if self.is_blocked(p, radius) {
                return false;
            }
        }

        true
    }

    fn sample_floor_height(&self, position: Vec2<f32>, probe: f32) -> Option<f32> {
        if let Some(h) = self.get_floor_height(position) {
            return Some(h);
        }

        let d = probe.max(0.15);
        let offsets = [
            Vec2::new(-d, 0.0),
            Vec2::new(d, 0.0),
            Vec2::new(0.0, -d),
            Vec2::new(0.0, d),
            Vec2::new(-d, -d),
            Vec2::new(-d, d),
            Vec2::new(d, -d),
            Vec2::new(d, d),
        ];
        for off in offsets {
            if let Some(h) = self.get_floor_height(position + off) {
                return Some(h);
            }
        }
        None
    }

    pub(crate) fn sample_reachable_floor_height(
        &self,
        position: Vec2<f32>,
        probe: f32,
        reference_y: f32,
        max_step_height: f32,
    ) -> Option<f32> {
        let d = probe.max(0.05);
        let offsets = [
            Vec2::zero(),
            Vec2::new(-d, 0.0),
            Vec2::new(d, 0.0),
            Vec2::new(0.0, -d),
            Vec2::new(0.0, d),
            Vec2::new(-d, -d),
            Vec2::new(-d, d),
            Vec2::new(d, -d),
            Vec2::new(d, d),
        ];

        const SAME_LEVEL_EPS: f32 = 0.05;
        let mut best_up: Option<f32> = None;
        let mut best_up_delta = f32::INFINITY;
        let mut center_best_up: Option<f32> = None;
        let mut center_best_up_delta = f32::INFINITY;
        let mut best_same: Option<f32> = None;
        let mut best_same_delta = f32::INFINITY;
        let mut best_down: Option<f32> = None;
        let mut best_down_delta = f32::INFINITY;

        let mut consider_height = |height: f32| {
            let delta = height - reference_y;
            if delta > SAME_LEVEL_EPS && delta <= max_step_height + 1e-3 {
                if delta < best_up_delta {
                    best_up_delta = delta;
                    best_up = Some(height);
                } else if (delta - best_up_delta).abs() < 1e-4
                    && height > best_up.unwrap_or(f32::NEG_INFINITY)
                {
                    best_up = Some(height);
                }
            } else if delta.abs() <= SAME_LEVEL_EPS {
                let same_delta = delta.abs();
                if same_delta < best_same_delta {
                    best_same_delta = same_delta;
                    best_same = Some(height);
                }
            } else if delta < -SAME_LEVEL_EPS && delta.abs() <= max_step_height + 1e-3 {
                let down_delta = delta.abs();
                if down_delta < best_down_delta {
                    best_down_delta = down_delta;
                    best_down = Some(height);
                } else if (down_delta - best_down_delta).abs() < 1e-4
                    && height > best_down.unwrap_or(f32::NEG_INFINITY)
                {
                    best_down = Some(height);
                }
            }
        };

        let chunk_coords = self.world_to_chunk(position);
        for chunk_collision in self.neighbor_chunks(chunk_coords) {
            for floor in &chunk_collision.walkable_floors {
                if self.point_in_polygon_2d(position, &floor.polygon_2d, 0.0) {
                    let height = floor.height_at(position);
                    let delta = height - reference_y;
                    if delta > SAME_LEVEL_EPS
                        && delta <= max_step_height + 1e-3
                        && delta < center_best_up_delta
                    {
                        center_best_up_delta = delta;
                        center_best_up = Some(height);
                    }
                    consider_height(height);
                }

                let mut nearest_edge = f32::INFINITY;
                for index in 0..floor.polygon_2d.len() {
                    nearest_edge = nearest_edge.min(self.point_to_segment_distance_2d(
                        position,
                        floor.polygon_2d[index],
                        floor.polygon_2d[(index + 1) % floor.polygon_2d.len()],
                    ));
                }
                if nearest_edge <= d {
                    consider_height(floor.height_at(position));
                }
            }
        }

        for off in offsets {
            if let Some(height) =
                self.get_floor_height_reachable(position + off, reference_y, max_step_height)
            {
                consider_height(height);
            }
        }

        center_best_up.or(best_same).or(best_up).or(best_down)
    }

    pub fn floor_support_samples(
        &self,
        position: Vec2<f32>,
        probe: f32,
        reference_y: f32,
        max_step_height: f32,
    ) -> Vec<CollisionProbeFloorSample> {
        let d = probe.max(0.05);
        let offsets = [
            Vec2::zero(),
            Vec2::new(-d, 0.0),
            Vec2::new(d, 0.0),
            Vec2::new(0.0, -d),
            Vec2::new(0.0, d),
            Vec2::new(-d, -d),
            Vec2::new(-d, d),
            Vec2::new(d, -d),
            Vec2::new(d, d),
        ];

        let mut samples = Vec::new();
        for offset in offsets {
            let sample_pos = position + offset;
            if let Some(height) = self.get_floor_height_nearest(sample_pos, reference_y) {
                samples.push(CollisionProbeFloorSample {
                    position: sample_pos,
                    height,
                    reachable: (height - reference_y).abs() <= max_step_height + 1e-3,
                });
            }
        }
        samples
    }

    fn navgrid_next_waypoint(
        &self,
        from: Vec2<f32>,
        target: Vec2<f32>,
        radius: f32,
        base_height: f32,
        max_step_height: f32,
        goal_radius: f32,
    ) -> Option<Vec2<f32>> {
        let path = self.navgrid_path(
            from,
            target,
            radius,
            base_height,
            max_step_height,
            goal_radius,
        )?;
        if path.len() < 2 {
            return Some(target);
        }
        Some(path[1])
    }

    fn navgrid_next_waypoint_with_target_height(
        &self,
        from: Vec2<f32>,
        target: Vec2<f32>,
        radius: f32,
        base_height: f32,
        max_step_height: f32,
        goal_radius: f32,
        target_height: Option<f32>,
    ) -> Option<Vec2<f32>> {
        let path = self.navgrid_path_3d_with_target_height(
            from,
            target,
            radius,
            base_height,
            max_step_height,
            goal_radius,
            target_height,
        )?;
        if path.len() < 2 {
            return Some(target);
        }
        Some(Vec2::new(path[1].x, path[1].z))
    }

    fn navgrid_path(
        &self,
        from: Vec2<f32>,
        target: Vec2<f32>,
        radius: f32,
        base_height: f32,
        max_step_height: f32,
        goal_radius: f32,
    ) -> Option<Vec<Vec2<f32>>> {
        self.navgrid_path_3d(
            from,
            target,
            radius,
            base_height,
            max_step_height,
            goal_radius,
        )
        .map(|path| {
            path.into_iter()
                .map(|point| Vec2::new(point.x, point.z))
                .collect()
        })
    }

    fn navgrid_path_3d(
        &self,
        from: Vec2<f32>,
        target: Vec2<f32>,
        radius: f32,
        base_height: f32,
        max_step_height: f32,
        goal_radius: f32,
    ) -> Option<Vec<Vec3<f32>>> {
        self.navgrid_path_3d_with_target_height(
            from,
            target,
            radius,
            base_height,
            max_step_height,
            goal_radius,
            None,
        )
    }

    pub fn navgrid_path_3d_to_height(
        &self,
        from: Vec2<f32>,
        target: Vec2<f32>,
        target_height: Option<f32>,
        radius: f32,
        reference_y: f32,
        max_step_height: f32,
        goal_radius: f32,
    ) -> Option<Vec<Vec3<f32>>> {
        let base_height = self
            .sample_reachable_floor_height(from, radius * 0.5, reference_y, max_step_height)
            .or_else(|| self.get_floor_height_reachable(from, reference_y, max_step_height))
            .or_else(|| {
                self.sample_reachable_floor_height(
                    target,
                    radius * 0.5,
                    reference_y,
                    max_step_height,
                )
            })
            .or_else(|| self.get_floor_height_reachable(target, reference_y, max_step_height))?;

        self.navgrid_path_3d_with_target_height(
            from,
            target,
            radius,
            base_height,
            max_step_height,
            goal_radius,
            target_height,
        )
    }

    fn navgrid_path_3d_with_target_height(
        &self,
        from: Vec2<f32>,
        target: Vec2<f32>,
        radius: f32,
        base_height: f32,
        max_step_height: f32,
        goal_radius: f32,
        target_height_override: Option<f32>,
    ) -> Option<Vec<Vec3<f32>>> {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        struct NavNode {
            cell: Vec2<i32>,
            height_key: i32,
        }

        impl NavNode {
            fn new(cell: Vec2<i32>, height: f32) -> Self {
                Self {
                    cell,
                    height_key: (height * 100.0).round() as i32,
                }
            }

            fn height(self) -> f32 {
                self.height_key as f32 / 100.0
            }
        }

        let cell = (radius * 0.6).clamp(0.15, 0.5);
        let nav_step_height = max_step_height;
        let direct_distance = (target - from).magnitude();
        let search_radius_world = (direct_distance + 8.0).clamp(8.0, 120.0);
        let search_radius_cells = (search_radius_world / cell).ceil() as i32;

        let start_cell = (from / cell).floor().as_::<i32>();
        let target_cell = (target / cell).floor().as_::<i32>();
        let cell_center = |c: Vec2<i32>| (c.map(|v| v as f32) + Vec2::broadcast(0.5)) * cell;

        let min_cell = Vec2::new(
            start_cell.x.min(target_cell.x) - search_radius_cells,
            start_cell.y.min(target_cell.y) - search_radius_cells,
        );
        let max_cell = Vec2::new(
            start_cell.x.max(target_cell.x) + search_radius_cells,
            start_cell.y.max(target_cell.y) + search_radius_cells,
        );

        let in_bounds = |c: Vec2<i32>| {
            c.x >= min_cell.x && c.x <= max_cell.x && c.y >= min_cell.y && c.y <= max_cell.y
        };
        let sample_point_nav_height = |pos: Vec2<f32>, reference_y: f32| {
            const SAME_LEVEL_EPS: f32 = 0.05;
            let mut best_up: Option<f32> = None;
            let mut best_up_delta = f32::INFINITY;
            let mut best_down: Option<f32> = None;
            let mut best_down_delta = f32::INFINITY;
            let mut best_same: Option<f32> = None;
            let mut best_same_delta = f32::INFINITY;

            let chunk_coords = self.world_to_chunk(pos);
            for chunk_collision in self.neighbor_chunks(chunk_coords) {
                for floor in &chunk_collision.walkable_floors {
                    if !self.point_in_polygon_2d(pos, &floor.polygon_2d, 0.0) {
                        continue;
                    }
                    let height = floor.height_at(pos);
                    let delta = height - reference_y;
                    if delta > SAME_LEVEL_EPS && delta <= max_step_height + 1e-3 {
                        if delta < best_up_delta {
                            best_up_delta = delta;
                            best_up = Some(height);
                        }
                    } else if delta < -SAME_LEVEL_EPS && delta.abs() <= max_step_height + 1e-3 {
                        let down_delta = delta.abs();
                        if down_delta < best_down_delta {
                            best_down_delta = down_delta;
                            best_down = Some(height);
                        }
                    } else if delta.abs() <= SAME_LEVEL_EPS {
                        let same_delta = delta.abs();
                        if same_delta < best_same_delta {
                            best_same_delta = same_delta;
                            best_same = Some(height);
                        }
                    }
                }
            }

            best_same.or(best_up).or(best_down)
        };
        let height_cache: RefCell<FxHashMap<(i32, i32, i32), Option<f32>>> =
            RefCell::new(FxHashMap::default());
        let passable_cache: RefCell<FxHashMap<(i32, i32, i32), bool>> =
            RefCell::new(FxHashMap::default());

        let sample_nav_height = |c: Vec2<i32>, reference_y: f32| {
            let key = (c.x, c.y, (reference_y * 100.0).round() as i32);
            if let Some(height) = height_cache.borrow().get(&key).copied() {
                return height;
            }
            let height = sample_point_nav_height(cell_center(c), reference_y);
            height_cache.borrow_mut().insert(key, height);
            height
        };
        let cell_passable = |c: Vec2<i32>, h: f32| {
            let key = (c.x, c.y, (h * 100.0).round() as i32);
            if let Some(passable) = passable_cache.borrow().get(&key).copied() {
                return passable;
            }
            let pos = cell_center(c);
            let pos3 = Vec3::new(pos.x, h, pos.y);
            let segments =
                self.collect_blocking_segments(pos3, radius, Some(h + max_step_height + 1e-3));
            if segments.iter().any(|seg| {
                self.check_point_against_segment(
                    Vec2::new(pos3.x, pos3.z),
                    seg.start,
                    seg.end,
                    radius,
                )
                .is_some()
            }) {
                passable_cache.borrow_mut().insert(key, false);
                return false;
            }

            let chunk_coords = self.world_to_chunk(pos);
            for chunk_collision in self.neighbor_chunks(chunk_coords) {
                for opening in &chunk_collision.dynamic_openings {
                    if !self.opening_is_blocking(opening) {
                        continue;
                    }
                    if pos3.y + radius < opening.floor_height
                        || pos3.y - radius > opening.ceiling_height
                    {
                        continue;
                    }
                    if self.point_in_polygon_2d(pos, &opening.boundary_2d, radius) {
                        passable_cache.borrow_mut().insert(key, false);
                        return false;
                    }
                }
            }

            passable_cache.borrow_mut().insert(key, true);
            true
        };

        let start_height = sample_nav_height(start_cell, base_height).unwrap_or(base_height);
        let (start_cell, start_height) = if cell_passable(start_cell, start_height) {
            (start_cell, start_height)
        } else {
            // Runtime movement can land near a cell edge on small stairs. Recover to the
            // nearest passable neighbor instead of declaring the actor stuck.
            let mut best_start: Option<(Vec2<i32>, f32, f32)> = None;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let candidate = start_cell + Vec2::new(dx, dy);
                    if !in_bounds(candidate) {
                        continue;
                    }
                    let Some(height) = sample_nav_height(candidate, base_height) else {
                        continue;
                    };
                    if (height - base_height).abs() > max_step_height + 1e-3 {
                        continue;
                    }
                    if !cell_passable(candidate, height) {
                        continue;
                    }
                    let dist = (cell_center(candidate) - from).magnitude();
                    if best_start.map_or(true, |(_, _, best_dist)| dist < best_dist) {
                        best_start = Some((candidate, height, dist));
                    }
                }
            }
            let (candidate, height, _) = best_start?;
            (candidate, height)
        };
        let start = NavNode::new(start_cell, start_height);
        let target_height = target_height_override.or_else(|| {
            self.get_floor_height_nearest(target, base_height)
                .or_else(|| self.sample_floor_height(target, radius * 0.5))
        });

        if !cell_passable(start_cell, start_height) {
            return None;
        }

        let successors = |node: &NavNode| {
            let dirs = [
                Vec2::new(-1, 0),
                Vec2::new(1, 0),
                Vec2::new(0, -1),
                Vec2::new(0, 1),
                Vec2::new(-1, -1),
                Vec2::new(-1, 1),
                Vec2::new(1, -1),
                Vec2::new(1, 1),
            ];

            let h0 = node.height();
            let p0 = cell_center(node.cell);
            dirs.iter()
                .map(|d| node.cell + *d)
                .filter(|n| in_bounds(*n))
                .filter_map(|n| {
                    let h1 = sample_nav_height(n, h0)?;
                    if (h0 - h1).abs() > nav_step_height + 1e-3 {
                        return None;
                    }
                    if !cell_passable(n, h1) {
                        return None;
                    }
                    let p1 = cell_center(n);
                    let step_start = Vec3::new(p0.x, h0, p0.y);
                    let step_delta = Vec3::new(p1.x - p0.x, 0.0, p1.y - p0.y);
                    let (stepped, blocked) = self.move_distance_with_step_clearance(
                        step_start,
                        step_delta,
                        radius,
                        Some(h0 + nav_step_height + 1e-3),
                    );
                    let stepped_2d = Vec2::new(stepped.x, stepped.z);
                    if blocked || (stepped_2d - p1).magnitude() > cell * 0.25 {
                        return None;
                    }
                    let is_diag = (n.x - node.cell.x).abs() == 1 && (n.y - node.cell.y).abs() == 1;
                    let height_cost = ((h1 - h0).abs() * 2.0).round() as i32;
                    let cost = if is_diag { 14 } else { 10 } + height_cost;
                    Some((NavNode::new(n, h1), cost))
                })
                .collect::<Vec<_>>()
        };

        let heuristic = |node: &NavNode| {
            let p = cell_center(node.cell);
            let d = (p - target).magnitude() - goal_radius;
            (d.max(0.0) * 10.0) as i32
        };

        let is_goal = |node: &NavNode| {
            let p = cell_center(node.cell);
            if (p - target).magnitude() > goal_radius.max(cell) {
                return false;
            }
            target_height
                .map(|height| (node.height() - height).abs() <= nav_step_height + 0.05)
                .unwrap_or(true)
        };

        let (path, _) = astar(&start, &successors, heuristic, is_goal)?;
        let mut points = Vec::with_capacity(path.len() + 2);
        points.push(Vec3::new(from.x, start_height, from.y));
        for node in path.iter().skip(1) {
            let center = cell_center(node.cell);
            points.push(Vec3::new(center.x, node.height(), center.y));
        }
        let target_y = target_height.unwrap_or_else(|| {
            path.last()
                .map(|node| node.height())
                .unwrap_or(start_height)
        });
        points.push(Vec3::new(target.x, target_y, target.y));
        Some(points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate_fixture() -> (crate::Map, CollisionWorld) {
        use crate::chunkbuilder::{ChunkBuilder, d3chunkbuilder::D3ChunkBuilder};

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_projects/Gate.eldiron");
        let contents = std::fs::read_to_string(path).expect("Gate fixture is readable");
        let project: serde_json::Value =
            serde_json::from_str(&contents).expect("Gate fixture is valid json");
        let map_value = project["regions"][0]["map"].clone();
        let map: crate::Map = serde_json::from_value(map_value).expect("Gate map deserializes");

        let assets = crate::Assets::default();
        let mut world = CollisionWorld::default();
        let mut chunk_builder = D3ChunkBuilder::new();
        let chunk_size = 10;
        let bbox = map.bbox();
        let min_chunk = Vec2::new(
            (bbox.min.x / chunk_size as f32).floor() as i32,
            (bbox.min.y / chunk_size as f32).floor() as i32,
        );
        let max_chunk = Vec2::new(
            (bbox.max.x / chunk_size as f32).floor() as i32,
            (bbox.max.y / chunk_size as f32).floor() as i32,
        );
        for cy in min_chunk.y..=max_chunk.y {
            for cx in min_chunk.x..=max_chunk.x {
                let chunk_origin = Vec2::new(cx, cy);
                let collision =
                    chunk_builder.build_collision(&map, &assets, chunk_origin, chunk_size);
                world.update_chunk(chunk_origin, collision);
            }
        }

        (map, world)
    }

    fn rect(min_x: f32, min_z: f32, max_x: f32, max_z: f32) -> Vec<Vec2<f32>> {
        vec![
            Vec2::new(min_x, min_z),
            Vec2::new(max_x, min_z),
            Vec2::new(max_x, max_z),
            Vec2::new(min_x, max_z),
        ]
    }

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

    #[test]
    fn test_floor_movement_steps_over_low_floor_edge() {
        let mut world = CollisionWorld::new(10);
        let mut chunk = ChunkCollision::new();
        let geo_id = GeoId::Sector(1);

        chunk.walkable_floors.push(WalkableFloor::flat(
            geo_id,
            0.5,
            vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(7.0, 0.0),
                Vec2::new(7.0, 5.0),
                Vec2::new(0.0, 5.0),
            ],
        ));
        chunk.walkable_floors.push(WalkableFloor::flat(
            geo_id,
            1.0,
            vec![
                Vec2::new(0.0, 5.0),
                Vec2::new(7.0, 5.0),
                Vec2::new(7.0, 6.0),
                Vec2::new(0.0, 6.0),
            ],
        ));
        chunk.static_barriers.push(StaticBarrier {
            geo_id,
            start: Vec2::new(0.0, 5.0),
            end: Vec2::new(7.0, 5.0),
            min_y: 0.0,
            max_y: 1.0,
        });
        world.update_chunk(Vec2::new(0, 0), chunk);

        let (end, arrived) = world
            .move_towards_on_floors_direct(
                Vec2::new(3.5, 4.4),
                Vec2::new(3.5, 5.6),
                1.2,
                0.5,
                1.0,
                0.5,
            )
            .unwrap();

        assert!(arrived);
        assert!(end.z > 5.5);
        assert!((end.y - 1.0).abs() < 1e-4);
    }

    #[test]
    fn mesh_goto_finds_gate_guardhouse_roof_route() {
        use crate::chunkbuilder::{ChunkBuilder, d3chunkbuilder::D3ChunkBuilder};

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_projects/Gate.eldiron");
        let contents = std::fs::read_to_string(path).expect("Gate fixture is readable");
        let project: serde_json::Value =
            serde_json::from_str(&contents).expect("Gate fixture is valid json");
        let map_value = project["regions"][0]["map"].clone();
        let map: crate::Map = serde_json::from_value(map_value).expect("Gate map deserializes");

        let guard = map
            .entities
            .iter()
            .find(|entity| {
                entity
                    .attributes
                    .get_str("class_name")
                    .map(|value| value.eq_ignore_ascii_case("Guard"))
                    .unwrap_or(false)
            })
            .expect("Gate fixture has a guard entity");
        let target = map
            .named_area_center("GuardHouseRoof")
            .expect("Gate fixture has GuardHouseRoof area");

        let assets = crate::Assets::default();
        let mut world = CollisionWorld::default();
        let mut chunk_builder = D3ChunkBuilder::new();
        let chunk_size = 10;
        let bbox = map.bbox();
        let min_chunk = Vec2::new(
            (bbox.min.x / chunk_size as f32).floor() as i32,
            (bbox.min.y / chunk_size as f32).floor() as i32,
        );
        let max_chunk = Vec2::new(
            (bbox.max.x / chunk_size as f32).floor() as i32,
            (bbox.max.y / chunk_size as f32).floor() as i32,
        );
        for cy in min_chunk.y..=max_chunk.y {
            for cx in min_chunk.x..=max_chunk.x {
                let chunk_origin = Vec2::new(cx, cy);
                let collision =
                    chunk_builder.build_collision(&map, &assets, chunk_origin, chunk_size);
                world.update_chunk(chunk_origin, collision);
            }
        }

        let route = world.navgrid_path(
            guard.get_pos_xz(),
            target,
            guard.attributes.get_float_default("radius", 0.5) - 0.01,
            guard.position.y,
            1.0,
            0.05,
        );

        assert!(
            route.as_ref().map(|path| path.len()).unwrap_or(0) > 2,
            "guard should find a multi-waypoint mesh route to GuardHouseRoof"
        );

        let route = route.unwrap();
        let start = guard.get_pos_xz();
        let first_waypoint = route[1];
        let radius = guard.attributes.get_float_default("radius", 0.5) - 0.01;
        let (next_position, arrived) = world
            .move_towards_on_floors(start, target, 0.25, radius, 1.0, guard.position.y)
            .expect("guard should take a mesh goto step");
        let next_2d = Vec2::new(next_position.x, next_position.z);
        let direct_2d = start + (target - start).normalized() * 0.25;
        let waypoint_2d = start + (first_waypoint - start).normalized() * 0.25;

        assert!(!arrived);
        assert!(
            next_2d.distance_squared(waypoint_2d) < next_2d.distance_squared(direct_2d),
            "first movement step should follow the stair route waypoint, not the roof center"
        );

        let probe = world.probe_path_direct(start, target, radius, 1.0, guard.position.y);
        assert!(probe.goto_path_found, "probe should expose the goto route");
        assert!(
            probe
                .goto_path
                .iter()
                .any(|point| point.y < guard.position.y - 0.5),
            "probe goto route should preserve stair height changes"
        );
        assert!(
            probe
                .goto_path
                .last()
                .map(|point| (point.y - 4.0).abs() < 1e-3)
                .unwrap_or(false),
            "probe goto route should end on the roof floor"
        );

        let mut pos = guard.position;
        let mut reached_roof = false;
        for _ in 0..120 {
            let Some((next, arrived)) = world.move_towards_on_floors(
                Vec2::new(pos.x, pos.z),
                target,
                0.25,
                radius,
                1.0,
                pos.y,
            ) else {
                break;
            };
            pos = next;
            if arrived {
                reached_roof = true;
                break;
            }
        }
        assert!(reached_roof, "guard should keep following the stair route");
        assert!(
            (pos.y - 4.0).abs() < 1e-3,
            "guard should end on the roof floor"
        );
    }

    #[test]
    fn mesh_goto_follows_gate_roof_route_from_road() {
        let (map, world) = gate_fixture();
        let start = map
            .named_area_center_3d("Road")
            .expect("Gate fixture has Road area");
        let target = map
            .named_area_center_3d("GateRoof")
            .expect("Gate fixture has GateRoof area");
        let radius = 0.49;
        let route = world
            .navgrid_path_3d_to_height(
                Vec2::new(start.x, start.z),
                Vec2::new(target.x, target.z),
                Some(target.y),
                radius,
                start.y,
                1.0,
                0.05,
            )
            .expect("Road should have a mesh route to GateRoof");

        assert!(
            route.len() > 2,
            "Road to GateRoof should use stairs, not a direct flat hop"
        );
        assert!(
            route
                .last()
                .map(|point| (point.y - target.y).abs() <= 1.05)
                .unwrap_or(false),
            "route should end at GateRoof height"
        );

        let mut pos = start;
        for waypoint in route.iter().skip(1) {
            let waypoint_2d = Vec2::new(waypoint.x, waypoint.z);
            let mut reached_waypoint = false;
            for _ in 0..80 {
                let (next, arrived) = world
                    .move_towards_on_floors_to_height(
                        Vec2::new(pos.x, pos.z),
                        waypoint_2d,
                        waypoint.y,
                        0.25,
                        radius,
                        1.0,
                        pos.y,
                    )
                    .expect("cached route waypoint should remain walkable");
                pos = next;
                if arrived {
                    reached_waypoint = true;
                    break;
                }
            }
            assert!(
                reached_waypoint,
                "actor should physically reach cached waypoint {waypoint:?} from {pos:?}"
            );
        }

        assert!(
            (Vec2::new(pos.x, pos.z) - Vec2::new(target.x, target.z)).magnitude() <= 0.1
                && (pos.y - target.y).abs() <= 1.05,
            "actor should end on GateRoof, got {pos:?}, target {target:?}"
        );
    }

    #[test]
    fn gate_player_direct_movement_blocks_guardhouse_front_wall() {
        let (_map, world) = gate_fixture();
        let start = Vec2::new(8.5, -0.6);
        let target = Vec2::new(8.5, -2.0);
        let radius = 0.49;

        let (end, arrived) = world
            .move_towards_on_floors_direct(start, target, 1.4, radius, 0.45, 0.375)
            .expect("guardhouse front floor should have collision context");

        assert!(
            !arrived,
            "solid guardhouse front wall should block direct player movement"
        );
        assert!(
            end.z > -1.45,
            "player should stay in front of the wall, got {end:?}"
        );
    }

    #[test]
    fn gate_player_direct_movement_can_climb_guardhouse_stairs() {
        let (_map, world) = gate_fixture();
        let radius = 0.49;
        let mut pos = Vec3::new(8.6, 0.375, -2.6);
        let target = Vec2::new(8.6, -4.2);

        for _ in 0..20 {
            let (next, _) = world
                .move_towards_on_floors_direct(
                    Vec2::new(pos.x, pos.z),
                    target,
                    0.12,
                    radius,
                    0.55,
                    pos.y,
                )
                .expect("guardhouse stairs should remain walkable");
            pos = next;
            if (target - Vec2::new(pos.x, pos.z)).magnitude() <= 0.1 {
                break;
            }
        }

        assert!(
            pos.z < -3.8 && pos.y > 0.5,
            "player should make upward progress on the stairs, got {pos:?}"
        );
    }

    #[test]
    fn test_floor_movement_bridges_tiny_floor_gap_both_directions() {
        let mut world = CollisionWorld::new(10);
        let mut chunk = ChunkCollision::new();
        let geo_id = GeoId::GeometryObject(uuid::Uuid::nil());

        chunk
            .walkable_floors
            .push(WalkableFloor::flat(geo_id, 0.5, rect(0.0, 0.0, 3.0, 5.0)));
        chunk
            .walkable_floors
            .push(WalkableFloor::flat(geo_id, 0.5, rect(3.02, 0.0, 6.0, 5.0)));
        world.update_chunk(Vec2::new(0, 0), chunk);

        let (forward, forward_arrived) = world
            .move_towards_on_floors_direct(
                Vec2::new(2.8, 2.5),
                Vec2::new(3.3, 2.5),
                0.5,
                0.5,
                1.0,
                0.5,
            )
            .unwrap();
        assert!(forward_arrived);
        assert!(forward.x > 3.25);
        assert!((forward.y - 0.5).abs() < 1e-4);

        let (backward, backward_arrived) = world
            .move_towards_on_floors_direct(
                Vec2::new(3.3, 2.5),
                Vec2::new(2.8, 2.5),
                0.5,
                0.5,
                1.0,
                0.5,
            )
            .unwrap();
        assert!(backward_arrived);
        assert!(backward.x < 2.85);
        assert!((backward.y - 0.5).abs() < 1e-4);
    }

    #[test]
    fn test_floor_direct_movement_preserves_tiny_input_steps() {
        let mut world = CollisionWorld::new(10);
        let mut chunk = ChunkCollision::new();
        let geo_id = GeoId::GeometryObject(uuid::Uuid::nil());

        chunk.walkable_floors.push(WalkableFloor::flat(
            geo_id,
            0.0,
            vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(4.0, 0.0),
                Vec2::new(4.0, 4.0),
                Vec2::new(0.0, 4.0),
            ],
        ));
        world.update_chunk(Vec2::new(0, 0), chunk);

        let from = Vec2::new(1.0, 1.0);
        let to = Vec2::new(1.008, 1.0);
        let (end, arrived) = world
            .move_towards_on_floors_direct(from, to, 0.008, 0.49, 1.0, 0.0)
            .unwrap();

        assert!(arrived);
        assert!(end.x > 1.007, "tiny input step was swallowed: {end:?}");
        assert!((end.y - 0.0).abs() < 1e-4);
    }

    #[test]
    fn test_floor_direct_movement_uses_radius_support_on_narrow_bridge_edge() {
        let mut world = CollisionWorld::new(10);
        let mut chunk = ChunkCollision::new();
        let lower_id = GeoId::GeometryObject(uuid::Uuid::nil());
        let side_id = GeoId::GeometryObject(uuid::Uuid::from_u128(2));
        let bridge_id = GeoId::GeometryObject(uuid::Uuid::from_u128(1));

        chunk.walkable_floors.push(WalkableFloor::flat(
            lower_id,
            0.5,
            vec![
                Vec2::new(-3.0, -3.0),
                Vec2::new(3.0, -3.0),
                Vec2::new(3.0, 3.0),
                Vec2::new(-3.0, 3.0),
            ],
        ));
        chunk.walkable_floors.push(WalkableFloor::flat(
            side_id,
            0.95,
            vec![
                Vec2::new(-2.0, -2.5),
                Vec2::new(1.0, -2.5),
                Vec2::new(1.0, -1.875),
                Vec2::new(-2.0, -1.875),
            ],
        ));
        chunk.walkable_floors.push(WalkableFloor::flat(
            bridge_id,
            1.75,
            vec![
                Vec2::new(-1.25, -1.875),
                Vec2::new(0.25, -1.875),
                Vec2::new(0.25, -1.800),
                Vec2::new(-1.25, -1.800),
            ],
        ));
        world.update_chunk(Vec2::new(0, 0), chunk);

        let (end, arrived) = world
            .move_towards_on_floors_direct(
                Vec2::new(-0.897, -1.890),
                Vec2::new(-0.881, -1.992),
                0.104,
                0.49,
                1.0,
                1.75,
            )
            .unwrap();

        assert!(arrived);
        assert!(end.z < -1.98);
        assert!(
            (end.y - 1.75).abs() < 1e-4,
            "bridge edge support snapped to wrong height: {end:?}",
        );
    }

    #[test]
    fn test_floor_direct_movement_steps_up_when_center_reaches_low_balcony() {
        let mut world = CollisionWorld::new(10);
        let mut chunk = ChunkCollision::new();
        let ground_id = GeoId::GeometryObject(uuid::Uuid::nil());
        let balcony_id = GeoId::GeometryObject(uuid::Uuid::from_u128(1));

        chunk.walkable_floors.push(WalkableFloor::flat(
            ground_id,
            0.0,
            vec![
                Vec2::new(-2.0, -1.0),
                Vec2::new(3.0, -1.0),
                Vec2::new(3.0, 1.0),
                Vec2::new(-2.0, 1.0),
            ],
        ));
        chunk.walkable_floors.push(WalkableFloor::flat(
            balcony_id,
            0.2,
            vec![
                Vec2::new(1.0, -1.0),
                Vec2::new(3.0, -1.0),
                Vec2::new(3.0, 1.0),
                Vec2::new(1.0, 1.0),
            ],
        ));
        world.update_chunk(Vec2::new(0, 0), chunk);

        let (end, arrived) = world
            .move_towards_on_floors_direct(
                Vec2::new(0.9, 0.0),
                Vec2::new(1.1, 0.0),
                0.2,
                0.49,
                1.0,
                0.0,
            )
            .unwrap();

        assert!(arrived);
        assert!(end.x > 1.09);
        assert!(
            (end.y - 0.2).abs() < 1e-4,
            "center entering low balcony should step up: {end:?}",
        );
    }
}
