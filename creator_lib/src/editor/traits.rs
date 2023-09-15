use crate::prelude::*;

#[allow(unused)]
pub trait EditorOptions: Sync + Send {
    fn new(
        _text: Vec<String>,
        rect: (usize, usize, usize, usize),
        asset: &Asset,
        context: &ScreenContext,
    ) -> Self
    where
        Self: Sized;

    fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext);

    fn draw(
        &mut self,
        frame: &mut [u8],
        anim_counter: usize,
        asset: &mut Asset,
        context: &mut ScreenContext,
        content: &mut Option<Box<dyn EditorContent>>,
    );

    fn mouse_down(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        content: &mut Option<Box<dyn EditorContent>>,
        toolbar: &mut Option<&mut ToolBar>,
    ) -> bool;

    fn mouse_up(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        content: &mut Option<Box<dyn EditorContent>>,
    ) -> bool;

    fn mouse_dragged(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        content: &mut Option<Box<dyn EditorContent>>,
    ) -> bool;

    fn mouse_wheel(
        &mut self,
        delta: (isize, isize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        content: &mut Option<Box<dyn EditorContent>>,
    ) -> bool {
        false
    }

    fn mouse_hover(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        content: &mut Option<Box<dyn EditorContent>>,
    ) -> bool {
        false
    }

    /// Updates a value from the dialog
    fn update_from_dialog(
        &mut self,
        id: (Uuid, Uuid, String),
        value: Value,
        asset: &mut Asset,
        context: &mut ScreenContext,
        content: &mut Option<Box<dyn EditorContent>>,
    ) {
    }

    // Sets the state of the atom widgets
    fn set_state(&mut self, state: WidgetState) {}

    /// Options are opening
    fn opening(
        &mut self,
        asset: &mut Asset,
        context: &mut ScreenContext,
        content: &mut Option<Box<dyn EditorContent>>,
    ) {
    }

    /// Options are closing
    fn closing(
        &mut self,
        asset: &mut Asset,
        context: &mut ScreenContext,
        content: &mut Option<Box<dyn EditorContent>>,
    ) {
    }

    // For TilemapOptions

    /// Updates the group widget based on the selected tile
    fn adjust_tile_usage(&mut self, asset: &Asset, context: &ScreenContext) {}

    /// Sets the tile anim for the current tile
    fn set_anim(&mut self, asset: &mut Asset, context: &ScreenContext) {}

    /// Clears the tile anim for the current tile
    fn clear_anim(&mut self, asset: &mut Asset, context: &ScreenContext) {}

    /// Sets the default tile for the current map
    fn set_default_tile(&mut self, asset: &mut Asset, context: &ScreenContext) {}

    /// Set the tile settings
    fn set_tile_settings(
        &mut self,
        open_editor: bool,
        asset: &mut Asset,
        context: &mut ScreenContext,
    ) {
    }

    // For RegionOptions

    /// Return and set the current region editor mode
    fn get_editor_mode(&self) -> RegionEditorMode {
        RegionEditorMode::Tiles
    }
    fn set_editor_mode(&mut self, mode: RegionEditorMode) {}

    /// Get the current tile usage
    fn get_tile_usage(&self) -> Vec<TileUsage> {
        vec![]
    }

    /// Get the current tile_id if any
    fn get_tilemap_index(&self) -> Option<usize> {
        None
    }

    /// Get the current tags
    fn get_tags(&self) -> Option<String> {
        None
    }

    /// Get the current layer
    fn get_layer(&self) -> usize {
        0
    }

    /// Set the current layer
    fn set_layer(&mut self, layer: usize) {}

    // For ScreenOptions

    /// Update the ui
    fn update_ui(
        &mut self,
        context: &mut ScreenContext,
        content: &mut Option<Box<dyn EditorContent>>,
    ) {
    }

    /// Set the name of the widget
    fn set_widget_name(
        &mut self,
        name: String,
        context: &mut ScreenContext,
        content: &mut Option<Box<dyn EditorContent>>,
    ) {
    }

    /// Returns the current screen editor mode
    fn get_screen_editor_mode(&self) -> super::screeneditor_options::ScreenEditorMode {
        super::screeneditor_options::ScreenEditorMode::Scripts
    }

    fn set_script_names(&mut self, scripts: Vec<&String>, index: usize) {}
}

#[derive(PartialEq)]
pub enum GraphMode {
    Overview,
    Detail,
}

#[allow(unused)]
pub trait EditorContent: Sync + Send {
    fn new(
        _text: Vec<String>,
        rect: (usize, usize, usize, usize),
        behavior_type: BehaviorType,
        asset: &Asset,
        context: &ScreenContext,
    ) -> Self
    where
        Self: Sized;

    fn resize(&mut self, width: usize, height: usize, _context: &mut ScreenContext);

    fn draw(
        &mut self,
        frame: &mut [u8],
        anim_counter: usize,
        asset: &mut Asset,
        context: &mut ScreenContext,
        options: &mut Option<Box<dyn EditorOptions>>,
    );

