use crate::prelude::*;
pub use rusterix::map::*;
use theframework::prelude::*;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ContentContext {
    Unknown,
    CharacterInstance(Uuid),
    ItemInstance(Uuid),
    Sector(Uuid),
    CharacterTemplate(Uuid),
    ItemTemplate(Uuid),
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum MapContext {
    Region,
    Model,
    Screen,
    Material,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum MapToolHelper {
    TilePicker,
    MaterialPicker,
    MaterialEditor,
    EffectsPicker,
    Preview,
}

impl MapToolHelper {
    pub fn set_from_index(&mut self, index: usize) {
        match index {
            1 => *self = MapToolHelper::MaterialPicker,
            2 => *self = MapToolHelper::MaterialEditor,
            3 => *self = MapToolHelper::EffectsPicker,
            4 => *self = MapToolHelper::Preview,
            _ => *self = MapToolHelper::TilePicker,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum WorldToolHelper {
    Brushes,
    TilePicker,
    MaterialPicker,
}

impl WorldToolHelper {
    pub fn set_from_index(&mut self, index: usize) {
        match index {
            1 => *self = WorldToolHelper::TilePicker,
            2 => *self = WorldToolHelper::MaterialPicker,
            _ => *self = WorldToolHelper::Brushes,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum WorldToolCamera {
    Orbit,
    FirstP,
}

impl WorldToolCamera {
    pub fn set_from_index(&mut self, index: usize) {
        match index {
            1 => *self = WorldToolCamera::FirstP,
            _ => *self = WorldToolCamera::Orbit,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum MapTextureMode {
    Preview,
    Floor,
    Wall,
    Ceiling,
}

/// This gives context to the server of the editing state for live highlighting.
pub struct ServerContext {
    /// The currently selected region in the editor.
    pub curr_region: Uuid,

    /// The current region content.
    pub curr_region_content: ContentContext,

    /// The currently selected character in the editor.
    pub curr_character: ContentContext,

    /// The currently selected item in the editor.
    pub curr_item: ContentContext,

    /// The current content context.
    pub cc: ContentContext,

    /// The currently selected codegrid in the code editor.
    pub curr_grid_id: Option<Uuid>,

    /// The currently selected screen.
    pub curr_screen: Uuid,

    /// The logged interactions of the characters.
    pub interactions: FxHashMap<Uuid, Vec<Interaction>>,

    /// The currently selected tile
    pub curr_tile_id: Option<Uuid>,

    /// The currently selected Material
    pub curr_material_id: Option<Uuid>,

    /// The conceptual display range [0..1] of the 2D preview.
    /// Only relevent in Model view. 0 is full conceptual display. 1 is full detail.
    pub conceptual_display: Option<f32>,

    pub curr_material: Option<Uuid>,

    pub curr_effect: Option<EffectWrapper>,

    /// The screen editor drawing mode.
    pub screen_editor_mode_foreground: bool,

    /// Hover geometry info
    pub hover: (Option<u32>, Option<u32>, Option<u32>),

    /// The current grid hover position
    pub hover_cursor: Option<Vec2<f32>>,

    /// Current Tool Type
    pub curr_map_tool_type: MapToolType,

    /// Current Map Context
    pub curr_map_context: MapContext,

    /// For map tools, indicates which helper is active
    pub curr_map_tool_helper: MapToolHelper,

    /// For world tools, indicates which helper is active
    pub curr_world_tool_helper: WorldToolHelper,

    /// For world tools, indicates which camera is active
    pub curr_world_tool_camera: WorldToolCamera,

    /// Map texture mode
    pub curr_texture_mode: MapTextureMode,

    /// A click on map content originated from the map
    pub content_click_from_map: bool,

    /// Dont show rect based geometry on map
    pub no_rect_geo_on_map: bool,

    /// Selected wall row, set by the linedef Hud
    pub selected_wall_row: Option<i32>,

    /// World mode is active
    pub world_mode: bool,

    /// Game server is running
    pub game_mode: bool,

    /// Map clipboard
    pub clipboard: Map,

    /// Map clipboard which is currently being pasted
    pub paste_clipboard: Option<Map>,
}

impl Default for ServerContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerContext {
    pub fn new() -> Self {
        Self {
            curr_region: Uuid::nil(),

            curr_region_content: ContentContext::Unknown,
            curr_character: ContentContext::Unknown,
            curr_item: ContentContext::Unknown,
            cc: ContentContext::Unknown,

            curr_grid_id: None,

            curr_screen: Uuid::nil(),

            interactions: FxHashMap::default(),

            curr_tile_id: None,

            curr_material_id: None,
            curr_effect: None,

            conceptual_display: None,

            curr_material: None,

            screen_editor_mode_foreground: false,

            hover: (None, None, None),
            hover_cursor: None,

            curr_map_tool_type: MapToolType::Linedef,
            curr_map_context: MapContext::Region,
            curr_map_tool_helper: MapToolHelper::TilePicker,
            curr_world_tool_helper: WorldToolHelper::Brushes,
            curr_world_tool_camera: WorldToolCamera::Orbit,
            curr_texture_mode: MapTextureMode::Floor,

            content_click_from_map: false,
            no_rect_geo_on_map: true,

            selected_wall_row: Some(0),

            world_mode: false,
            game_mode: false,

            clipboard: Map::default(),
            paste_clipboard: None,
        }
    }

    /// Clears all state data.
    pub fn clear(&mut self) {
        self.curr_region_content = ContentContext::Unknown;
        self.curr_character = ContentContext::Unknown;
        self.curr_item = ContentContext::Unknown;
        self.cc = ContentContext::Unknown;

        self.curr_region = Uuid::nil();
        self.curr_grid_id = None;
        self.curr_screen = Uuid::nil();
        self.interactions.clear();
    }

    pub fn clear_interactions(&mut self) {
        self.interactions.clear();
    }

    /// Convert local screen position to a map grid position
    pub fn local_to_map_grid(
        &self,
        screen_size: Vec2<f32>,
        coord: Vec2<f32>,
        map: &Map,
        subdivisions: f32,
    ) -> Vec2<f32> {
        let grid_space_pos = coord - screen_size / 2.0 - Vec2::new(map.offset.x, -map.offset.y);
        let snapped = grid_space_pos / map.grid_size;
        let rounded = snapped.map(|x| x.round());

        if subdivisions > 1.0 {
            let subdivision_size = 1.0 / subdivisions;
            // Calculate fractional part of the snapped position
            let fractional = snapped - rounded;
            // Snap the fractional part to the nearest subdivision
            rounded + fractional.map(|x| (x / subdivision_size).round() * subdivision_size)
        } else {
            rounded
        }
    }

    /// Snap to a grid cell
    pub fn local_to_map_cell(
        &self,
        screen_size: Vec2<f32>,
        coord: Vec2<f32>,
        map: &Map,
        subdivisions: f32,
    ) -> Vec2<f32> {
        let grid_space_pos = coord - screen_size / 2.0 - Vec2::new(map.offset.x, -map.offset.y);
        let grid_cell = (grid_space_pos / map.grid_size).map(|x| x.floor());

        if subdivisions > 1.0 {
            let sub_cell_size = map.grid_size / subdivisions;
            let sub_index = grid_space_pos
                .map(|x| x.rem_euclid(map.grid_size) / sub_cell_size)
                .map(|x| x.floor());
            grid_cell + sub_index / subdivisions
        } else {
            grid_cell
        }
    }

    /// Convert a map grid position to a local screen position
    pub fn map_grid_to_local(screen_size: Vec2<f32>, grid_pos: Vec2<f32>, map: &Map) -> Vec2<f32> {
        let grid_space_pos = grid_pos * map.grid_size;
        grid_space_pos + Vec2::new(map.offset.x, -map.offset.y) + screen_size / 2.0
    }

    /// Centers the map at the given grid position.
    pub fn center_map_at_grid_pos(
        &mut self,
        _screen_size: Vec2<f32>,
        grid_pos: Vec2<f32>,
        map: &mut Map,
    ) {
        let pixel_pos = grid_pos * map.grid_size;
        map.offset.x = -(pixel_pos.x);
        map.offset.y = pixel_pos.y;
    }

    /// Returns the geometry at the given screen_position
    pub fn geometry_at(
        &self,
        screen_size: Vec2<f32>,
        screen_pos: Vec2<f32>,
        map: &Map,
    ) -> (Option<u32>, Option<u32>, Option<u32>) {
        let mut selection: (Option<u32>, Option<u32>, Option<u32>) = (None, None, None);
        let hover_threshold = 6.0;

        // Check the vertices
        for vertex in &map.vertices {
            if let Some(vertex_pos) = map.get_vertex(vertex.id) {
                let vertex_pos = Self::map_grid_to_local(screen_size, vertex_pos, map);
                if (screen_pos - vertex_pos).magnitude() <= hover_threshold {
                    selection.0 = Some(vertex.id);
                    //break;
                    return selection;
                }
            }
        }

        // Check the lines
        for linedef in &map.linedefs {
            if self.no_rect_geo_on_map && map.is_linedef_in_rect(linedef.id) {
                continue;
            }

            let start_vertex = map.get_vertex(linedef.start_vertex);
            let end_vertex = map.get_vertex(linedef.end_vertex);

            if let Some(start_vertex) = start_vertex {
                if let Some(end_vertex) = end_vertex {
                    let start_pos = Self::map_grid_to_local(screen_size, start_vertex, map);
                    let end_pos = Self::map_grid_to_local(screen_size, end_vertex, map);

                    // Compute the perpendicular distance from the point to the line
                    let line_vec = end_pos - start_pos;
                    let mouse_vec = screen_pos - start_pos;
                    let line_vec_length = line_vec.magnitude();
                    let projection = mouse_vec.dot(line_vec) / (line_vec_length * line_vec_length);
                    let closest_point = start_pos + projection.clamp(0.0, 1.0) * line_vec;
                    let distance = (screen_pos - closest_point).magnitude();

                    if distance <= hover_threshold {
                        selection.1 = Some(linedef.id);
                        //break;
                        return selection;
                    }
                }
            }
        }

        // Check the sectors

        /// Point-in-polygon test using the ray-casting method
        fn point_in_polygon(point: Vec2<f32>, polygon: &[Vec2<f32>]) -> bool {
            let mut inside = false;
            let mut j = polygon.len() - 1;

            for i in 0..polygon.len() {
                if (polygon[i].y > point.y) != (polygon[j].y > point.y)
                    && point.x
                        < (polygon[j].x - polygon[i].x) * (point.y - polygon[i].y)
                            / (polygon[j].y - polygon[i].y)
                            + polygon[i].x
                {
                    inside = !inside;
                }
                j = i;
            }

            inside
        }

        // Reverse on sorted sectors by area (to allow to pick small sectors first)
        let ordered = map.sorted_sectors_by_area();
        for sector in ordered.iter().rev() {
            if self.no_rect_geo_on_map && sector.properties.contains("rect_rendering") {
                continue;
            }
            let mut vertices = Vec::new();
            for &linedef_id in &sector.linedefs {
                if let Some(linedef) = map.find_linedef(linedef_id) {
                    if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                        let vertex =
                            Self::map_grid_to_local(screen_size, start_vertex.as_vec2(), map);

                        // Add the vertex to the list if it isn't already there
                        if vertices.last() != Some(&vertex) {
                            vertices.push(vertex);
                        }
                    }
                }
            }

            if point_in_polygon(screen_pos, &vertices) {
                selection.2 = Some(sector.id);
                return selection;
            }
        }

        selection
    }

    /// Returns all geometry within the given rectangle.
    pub fn geometry_in_rectangle(
        &self,
        top_left: Vec2<f32>,
        bottom_right: Vec2<f32>,
        map: &Map,
    ) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
        let mut selection: (Vec<u32>, Vec<u32>, Vec<u32>) = (Vec::new(), Vec::new(), Vec::new());

        // Define helper to check if a point is within the rectangle
        fn point_in_rectangle(
            point: Vec2<f32>,
            top_left: Vec2<f32>,
            bottom_right: Vec2<f32>,
        ) -> bool {
            point.x >= top_left.x
                && point.x <= bottom_right.x
                && point.y >= top_left.y
                && point.y <= bottom_right.y
        }

        /// Check if a line segment intersects a rectangle
        fn line_intersects_rectangle(
            a: Vec2<f32>,
            b: Vec2<f32>,
            top_left: Vec2<f32>,
            bottom_right: Vec2<f32>,
        ) -> bool {
            // fn between(x: f32, min: f32, max: f32) -> bool {
            //     x >= min && x <= max
            // }

            // Axis-Aligned Bounding Box (AABB) test for the line segment
            let (min_x, max_x) = (a.x.min(b.x), a.x.max(b.x));
            let (min_y, max_y) = (a.y.min(b.y), a.y.max(b.y));

            if min_x > bottom_right.x
                || max_x < top_left.x
                || min_y > bottom_right.y
                || max_y < top_left.y
            {
                return false; // Line is outside the rectangle
            }

            // Check if either endpoint is inside the rectangle
            if point_in_rectangle(a, top_left, bottom_right)
                || point_in_rectangle(b, top_left, bottom_right)
            {
                return true;
            }

            // Check edge intersections
            let rect_edges = [
                (top_left, Vec2::new(bottom_right.x, top_left.y)), // Top
                (Vec2::new(bottom_right.x, top_left.y), bottom_right), // Right
                (bottom_right, Vec2::new(top_left.x, bottom_right.y)), // Bottom
                (Vec2::new(top_left.x, bottom_right.y), top_left), // Left
            ];

            rect_edges
                .iter()
                .any(|&(p1, p2)| line_segments_intersect(a, b, p1, p2))
        }

        /// Check if two line segments intersect
        fn line_segments_intersect(
            p1: Vec2<f32>,
            p2: Vec2<f32>,
            q1: Vec2<f32>,
            q2: Vec2<f32>,
        ) -> bool {
            fn ccw(a: Vec2<f32>, b: Vec2<f32>, c: Vec2<f32>) -> bool {
                (c.y - a.y) * (b.x - a.x) > (b.y - a.y) * (c.x - a.x)
            }

            ccw(p1, q1, q2) != ccw(p2, q1, q2) && ccw(p1, p2, q1) != ccw(p1, p2, q2)
        }

        // Check vertices
        for vertex in &map.vertices {
            if let Some(vertex_pos) = map.get_vertex(vertex.id) {
                if point_in_rectangle(vertex_pos, top_left, bottom_right) {
                    selection.0.push(vertex.id);
                }
            }
        }

        // Check linedefs
        for linedef in &map.linedefs {
            let start_vertex = map.get_vertex(linedef.start_vertex);
            let end_vertex = map.get_vertex(linedef.end_vertex);

            if let (Some(start_vertex), Some(end_vertex)) = (start_vertex, end_vertex) {
                let start_pos = start_vertex;
                let end_pos = end_vertex;

                // Check if either endpoint is inside the rectangle
                if point_in_rectangle(start_pos, top_left, bottom_right)
                    || point_in_rectangle(end_pos, top_left, bottom_right)
                {
                    selection.1.push(linedef.id);
                }
            }
        }

        // Check sectors
        // fn point_in_polygon(point: Vec2f, polygon: &[Vec2f]) -> bool {
        //     let mut inside = false;
        //     let mut j = polygon.len() - 1;

        //     for i in 0..polygon.len() {
        //         if (polygon[i].y > point.y) != (polygon[j].y > point.y)
        //             && point.x
        //                 < (polygon[j].x - polygon[i].x) * (point.y - polygon[i].y)
        //                     / (polygon[j].y - polygon[i].y)
        //                     + polygon[i].x
        //         {
        //             inside = !inside;
        //         }
        //         j = i;
        //     }

        //     inside
        // }

        for sector in &map.sectors {
            let mut vertices = Vec::new();
            for &linedef_id in &sector.linedefs {
                if let Some(linedef) = map.find_linedef(linedef_id) {
                    if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                        let vertex = start_vertex.as_vec2();

                        // Add the vertex to the list if it isn't already there
                        if vertices.last() != Some(&vertex) {
                            vertices.push(vertex);
                        }
                    }
                }
            }

            // Check if any part of the sector polygon is in the rectangle
            if vertices
                .iter()
                .any(|v| point_in_rectangle(*v, top_left, bottom_right))
                || vertices.windows(2).any(|pair| {
                    // For edges, check if they intersect the rectangle
                    let (a, b) = (pair[0], pair[1]);
                    line_intersects_rectangle(a, b, top_left, bottom_right)
                })
            {
                selection.2.push(sector.id);
            }
        }

        selection
    }

    /// Returns false if the hover is empty
    pub fn hover_is_empty(&self) -> bool {
        self.hover.0.is_none() && self.hover.1.is_none() && self.hover.2.is_none()
    }

    /// Converts the hover into arrays.
    pub fn hover_to_arrays(&self) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
        let mut arrays: (Vec<u32>, Vec<u32>, Vec<u32>) = (vec![], vec![], vec![]);
        if let Some(v) = self.hover.0 {
            arrays.0.push(v);
        }
        if let Some(l) = self.hover.1 {
            arrays.1.push(l);
        }
        if let Some(s) = self.hover.2 {
            arrays.2.push(s);
        }
        arrays
    }

    /// Adds the given interactions provided by a server tick to the context.
    pub fn add_interactions(&mut self, interactions: Vec<Interaction>) {
        for interaction in interactions {
            if let Some(interactions) = self.interactions.get_mut(&interaction.to) {
                interactions.push(interaction);
            } else {
                self.interactions.insert(interaction.to, vec![interaction]);
            }
        }
    }
}
