use crate::prelude::*;
use theframework::prelude::*;

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

    // Selection
    pub selected_vertices: Vec<u32>,
    pub selected_linedefs: Vec<u32>,
    pub selected_sectors: Vec<u32>,

    pub hover: (Option<u32>, Option<u32>, Option<u32>),
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

            selected_vertices: vec![],
            selected_linedefs: vec![],
            selected_sectors: vec![],

            hover: (None, None, None),
        }
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
