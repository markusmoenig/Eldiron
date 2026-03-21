use pathfinding::prelude::astar;
use rustc_hash::FxHashMap;
use scenevm::GeoId;
use vek::{Vec2, Vec3};

fn stairs_debug_enabled() -> bool {
    matches!(
        std::env::var("ELDIRON_STAIRS_DEBUG").ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
    )
}

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
}

#[derive(Clone, Copy, Debug)]
struct CollisionSegment {
    geo_id: GeoId,
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
            static_barriers: Vec::new(),
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
                    if closest_collision.map_or(true, |(d, _, _)| distance < d) {
                        closest_collision = Some((distance, normal, seg.geo_id));
                    }
                }
            }

            match closest_collision {
                Some((distance, normal, geo_id)) => {
                    blocked = true;
                    if stairs_debug_enabled() {
                        println!(
                            "[StairsDebug][blocked-segment] pos=({:.3},{:.3},{:.3}) move=({:.3},{:.3},{:.3}) geo={:?} dist={:.3} normal=({:.3},{:.3})",
                            start_pos.x,
                            start_pos.y,
                            start_pos.z,
                            move_vector.x,
                            move_vector.y,
                            move_vector.z,
                            geo_id,
                            distance,
                            normal.x,
                            normal.y
                        );
                    }

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

        if let Some(chunk_collision) = self.chunks.get(&chunk_coords) {
            let mut best_height: Option<f32> = None;
            for floor in &chunk_collision.walkable_floors {
                if self.point_in_polygon_2d(position, &floor.polygon_2d, 0.0) {
                    best_height = Some(match best_height {
                        Some(curr) => curr.max(floor.height),
                        None => floor.height,
                    });
                }
            }
            if best_height.is_some() {
                return best_height;
            }
        }

        None
    }

    /// Find floor height at position, preferring floors at or below `reference_y`.
    /// This avoids snapping actors to ceilings/upper decks in overlapping XZ areas.
    /// If no floor exists at/below reference, falls back to the nearest above.
    pub fn get_floor_height_nearest(&self, position: Vec2<f32>, reference_y: f32) -> Option<f32> {
        let chunk_coords = self.world_to_chunk(position);

        if let Some(chunk_collision) = self.chunks.get(&chunk_coords) {
            const FLOOR_EPS: f32 = 0.05;
            let mut best_below: Option<f32> = None;
            let mut best_above: Option<f32> = None;
            let mut best_above_dist = f32::INFINITY;
            for floor in &chunk_collision.walkable_floors {
                if self.point_in_polygon_2d(position, &floor.polygon_2d, 0.0) {
                    if floor.height <= reference_y + FLOOR_EPS {
                        best_below = Some(match best_below {
                            Some(curr) => curr.max(floor.height),
                            None => floor.height,
                        });
                    } else {
                        let d = floor.height - reference_y;
                        if d < best_above_dist {
                            best_above_dist = d;
                            best_above = Some(floor.height);
                        } else if (d - best_above_dist).abs() < 1e-4
                            && floor.height < best_above.unwrap_or(f32::INFINITY)
                        {
                            // Stable tie-breaker for above floors: prefer lower one.
                            best_above = Some(floor.height);
                        }
                    }
                }
            }
            if best_below.is_some() || best_above.is_some() {
                return best_below.or(best_above);
            }
        }

        None
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

        if let Some(chunk_collision) = self.chunks.get(&chunk_coords) {
            const SAME_LEVEL_EPS: f32 = 0.05;
            let mut best_up: Option<f32> = None;
            let mut best_up_delta = f32::INFINITY;
            let mut best_same: Option<f32> = None;
            let mut best_down: Option<f32> = None;
            let mut best_down_delta = f32::INFINITY;

            for floor in &chunk_collision.walkable_floors {
                if !self.point_in_polygon_2d(position, &floor.polygon_2d, 0.0) {
                    continue;
                }
                let delta = floor.height - reference_y;
                if delta > SAME_LEVEL_EPS && delta <= max_step_height + 1e-3 {
                    if delta < best_up_delta {
                        best_up_delta = delta;
                        best_up = Some(floor.height);
                    } else if (delta - best_up_delta).abs() < 1e-4
                        && floor.height > best_up.unwrap_or(f32::NEG_INFINITY)
                    {
                        best_up = Some(floor.height);
                    }
                } else if delta >= -SAME_LEVEL_EPS && delta <= SAME_LEVEL_EPS {
                    best_same = Some(floor.height);
                } else if delta < -SAME_LEVEL_EPS && delta.abs() <= max_step_height + 1e-3 {
                    let down_delta = delta.abs();
                    if down_delta < best_down_delta {
                        best_down_delta = down_delta;
                        best_down = Some(floor.height);
                    } else if (down_delta - best_down_delta).abs() < 1e-4
                        && floor.height > best_down.unwrap_or(f32::NEG_INFINITY)
                    {
                        best_down = Some(floor.height);
                    }
                }
            }

            return best_up.or(best_same).or(best_down);
        }

        None
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
        let base_height = self
            .get_floor_height_reachable(from, reference_y, max_step_height)
            .or_else(|| self.get_floor_height_reachable(to, reference_y, max_step_height))?;

        let waypoint = if self.segment_is_clear(
            Vec3::new(from.x, base_height, from.y),
            Vec3::new(to.x, base_height, to.y),
            radius,
        ) {
            to
        } else {
            self.navgrid_next_waypoint(from, to, radius, base_height, max_step_height, 0.05)?
        };

        let (new_position, _) = self.step_towards_point(
            from,
            waypoint,
            speed,
            radius,
            base_height,
            max_step_height,
            0.05,
        );
        let arrived = (to - Vec2::new(new_position.x, new_position.z)).magnitude() <= 0.05;
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
            .get_floor_height_reachable(from, reference_y, max_step_height)
            .or_else(|| self.get_floor_height_reachable(to, reference_y, max_step_height))?;

        let (new_position, _) =
            self.step_towards_point(from, to, speed, radius, base_height, max_step_height, 0.05);
        let arrived = (to - Vec2::new(new_position.x, new_position.z)).magnitude() <= 0.05;
        Some((new_position, arrived))
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
                dest_radius,
            )?
        };

        let (new_position, _) = self.step_towards_point(
            from,
            waypoint,
            speed,
            agent_radius,
            base_height,
            max_step_height,
            0.05,
        );
        let arrived =
            (target - Vec2::new(new_position.x, new_position.z)).magnitude() <= dest_radius;
        Some((new_position, arrived))
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

    fn step_towards_point(
        &self,
        from: Vec2<f32>,
        target: Vec2<f32>,
        speed: f32,
        radius: f32,
        base_height: f32,
        max_step_height: f32,
        arrival_radius: f32,
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
                .get_floor_height_reachable(current_2d, current.y, max_step_height)
                .unwrap_or(current.y);
            let probe_2d = current_2d + Vec2::new(delta.x, delta.z);
            let mut move_height = current_floor;
            if let Some(next_floor) =
                self.get_floor_height_reachable(probe_2d, current_floor, max_step_height)
            {
                if (next_floor - current_floor).abs() <= max_step_height + 1e-3 {
                    move_height = next_floor;
                }
            }
            if stairs_debug_enabled() && (move_height - current.y).abs() > 0.001 {
                println!(
                    "[StairsDebug][step-floor] from=({:.3},{:.3},{:.3}) probe=({:.3},{:.3}) current_floor={:.3} move_height={:.3}",
                    current.x,
                    current.y,
                    current.z,
                    probe_2d.x,
                    probe_2d.y,
                    current_floor,
                    move_height
                );
            }
            current.y = move_height;

            let (mut next, blocked) = self.move_distance(current, delta, radius);
            let next_2d = Vec2::new(next.x, next.z);
            if let Some(next_floor) =
                self.get_floor_height_reachable(next_2d, current.y, max_step_height)
            {
                if (next_floor - current_floor).abs() <= max_step_height + 1e-3 {
                    if stairs_debug_enabled() && (next_floor - next.y).abs() > 0.001 {
                        println!(
                            "[StairsDebug][land-floor] pos=({:.3},{:.3},{:.3}) next_floor={:.3} blocked={}",
                            next.x, next.y, next.z, next_floor, blocked
                        );
                    }
                    next.y = next_floor;
                }
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

    fn collect_blocking_segments(&self, position: Vec3<f32>, radius: f32) -> Vec<CollisionSegment> {
        let chunk_coords = self.world_to_chunk(Vec2::new(position.x, position.z));
        let mut segments = Vec::new();

        for dx in -1..=1 {
            for dy in -1..=1 {
                let check_chunk = Vec2::new(chunk_coords.x + dx, chunk_coords.y + dy);
                if let Some(chunk_collision) = self.chunks.get(&check_chunk) {
                    for barrier in &chunk_collision.static_barriers {
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

    fn navgrid_next_waypoint(
        &self,
        from: Vec2<f32>,
        target: Vec2<f32>,
        radius: f32,
        base_height: f32,
        max_step_height: f32,
        goal_radius: f32,
    ) -> Option<Vec2<f32>> {
        let cell = (radius * 1.25).clamp(0.2, 0.8);
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
        let sample_height = |c: Vec2<i32>| {
            self.get_floor_height_reachable(cell_center(c), base_height, max_step_height)
                .or_else(|| self.get_floor_height_nearest(cell_center(c), base_height))
                .or_else(|| self.sample_floor_height(cell_center(c), radius * 0.5))
                .unwrap_or(base_height)
        };
        let cell_passable = |c: Vec2<i32>| {
            let pos = cell_center(c);
            let h = sample_height(c);
            !self.is_blocked(Vec3::new(pos.x, h, pos.y), radius)
        };

        if !cell_passable(start_cell)
            || self.is_blocked(Vec3::new(from.x, base_height, from.y), radius)
        {
            return None;
        }

        let successors = |cell_pos: &Vec2<i32>| {
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

            let h0 = sample_height(*cell_pos);
            let p0 = cell_center(*cell_pos);
            dirs.iter()
                .map(|d| *cell_pos + *d)
                .filter(|n| in_bounds(*n) && cell_passable(*n))
                .filter_map(|n| {
                    let h1 = sample_height(n);
                    if (h0 - h1).abs() > max_step_height {
                        return None;
                    }
                    let p1 = cell_center(n);
                    let clear = self.segment_is_clear(
                        Vec3::new(p0.x, h0, p0.y),
                        Vec3::new(p1.x, h1, p1.y),
                        radius,
                    );
                    if !clear {
                        return None;
                    }
                    let is_diag = (n.x - cell_pos.x).abs() == 1 && (n.y - cell_pos.y).abs() == 1;
                    let cost = if is_diag { 14 } else { 10 };
                    Some((n, cost))
                })
                .collect::<Vec<_>>()
        };

        let heuristic = |c: &Vec2<i32>| {
            let p = cell_center(*c);
            let d = (p - target).magnitude() - goal_radius;
            (d.max(0.0) * 10.0) as i32
        };

        let is_goal = |c: &Vec2<i32>| {
            let p = cell_center(*c);
            (p - target).magnitude() <= goal_radius.max(cell)
        };

        let (path, _) = astar(&start_cell, successors, heuristic, is_goal)?;
        if path.len() < 2 {
            return Some(target);
        }
        Some(cell_center(path[1]))
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
