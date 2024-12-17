use crate::prelude::*;
use theframework::prelude::*;

#[derive(PartialEq)]
pub enum MapContext {
    Region,
    Model,
    Screen,
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

    /// The currently selected character in the editor.
    pub curr_character: Option<Uuid>,

    /// The currently selected character instance in the editor.
    pub curr_character_instance: Option<Uuid>,

    /// The currently selected item in the editor.
    pub curr_item: Option<Uuid>,

    /// The currently selected item instance in the editor.
    pub curr_item_instance: Option<Uuid>,

    /// The currently selected area in the editor.
    pub curr_area: Option<Uuid>,

    /// The currently selected codegrid in the code editor.
    pub curr_grid_id: Option<Uuid>,

    /// If the user selects a tile area.
    pub tile_selection: Option<TileSelection>,

    /// The currently selected screen.
    pub curr_screen: Uuid,

    /// The currently selected widget.
    pub curr_widget: Option<Uuid>,

    /// Show the fx marker on the tiles
    pub show_fx_marker: bool,

    /// The logged interactions of the characters.
    pub interactions: FxHashMap<Uuid, Vec<Interaction>>,

    /// The currently selected tile
    pub curr_tile_id: Option<Uuid>,

    /// The currently selected layer role
    pub curr_layer_role: Layer2DRole,

    /// The conceptual display range [0..1] of the 2D preview.
    /// Only relevent in Model view. 0 is full conceptual display. 1 is full detail.
    pub conceptual_display: Option<f32>,

    pub curr_geo_object: Option<Uuid>,
    pub curr_geo_node: Option<Uuid>,

    pub curr_material_object: Option<Uuid>,
    pub curr_brush: Uuid,

    /// The screen editor drawing mode.
    pub screen_editor_mode_foreground: bool,

    /// Hover geometry info
    pub hover: (Option<u32>, Option<u32>, Option<u32>),

    /// The current grid hover position
    pub hover_cursor: Option<Vec2f>,

    /// Current Tool Type
    pub curr_map_tool_type: MapToolType,

    /// Current Map Context
    pub curr_map_context: MapContext,

    /// For map tools, 0 shows tile picker, 1 shows procedural materials
    pub curr_map_material: i32,

    /// Map texture mode
    pub curr_texture_mode: MapTextureMode,
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

            curr_character: None,
            curr_character_instance: None,

            curr_item: None,
            curr_item_instance: None,

            curr_area: None,

            curr_grid_id: None,

            tile_selection: None,

            curr_screen: Uuid::nil(),
            curr_widget: None,

            show_fx_marker: false,

            interactions: FxHashMap::default(),

            curr_tile_id: None,
            curr_layer_role: Layer2DRole::Ground,

            conceptual_display: None,

            curr_geo_object: None,
            curr_geo_node: None,

            curr_material_object: None,
            curr_brush: Uuid::nil(),

            screen_editor_mode_foreground: false,

            hover: (None, None, None),
            hover_cursor: None,