    fn mouse_down(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        options: &mut Option<Box<dyn EditorOptions>>,
        toolbar: &mut Option<&mut ToolBar>,
    ) -> bool;

    fn mouse_up(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        options: &mut Option<Box<dyn EditorOptions>>,
        toolbar: &mut Option<&mut ToolBar>,
    ) -> bool;

    fn mouse_dragged(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        options: &mut Option<Box<dyn EditorOptions>>,
        toolbar: &mut Option<&mut ToolBar>,
    ) -> bool;

    fn mouse_wheel(
        &mut self,
        delta: (isize, isize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        options: &mut Option<Box<dyn EditorOptions>>,
        toolbar: &mut Option<&mut ToolBar>,
    ) -> bool;

    fn mouse_hover(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        options: &mut Option<Box<dyn EditorOptions>>,
        toolbar: &mut Option<&mut ToolBar>,
    ) -> bool {
        false
    }

    fn key_down(
        &mut self,
        char: Option<char>,
        key: Option<WidgetKey>,
        asset: &mut Asset,
        context: &mut ScreenContext,
        options: &mut Option<Box<dyn EditorOptions>>,
        toolbar: &mut Option<&mut ToolBar>,
    ) -> bool {
        false
    }

    /// Content is opening
    fn opening(
        &mut self,
        asset: &mut Asset,
        context: &mut ScreenContext,
        options: &mut Option<Box<dyn EditorOptions>>,
    ) {
    }

    /// Content is closing
    fn closing(
        &mut self,
        asset: &mut Asset,
        context: &mut ScreenContext,
        options: &mut Option<Box<dyn EditorOptions>>,
    ) {
    }

    // For TileMapWidget

    // Returns true if we show an image
    fn is_image(&mut self) -> bool {
        false
    }

    // Set the current tilemap id
    fn set_tilemap_id(&mut self, id: Uuid, asset: &mut Asset) {}

    /// Converts a screen position to a map grid position
    fn screen_to_map(&self, asset: &Asset, screen_pos: (usize, usize)) -> Option<(usize, usize)> {
        None
    }

    // For RegionWidget

    /// Sets a region id
    fn set_region_id(
        &mut self,
        id: Uuid,
        context: &mut ScreenContext,
        options: &mut Option<Box<dyn EditorOptions>>,
    ) {
    }

    /// Get the tile id
    fn get_tile_id(&self, pos: (usize, usize)) -> Option<(isize, isize)> {
        None
    }

    /// Returns the selected tile
    fn get_selected_tile(&self) -> Option<TileData> {
        None
    }

    /// Return the tile selector
    fn get_tile_selector(&mut self) -> Option<&mut TileSelectorWidget> {
        None
    }

    /// Returns the region_id
    fn get_region_id(&self) -> Uuid {
        Uuid::new_v4()
    }

    /// Returns the currently selected tile id in the editor
    fn get_selected_editor_tile(&self) -> Option<(isize, isize)> {
        None
    }

    /// Returns the currently selected character Uuid in the editor
    fn get_selected_editor_character(&self) -> Option<Uuid> {
        None
    }

    /// Returns the currently selected loot Uuid in the editor
    fn get_selected_editor_loot(&self) -> Option<Uuid> {
        None
    }

    /// Return the behavior graph
    fn get_behavior_graph(&mut self) -> Option<&mut NodeGraph> {
        None
    }

    /// Update the area ui
    fn update_area_ui(&mut self, context: &mut ScreenContext) {}

    /// Sets a new name for the current area
    fn set_area_name(&mut self, name: String, context: &mut ScreenContext) {}

    /// Gets the layer mask for the hovered tile (if any)
    fn get_layer_mask(&mut self, context: &mut ScreenContext) -> Option<Vec<Option<TileData>>> {
        None
    }

    // For NodeGraphs

    fn debug_data(&mut self, context: &mut ScreenContext, data: BehaviorDebugData) {}

    fn set_mode(&mut self, mode: GraphMode, context: &ScreenContext) {}
    fn set_mode_and_rect(
        &mut self,
        mode: GraphMode,
        rect: (usize, usize, usize, usize),
        context: &mut ScreenContext,
    ) {
    }
    fn set_mode_and_nodes(
        &mut self,
        mode: GraphMode,
        nodes: Vec<NodeWidget>,
        _context: &ScreenContext,
    ) {
    }

    /// Returns the rectangle for the given node either in relative or absolute coordinates
    fn get_node_rect(&self, node_index: usize, relative: bool) -> (isize, isize, usize, usize) {
        (0, 0, 0, 0)
    }

    /// Updates a node value from the dialog
    fn update_from_dialog(
        &mut self,
        id: (Uuid, Uuid, String),
        value: Value,
        asset: &mut Asset,
        context: &mut ScreenContext,
        options: &mut Option<Box<dyn EditorOptions>>,
    ) {
    }

    /// Marks the two nodes as dirty
    fn changed_selection(&mut self, old_selection: usize, new_selection: usize) {}

    /// Mark all nodes as dirty
    fn mark_all_dirty(&mut self) {}

    /// Set the behavior id, this will take the bevhavior node data and create the node widgets
    fn set_behavior_id(&mut self, id: Uuid, context: &mut ScreenContext) {}

    /// Adds a node of the type identified by its name
    fn add_node_of_name(
        &mut self,
        name: String,
        position: (isize, isize),
        context: &mut ScreenContext,
    ) {
    }

    /// Adds an overview node
    fn add_overview_node(&mut self, node: NodeWidget, context: &mut ScreenContext) {}

    /// Inits the node widget (atom widgets, id)
    fn init_node_widget(&mut self, node_widget: &mut NodeWidget, context: &mut ScreenContext) {}

    /// Sets up the corner node widget
    fn setup_corner_node_widget(
        &mut self,
        behavior_data: &GameBehaviorData,
        node_data: &BehaviorNode,
        node_widget: &mut NodeWidget,
        context: &ScreenContext,
    ) {
    }

    /// Converts the index of a node widget to a node id
    fn widget_index_to_node_id(&self, index: usize) -> Uuid {
        Uuid::new_v4()
    }

    /// Converts the id of a node to a widget index
    fn node_id_to_widget_index(&self, id: Uuid) -> usize {
        0
    }

    /// Returns true if the node connector is a source connector (Right or Bottom)
    fn connector_is_source(&self, connector: BehaviorNodeConnector) -> bool {
        false
    }

    /// Disconnect the node from all connections
    fn disconnect_node(&mut self, id: Uuid, context: &mut ScreenContext) {}

    /// Disconnect the node from all connections
    fn delete_node(&mut self, id: Uuid, context: &mut ScreenContext) {}

    /// Sets the widget and behavior data for the given atom id
    fn set_node_atom_data(
        &mut self,
        node_atom_id: (Uuid, Uuid, String),
        value: Value,
        context: &mut ScreenContext,
    ) {
    }

    /// Checks the visibility of a node
    fn check_node_visibility(&mut self, context: &ScreenContext) {}

    /// Marks all connected nodes as visible
    fn mark_connections_visible(&mut self, id: Uuid, context: &ScreenContext) {}

    /// Checks if the given node id is part of an unconnected branch.
    fn belongs_to_standalone_branch(&mut self, id: Uuid, context: &ScreenContext) -> bool {
        false
    }

    /// Collects the children indices of the given node id so that they can all be dragged at once
    fn collect_drag_children_indices(&mut self, id: Uuid, context: &ScreenContext) {}

    /// Returns the behavior id for the current behavior and graph type
    fn get_curr_behavior_id(&self, context: &ScreenContext) -> Uuid {
        Uuid::new_v4()
    }

    /// Returns the current node id for the given graph type
    fn get_curr_node_id(&self, context: &ScreenContext) -> Option<Uuid> {
        None
    }

    /// Marks the node graph for redraw
    fn set_dirty(&mut self) {}

    /// Gets the node vec
    fn get_nodes(&mut self) -> Option<&mut Vec<NodeWidget>> {
        None
    }

    /// Get the rect
    fn get_rect(&self) -> (usize, usize, usize, usize) {
        (0, 0, 0, 0)
    }

    /// Get the offset
    fn get_offset(&self) -> (isize, isize) {
        (0, 0)
    }

    /// Get the preview widget
    fn get_preview_widget(&mut self) -> Option<&mut NodePreviewWidget> {
        None
    }

    /// A game debug update
    fn debug_update(&mut self, update: GameUpdate, context: &mut ScreenContext) {}

    /// Debugging stopped
    fn debugging_stopped(&mut self) {}

    /// Get the sub node type
    fn get_sub_node_type(&mut self) -> NodeSubType {
        NodeSubType::None
    }

    /// Set the sub type of the node
    fn set_sub_node_type(&mut self, sub_type: NodeSubType, context: &mut ScreenContext) {}

    /// Sort / update the node graph
    fn sort(&mut self, context: &mut ScreenContext) {}

    /// Get the currently active indices in the node graph
    fn get_active_indices(&self) -> Vec<usize> {
        vec![]
    }

    fn set_active_indices(&mut self, indices: Vec<usize>) {}

    // For ScreenEditor

    fn get_hover_rect(&self) -> Option<(usize, usize, usize, usize)> {
        None
    }
    fn set_current_script(&mut self, script: String, context: &mut ScreenContext) {}

    // Undo / Redo

    fn is_undo_available(&self, context: &ScreenContext) -> bool {
        false
    }
    fn is_redo_available(&self, context: &ScreenContext) -> bool {
        false
    }

    fn undo(&mut self, context: &mut ScreenContext) {}
    fn redo(&mut self, context: &mut ScreenContext) {}
}
