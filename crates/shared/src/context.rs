use crate::prelude::*;
use rusterix::Surface;
pub use rusterix::{Value, VertexBlendPreset, map::*};
use scenevm::GeoId;
use theframework::prelude::*;

/// Identifies which texture is currently being edited by editor tools.
/// This abstraction allows the same draw/fill/pick tools to work on
/// any paintable texture source (tiles, avatar frames, future types).
/// Each variant also determines how colors are resolved for painting.
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum PixelEditingContext {
    /// No active editing target.
    None,
    /// Editing a tile texture: (tile_id, frame_index).
    Tile(Uuid, usize),
    /// Editing an avatar animation frame: (avatar_id, anim_id, perspective_index, frame_index).
    AvatarFrame(Uuid, Uuid, usize, usize),
}

impl Default for PixelEditingContext {
    fn default() -> Self {
        Self::None
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum AvatarAnchorEditSlot {
    None,
    Main,
    Off,
}

impl Default for AvatarAnchorEditSlot {
    fn default() -> Self {
        Self::None
    }
}

impl PixelEditingContext {
    /// Returns the [r, g, b, a] color to paint with for this context.
    /// For tiles: uses the palette color with opacity applied.
    /// For avatar frames: uses the selected body marker color.
    pub fn get_draw_color(
        &self,
        palette: &ThePalette,
        opacity: f32,
        body_marker_color: Option<[u8; 4]>,
    ) -> Option<[u8; 4]> {
        match self {
            PixelEditingContext::None => None,
            PixelEditingContext::Tile(..) => {
                if let Some(color) = palette.get_current_color() {
                    let mut arr = color.to_u8_array();
                    arr[3] = (arr[3] as f32 * opacity) as u8;
                    Some(arr)
                } else {
                    None
                }
            }
            PixelEditingContext::AvatarFrame(..) => body_marker_color,
        }
    }

    /// Returns the number of animation frames for this context.
    pub fn get_frame_count(&self, project: &Project) -> usize {
        match self {
            PixelEditingContext::None => 0,
            PixelEditingContext::Tile(tile_id, _) => {
                project.tiles.get(tile_id).map_or(0, |t| t.textures.len())
            }
            PixelEditingContext::AvatarFrame(avatar_id, anim_id, persp_index, _) => project
                .avatars
                .get(avatar_id)
                .and_then(|a| a.animations.iter().find(|anim| anim.id == *anim_id))
                .and_then(|anim| anim.perspectives.get(*persp_index))
                .map_or(0, |p| p.frames.len()),
        }
    }

    /// Returns a copy of this context with the frame index replaced.
    pub fn with_frame(&self, frame_index: usize) -> Self {
        match *self {
            PixelEditingContext::None => PixelEditingContext::None,
            PixelEditingContext::Tile(tile_id, _) => {
                PixelEditingContext::Tile(tile_id, frame_index)
            }
            PixelEditingContext::AvatarFrame(avatar_id, anim_id, persp_index, _) => {
                PixelEditingContext::AvatarFrame(avatar_id, anim_id, persp_index, frame_index)
            }
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum GizmoMode {
    XZ,
    XY,
    YZ,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum EditorViewMode {
    D2,
    Orbit,
    Iso,
    FirstP,
}

impl EditorViewMode {
    pub fn to_index(&self) -> i32 {
        match self {
            EditorViewMode::D2 => 0,
            EditorViewMode::Orbit => 1,
            EditorViewMode::Iso => 2,
            EditorViewMode::FirstP => 3,
        }
    }
    pub fn from_index(idx: i32) -> Self {
        match idx {
            1 => EditorViewMode::Orbit,
            2 => EditorViewMode::Iso,
            3 => EditorViewMode::FirstP,
            _ => EditorViewMode::D2,
        }
    }

    pub fn is_3d(&self) -> bool {
        match self {
            EditorViewMode::D2 => false,
            _ => true,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ContentContext {
    Unknown,
    CharacterInstance(Uuid),
    ItemInstance(Uuid),
    Sector(Uuid),
    CharacterTemplate(Uuid),
    ItemTemplate(Uuid),
    Shader(Uuid),
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ProjectContext {
    Unknown,
    Region(Uuid),
    RegionSettings(Uuid),
    RegionCharacterInstance(Uuid, Uuid),
    RegionItemInstance(Uuid, Uuid),
    Character(Uuid),
    CharacterVisualCode(Uuid),
    CharacterCode(Uuid),
    CharacterData(Uuid),
    CharacterPreviewRigging(Uuid),
    Item(Uuid),
    ItemVisualCode(Uuid),
    ItemCode(Uuid),
    ItemData(Uuid),
    Tilemap(Uuid),
    Screen(Uuid),
    ScreenWidget(Uuid, Uuid),
    Asset(Uuid),
    Avatar(Uuid),
    AvatarAnimation(Uuid, Uuid, usize),
    ProjectSettings,
    DebugLog,
}

impl ProjectContext {
    pub fn id(self) -> Option<Uuid> {
        match self {
            ProjectContext::Unknown
            | ProjectContext::ProjectSettings
            | ProjectContext::DebugLog => None,
            ProjectContext::Region(id)
            | ProjectContext::RegionSettings(id)
            | ProjectContext::RegionCharacterInstance(id, _)
            | ProjectContext::RegionItemInstance(id, _)
            | ProjectContext::Character(id)
            | ProjectContext::CharacterVisualCode(id)
            | ProjectContext::CharacterCode(id)
            | ProjectContext::CharacterData(id)
            | ProjectContext::CharacterPreviewRigging(id)
            | ProjectContext::Item(id)
            | ProjectContext::ItemVisualCode(id)
            | ProjectContext::ItemCode(id)
            | ProjectContext::ItemData(id)
            | ProjectContext::Tilemap(id)
            | ProjectContext::Screen(id)
            | ProjectContext::ScreenWidget(id, _)
            | ProjectContext::Asset(id)
            | ProjectContext::Avatar(id)
            | ProjectContext::AvatarAnimation(id, _, _) => Some(id),
        }
    }

    pub fn is_region(&self) -> bool {
        match self {
            ProjectContext::Region(_)
            | ProjectContext::RegionSettings(_)
            | ProjectContext::RegionCharacterInstance(_, _)
            | ProjectContext::RegionItemInstance(_, _) => true,
            _ => false,
        }
    }

    pub fn get_region_character_instance_id(&self) -> Option<Uuid> {
        match self {
            ProjectContext::RegionCharacterInstance(_, instance_id) => Some(*instance_id),
            _ => None,
        }
    }

    pub fn get_region_item_instance_id(&self) -> Option<Uuid> {
        match self {
            ProjectContext::RegionItemInstance(_, instance_id) => Some(*instance_id),
            _ => None,
        }
    }

    pub fn is_character(&self) -> bool {
        match self {
            ProjectContext::Character(_)
            | ProjectContext::CharacterVisualCode(_)
            | ProjectContext::CharacterCode(_)
            | ProjectContext::CharacterData(_)
            | ProjectContext::CharacterPreviewRigging(_) => true,
            _ => false,
        }
    }

    pub fn is_item(&self) -> bool {
        match self {
            ProjectContext::Item(_)
            | ProjectContext::ItemVisualCode(_)
            | ProjectContext::ItemCode(_)
            | ProjectContext::ItemData(_) => true,
            _ => false,
        }
    }

    pub fn is_tilemap(&self) -> bool {
        match self {
            ProjectContext::Tilemap(_) => true,
            _ => false,
        }
    }

    pub fn is_screen(&self) -> bool {
        match self {
            ProjectContext::Screen(_) | ProjectContext::ScreenWidget(_, _) => true,
            _ => false,
        }
    }

    pub fn get_screen_widget_id(&self) -> Option<Uuid> {
        match self {
            ProjectContext::ScreenWidget(_, widget_id) => Some(*widget_id),
            _ => None,
        }
    }

    pub fn is_asset(&self) -> bool {
        match self {
            ProjectContext::Asset(_) => true,
            _ => false,
        }
    }

    pub fn is_project_settings(&self) -> bool {
        match self {
            ProjectContext::ProjectSettings => true,
            _ => false,
        }
    }

    pub fn has_custom_map(&self) -> bool {
        match self {
            ProjectContext::Screen(_) => true,
            _ => false,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum MapContext {
    Region,
    Screen,
    Model,
    Shader,
    Character,
    Item,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum MapToolHelper {
    TilePicker,
    NodeEditor,
    ShaderEditor,
    ShapePicker,
}

impl MapToolHelper {
    pub fn set_from_index(&mut self, index: usize) {
        match index {
            1 => *self = MapToolHelper::NodeEditor,
            2 => *self = MapToolHelper::ShaderEditor,
            3 => *self = MapToolHelper::ShapePicker,
            _ => *self = MapToolHelper::TilePicker,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum RenderToolHelper {
    GlobalRender,
    LocalRender,
    Tracer,
}

impl RenderToolHelper {
    pub fn set_from_index(&mut self, index: usize) {
        match index {
            // 1 => *self = RenderToolHelper::LocalRender,
            1 => *self = RenderToolHelper::Tracer,
            _ => *self = RenderToolHelper::GlobalRender,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum WorldToolHelper {
    Brushes,
    TilePicker,
    MaterialPicker,
    GlobalRender,
}

impl WorldToolHelper {
    pub fn set_from_index(&mut self, index: usize) {
        match index {
            1 => *self = WorldToolHelper::TilePicker,
            2 => *self = WorldToolHelper::MaterialPicker,
            3 => *self = WorldToolHelper::GlobalRender,
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
    // Tree Ids
    pub tree_regions_id: Uuid,
    pub tree_characters_id: Uuid,
    pub tree_items_id: Uuid,
    pub tree_tilemaps_id: Uuid,
    pub tree_screens_id: Uuid,
    pub tree_avatars_id: Uuid,
    pub tree_assets_id: Uuid,
    pub tree_assets_fonts_id: Uuid,
    pub tree_palette_id: Uuid,
    pub tree_settings_id: Uuid,

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

    /// The current project context.
    pub pc: ProjectContext,

    /// The currently selected codegrid in the code editor.
    pub curr_grid_id: Option<Uuid>,

    /// The currently selected screen.
    pub curr_screen: Uuid,

    /// The logged interactions of the characters.
    pub interactions: FxHashMap<Uuid, Vec<Interaction>>,

    /// The currently selected tile
    pub curr_tile_id: Option<Uuid>,

    /// The current frame/texture index being edited in tile editor
    pub curr_tile_frame_index: usize,

    /// The palette opacity for drawing tools
    pub palette_opacity: f32,

    /// The currently selected model
    pub curr_model_id: Option<Uuid>,

    /// The currently selected material
    pub curr_material_id: Option<Uuid>,

    pub curr_effect: Option<EffectWrapper>,

    /// The screen editor drawing mode.
    pub screen_editor_mode_foreground: bool,

    /// Hover geometry info
    pub hover: (Option<u32>, Option<u32>, Option<u32>),

    /// The current grid hover position
    pub hover_cursor: Option<Vec2<f32>>,

    /// The current 3d hover position
    pub hover_cursor_3d: Option<Vec3<f32>>,

    /// The current grid hover height
    pub hover_height: Option<f32>,

    /// Current Tool Type
    pub curr_map_tool_type: MapToolType,

    /// Current Map Context
    curr_map_context: MapContext,

    /// For map tools, indicates which helper is active
    pub curr_map_tool_helper: MapToolHelper,

    /// For render tools, indicates which helper is active
    pub curr_render_tool_helper: RenderToolHelper,

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

    /// Show wall profile in linedef mode.
    pub profile_view: Option<u32>,

    /// The current editing surface
    pub editing_surface: Option<Surface>,

    /// The currently selected action
    pub curr_action_id: Option<Uuid>,

    /// Automatially apply actions
    pub auto_action: bool,

    /// Pending entity position changes: (from, to)
    pub moved_entities: FxHashMap<Uuid, (Vec3<f32>, Vec3<f32>)>,

    /// Pending item position changes: (from, to)
    pub moved_items: FxHashMap<Uuid, (Vec3<f32>, Vec3<f32>)>,

    /// Selected wall row, set by the linedef Hud
    pub selected_wall_row: Option<i32>,

    /// View mode of the editor
    pub editor_view_mode: EditorViewMode,

    /// World mode is active
    pub world_mode: bool,

    /// Game server is running
    pub game_mode: bool,

    /// Map clipboard
    pub clipboard: Map,

    /// Map clipboard which is currently being pasted
    pub paste_clipboard: Option<Map>,

    /// Background Progress Text
    pub background_progress: Option<String>,

    /// Character Region Override
    pub character_region_override: bool,

    /// Item Region Override
    pub item_region_override: bool,

    /// Animation counter for tile previews
    pub animation_counter: usize,

    /// The current 3D hover hit
    pub geo_hit: Option<GeoId>,

    /// The current geometry hover hit position
    pub geo_hit_pos: Vec3<f32>,

    /// Temporary storage for the editing positon
    pub editing_pos_buffer: Option<Vec3<f32>>,

    /// The index of the selected icon in the hud
    pub selected_hud_icon_index: i32,

    ///Switch for showing 3D editing geometry
    pub show_editing_geometry: bool,

    /// Position of the 2D editing slice.
    pub editing_slice: f32,

    /// The current plane for 3D movement
    pub gizmo_mode: GizmoMode,

    // For the Rect tool, identify the current sector and tile for preview
    pub rect_sector_id_3d: Option<u32>,
    pub rect_tile_id_3d: (i32, i32),
    pub rect_terrain_id: Option<(i32, i32)>,
    pub rect_blend_preset: VertexBlendPreset,

    /// Game input mode
    pub game_input_mode: bool,

    /// Help mode state
    pub help_mode: bool,

    /// The current editing context for texture editor tools.
    pub editing_ctx: PixelEditingContext,

    /// The selected body marker color for avatar painting.
    pub body_marker_color: Option<[u8; 4]>,
    /// Active avatar anchor edit slot for pixel editor clicks.
    pub avatar_anchor_slot: AvatarAnchorEditSlot,
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

            tree_regions_id: Uuid::new_v4(),
            tree_characters_id: Uuid::new_v4(),
            tree_items_id: Uuid::new_v4(),
            tree_tilemaps_id: Uuid::new_v4(),
            tree_screens_id: Uuid::new_v4(),
            tree_avatars_id: Uuid::new_v4(),
            tree_assets_id: Uuid::new_v4(),
            tree_assets_fonts_id: Uuid::new_v4(),
            tree_palette_id: Uuid::new_v4(),
            tree_settings_id: Uuid::new_v4(),

            curr_region_content: ContentContext::Unknown,
            curr_character: ContentContext::Unknown,
            curr_item: ContentContext::Unknown,
            cc: ContentContext::Unknown,

            pc: ProjectContext::Unknown,

            curr_grid_id: None,

            curr_screen: Uuid::nil(),

            interactions: FxHashMap::default(),

            curr_tile_id: None,
            curr_tile_frame_index: 0,

            palette_opacity: 1.0,

            curr_model_id: None,
            curr_material_id: None,

            curr_effect: None,

            screen_editor_mode_foreground: false,

            hover: (None, None, None),
            hover_cursor: None,
            hover_cursor_3d: None,
            hover_height: None,

            curr_map_tool_type: MapToolType::Linedef,
            curr_map_context: MapContext::Region,
            curr_map_tool_helper: MapToolHelper::TilePicker,
            curr_render_tool_helper: RenderToolHelper::GlobalRender,
            curr_world_tool_helper: WorldToolHelper::Brushes,
            curr_world_tool_camera: WorldToolCamera::Orbit,
            curr_texture_mode: MapTextureMode::Floor,

            content_click_from_map: false,
            no_rect_geo_on_map: true,
            profile_view: None,

            editing_surface: None,
            curr_action_id: None,
            auto_action: true,

            moved_entities: FxHashMap::default(),
            moved_items: FxHashMap::default(),

            selected_wall_row: Some(0),

            editor_view_mode: EditorViewMode::D2,

            world_mode: false,
            game_mode: false,

            clipboard: Map::default(),
            paste_clipboard: None,

            background_progress: None,

            character_region_override: true,
            item_region_override: true,

            animation_counter: 0,

            geo_hit: None,
            geo_hit_pos: Vec3::zero(),

            editing_pos_buffer: None,

            selected_hud_icon_index: 0,
            show_editing_geometry: true,

            gizmo_mode: GizmoMode::XZ,

            editing_slice: 0.0,
            rect_sector_id_3d: None,
            rect_tile_id_3d: (0, 0),
            rect_terrain_id: None,
            rect_blend_preset: VertexBlendPreset::Solid,

            game_input_mode: false,
            help_mode: false,

            editing_ctx: PixelEditingContext::None,
            body_marker_color: None,
            avatar_anchor_slot: AvatarAnchorEditSlot::None,
        }
    }

    /// Checks if the PolyView has focus.
    pub fn polyview_has_focus(&self, ctx: &TheContext) -> bool {
        if let Some(focus) = &ctx.ui.focus {
            if focus.name == "PolyView" {
                return true;
            }
        }
        false
    }

    /// Returns the current map context
    pub fn get_map_context(&self) -> MapContext {
        if self.pc.is_screen() {
            return MapContext::Screen;
        }

        MapContext::Region

        /*
        if (self.curr_map_context == MapContext::Character && self.character_region_override)
            || (self.curr_map_context == MapContext::Item && self.item_region_override)
            || (self.curr_map_context == MapContext::Shader)
        {
            MapContext::Region
        } else {
            self.curr_map_context
        }*/
    }

    pub fn set_map_context(&mut self, map_context: MapContext) {
        self.curr_map_context = map_context;
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
        self.moved_entities.clear();
        self.moved_items.clear();
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
            if vertex.intersects_vertical_slice(self.editing_slice, 1.0) {
                if let Some(vertex_pos) = map.get_vertex(vertex.id) {
                    let vertex_pos = Self::map_grid_to_local(screen_size, vertex_pos, map);
                    if (screen_pos - vertex_pos).magnitude() <= hover_threshold {
                        selection.0 = Some(vertex.id);
                        break;
                    }
                }
            }
        }

        // Check the lines
        for linedef in &map.linedefs {
            if linedef.intersects_vertical_slice(map, self.editing_slice, 1.0) {
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
                        let projection =
                            mouse_vec.dot(line_vec) / (line_vec_length * line_vec_length);
                        let closest_point = start_pos + projection.clamp(0.0, 1.0) * line_vec;
                        let distance = (screen_pos - closest_point).magnitude();

                        if distance <= hover_threshold {
                            selection.1 = Some(linedef.id);
                            break;
                        }
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
            if sector.intersects_vertical_slice(map, self.editing_slice, 1.0) {
                if self.no_rect_geo_on_map
                    && sector.properties.contains("rect")
                    && sector.name.is_empty()
                {
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
            if vertex.intersects_vertical_slice(self.editing_slice, 1.0) {
                if let Some(vertex_pos) = map.get_vertex(vertex.id) {
                    if point_in_rectangle(vertex_pos, top_left, bottom_right) {
                        selection.0.push(vertex.id);
                    }
                }
            }
        }

        // Check linedefs
        for linedef in &map.linedefs {
            if linedef.intersects_vertical_slice(map, self.editing_slice, 1.0) {
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
            if sector.intersects_vertical_slice(map, self.editing_slice, 1.0) {
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