            curr_map_tool_type: MapToolType::Linedef,
            curr_map_context: MapContext::Region,
            curr_map_material: 0,
            curr_texture_mode: MapTextureMode::Floor,
        }
    }

    /// Returns the material or texture for the given mode (Floor, Wall, Ceiling) for the current map selection.
    /// None means nothing selected
    /// Some(None, None) means there is a selection but its ambigous
    pub fn get_texture_for_mode(
        &self,
        mode: MapTextureMode,
        map: &Map,
    ) -> Option<(Option<Uuid>, Option<u8>)> {
        match mode {
            MapTextureMode::Floor => {
                if let Some(sector_id) = map.selected_sectors.first() {
                    if let Some(sector) = map.find_sector(*sector_id) {
                        if let Some(texture_uuid) = sector.floor_texture {
                            return Some((Some(texture_uuid), None));
                        }
                        if let Some(material_index) = sector.floor_material {
                            return Some((None, Some(material_index)));
                        }
                    }
                }
            }
            MapTextureMode::Wall => {}
            MapTextureMode::Ceiling => {
                if let Some(sector_id) = map.selected_sectors.first() {
                    if let Some(sector) = map.find_sector(*sector_id) {
                        if let Some(texture_uuid) = sector.ceiling_texture {
                            return Some((Some(texture_uuid), None));
                        }
                    }
                }
            }
            _ => {}
        }
        None
    }

    pub fn clear(&mut self) {
        self.curr_region = Uuid::nil();
        self.curr_character = None;
        self.curr_character_instance = None;
        self.curr_item = None;
        self.curr_item_instance = None;
        self.curr_area = None;
        self.curr_grid_id = None;
        self.tile_selection = None;
        self.curr_screen = Uuid::nil();
        self.interactions.clear();
    }

    pub fn clear_interactions(&mut self) {
        self.interactions.clear();
    }

    /// Convert local screen position to a map grid position
    pub fn local_to_map_grid(
        &self,
        screen_size: Vec2f,
        coord: Vec2f,
        map: &Map,
        subdivisions: f32,
    ) -> Vec2f {
        let grid_space_pos = coord - screen_size / 2.0 - vec2f(map.offset.x, -map.offset.y);
        let snapped = grid_space_pos / map.grid_size;
        let rounded = round(snapped);

        if subdivisions > 1.0 {
            let subdivision_size = 1.0 / subdivisions;

            // Calculate fractional part of the snapped position
            let fractional = snapped - rounded;

            // Snap the fractional part to the nearest subdivision
            rounded + round(fractional / subdivision_size) * subdivision_size
        } else {
            rounded
        }
    }

    /// Convert a map grid position to a local screen position
    pub fn map_grid_to_local(screen_size: Vec2f, grid_pos: Vec2f, map: &Map) -> Vec2f {
        let grid_space_pos = grid_pos * map.grid_size;
        grid_space_pos + vec2f(map.offset.x, -map.offset.y) + screen_size / 2.0
    }

    /// Centers the map at the given grid position.
    pub fn center_map_at_grid_pos(&mut self, _screen_size: Vec2f, grid_pos: Vec2f, map: &mut Map) {
        let pixel_pos = grid_pos * map.grid_size;
        map.offset.x = -(pixel_pos.x);
        map.offset.y = pixel_pos.y;
    }

    /// Returns the geometry at the given screen_position
    pub fn geometry_at(
        &self,
        screen_size: Vec2f,
        screen_pos: Vec2f,
        map: &Map,
    ) -> (Option<u32>, Option<u32>, Option<u32>) {
        let mut selection: (Option<u32>, Option<u32>, Option<u32>) = (None, None, None);
        let hover_threshold = 6.0;

        // Check the vertices
        for vertex in &map.vertices {
            let vertex_pos = Self::map_grid_to_local(screen_size, vertex.as_vec2f(), map);
            if length(screen_pos - vertex_pos) <= hover_threshold {
                selection.0 = Some(vertex.id);
                //break;
                return selection;
            }
        }

        // Check the lines
        for linedef in &map.linedefs {
            let start_vertex = map.find_vertex(linedef.start_vertex);
            let end_vertex = map.find_vertex(linedef.end_vertex);

            if let Some(start_vertex) = start_vertex {
                if let Some(end_vertex) = end_vertex {
                    let start_pos =
                        Self::map_grid_to_local(screen_size, start_vertex.as_vec2f(), map);
                    let end_pos = Self::map_grid_to_local(screen_size, end_vertex.as_vec2f(), map);

                    // Compute the perpendicular distance from the point to the line
                    let line_vec = end_pos - start_pos;
                    let mouse_vec = screen_pos - start_pos;
                    let line_vec_length = length(line_vec);
                    let projection = dot(mouse_vec, line_vec) / (line_vec_length * line_vec_length);
                    let closest_point = start_pos + projection.clamp(0.0, 1.0) * line_vec;
                    let distance = length(screen_pos - closest_point);

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
        fn point_in_polygon(point: Vec2f, polygon: &[Vec2f]) -> bool {
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

        for sector in &map.sectors {
            let mut vertices = Vec::new();
            for &linedef_id in &sector.linedefs {
                if let Some(linedef) = map.linedefs.get(linedef_id as usize) {
                    if let Some(start_vertex) = map.vertices.get(linedef.start_vertex as usize) {
                        let vertex =
                            Self::map_grid_to_local(screen_size, start_vertex.as_vec2f(), map);

                        // Add the vertex to the list if it isn't already there
                        if vertices.last() != Some(&vertex) {
                            vertices.push(vertex);
                        }
                    }
                }
            }

            if point_in_polygon(screen_pos, &vertices) {
                selection.2 = Some(sector.id);
                // break;
                return selection;
            }
        }

        selection
    }

    /// Returns all geometry within the given rectangle.
    pub fn geometry_in_rectangle(
        &self,
        top_left: Vec2f,
        bottom_right: Vec2f,
        map: &Map,
    ) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
        let mut selection: (Vec<u32>, Vec<u32>, Vec<u32>) = (Vec::new(), Vec::new(), Vec::new());

        // Define helper to check if a point is within the rectangle
        fn point_in_rectangle(point: Vec2f, top_left: Vec2f, bottom_right: Vec2f) -> bool {
            point.x >= top_left.x
                && point.x <= bottom_right.x
                && point.y >= top_left.y
                && point.y <= bottom_right.y
        }

        /// Check if a line segment intersects a rectangle
        fn line_intersects_rectangle(
            a: Vec2f,
            b: Vec2f,
            top_left: Vec2f,
            bottom_right: Vec2f,
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
                (top_left, Vec2f::new(bottom_right.x, top_left.y)), // Top
                (Vec2f::new(bottom_right.x, top_left.y), bottom_right), // Right
                (bottom_right, Vec2f::new(top_left.x, bottom_right.y)), // Bottom
                (Vec2f::new(top_left.x, bottom_right.y), top_left), // Left
            ];

            rect_edges
                .iter()
                .any(|&(p1, p2)| line_segments_intersect(a, b, p1, p2))
        }

        /// Check if two line segments intersect
        fn line_segments_intersect(p1: Vec2f, p2: Vec2f, q1: Vec2f, q2: Vec2f) -> bool {
            fn ccw(a: Vec2f, b: Vec2f, c: Vec2f) -> bool {
                (c.y - a.y) * (b.x - a.x) > (b.y - a.y) * (c.x - a.x)
            }

            ccw(p1, q1, q2) != ccw(p2, q1, q2) && ccw(p1, p2, q1) != ccw(p1, p2, q2)
        }

        // Check vertices
        for vertex in &map.vertices {
            let vertex_pos = vertex.as_vec2f();
            if point_in_rectangle(vertex_pos, top_left, bottom_right) {
                selection.0.push(vertex.id);
            }
        }

        // Check linedefs
        for linedef in &map.linedefs {
            let start_vertex = map.find_vertex(linedef.start_vertex);
            let end_vertex = map.find_vertex(linedef.end_vertex);

            if let (Some(start_vertex), Some(end_vertex)) = (start_vertex, end_vertex) {
                let start_pos = start_vertex.as_vec2f();
                let end_pos = end_vertex.as_vec2f();

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
                if let Some(linedef) = map.linedefs.get(linedef_id as usize) {
                    if let Some(start_vertex) = map.vertices.get(linedef.start_vertex as usize) {
                        let vertex = start_vertex.as_vec2f();

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
