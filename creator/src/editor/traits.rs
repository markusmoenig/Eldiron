use crate::editor::node::NodeWidget;
use core_server::gamedata::behavior::*;
use core_shared::asset::{Asset, TileUsage};

use crate::editor::ScreenContext;
use crate::WidgetState;
use crate::editor::{ ToolBar, TileSelectorWidget, NodeGraph };

use crate::editor::regionoptions::RegionEditorMode;

use super::node_preview::NodePreviewWidget;

#[allow(unused)]
pub trait EditorOptions {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self where Self: Sized;

    fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext);

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>);

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) -> bool;

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) -> bool;

    fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) -> bool;

    fn mouse_wheel(&mut self, delta: (isize, isize), asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) -> bool { false }

    fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) -> bool { false }

    // Sets the state of the atom widgets
    fn set_state(&mut self, state: WidgetState) {}

    /// Options are opening
    fn opening(&mut self, asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) { }

    /// Options are closing
    fn closing(&mut self, asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) { }


    // For TilemapOptions

    /// Updates the group widget based on the selected tile
    fn adjust_tile_usage(&mut self, asset: &Asset, context: &ScreenContext) {}

    /// Sets the tile anim for the current tile
    fn set_anim(&mut self, asset: &mut Asset, context: &ScreenContext) {}

    /// Clears the tile anim for the current tile
    fn clear_anim(&mut self, asset: &mut Asset, context: &ScreenContext) {}

    /// Sets the default tile for the current map
    fn set_default_tile(&mut self, asset: &mut Asset, context: &ScreenContext) {}

    /// Set the tile tags
    fn set_tags(&mut self, tags: String, asset: &mut Asset, context: &ScreenContext) {}

    // For RegionOptions

    /// Returns the current region editor mode
    fn get_editor_mode(&self) -> RegionEditorMode { RegionEditorMode::Tiles }

    /// Update the area ui
    fn update_area_ui(&mut self, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) {}

    /// Sets a new name for the current area
    fn set_area_name(&mut self, name: String, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) {}

    /// Get the current tile usage
    fn get_tile_usage(&self) -> TileUsage { TileUsage::Environment }

    /// Get the current tile_id if any
    fn get_tilemap_index(&self) -> Option<usize> { None }

    /// Get the current tags
    fn get_tags(&self) -> Option<String> { None }

    /// Get the current layer
    fn get_layer(&self) -> usize { 0 }

    /// Set the tags
    fn set_region_tags(&mut self, tags: String, asset: &mut Asset, context: &ScreenContext, content: &mut Option<Box<dyn EditorContent>>) {}

    /// Sets the area names
    fn set_area_names(&mut self, names: Vec<String>) {}

    // For ScreenOptions

    /// Update the ui
    fn update_ui(&mut self, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) {}

    /// Set the name of the widget
    fn set_widget_name(&mut self, name: String, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) {}

    /// Returns the current region editor mode
    fn get_screen_editor_mode(&self) -> super::screeneditor_options::ScreenEditorMode { super::screeneditor_options::ScreenEditorMode::Script }

}

#[derive(PartialEq)]
pub enum GraphMode {
    Overview,
    Detail
}

#[allow(unused)]
pub trait EditorContent {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), behavior_type: BehaviorType, asset: &Asset, context: &ScreenContext) -> Self where Self: Sized;

    fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext);

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>);

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>, toolbar: &mut Option<&mut ToolBar>) -> bool;

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>, toolbar: &mut Option<&mut ToolBar>) -> bool;

    fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>, toolbar: &mut Option<&mut ToolBar>) -> bool;

    fn mouse_wheel(&mut self, delta: (isize, isize), asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>, toolbar: &mut Option<&mut ToolBar>) -> bool;

    fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>, toolbar: &mut Option<&mut ToolBar>) -> bool { false }

    /// Content is opening
    fn opening(&mut self, asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>) {}

    /// Content is closing
    fn closing(&mut self, asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>) { }


    // For TileMapWidget

    // Set the current tilemap id
    fn set_tilemap_id(&mut self, id: usize) {}

    /// Converts a screen position to a map grid position
    fn screen_to_map(&self, asset: &Asset, screen_pos: (usize, usize)) -> Option<(usize, usize)> { None }

    // For RegionWidget


    /// Sets a region id
    fn set_region_id(&mut self, id: usize, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>) {}

    /// Get the tile id
    fn get_tile_id(&self, pos: (usize, usize)) -> Option<(isize, isize)> { None }

    /// Returns the selected tile
    fn get_selected_tile(&self) -> Option<(usize, usize, usize, TileUsage)> { None }

    /// Return the tile selector
    fn get_tile_selector(&mut self) -> Option<&mut TileSelectorWidget> { None }

    /// Returns the region_id
    fn get_region_id(&self) -> usize { 0 }

    /// Return the behavior graph
    fn get_behavior_graph(&mut self) -> Option<&mut NodeGraph> { None }


    // For NodeGraphs

    fn update(&mut self, context: &mut ScreenContext) {}

    fn set_mode(&mut self, mode: GraphMode, context: &ScreenContext) {}
    fn set_mode_and_rect(&mut self, mode: GraphMode, rect: (usize, usize, usize, usize), context: &ScreenContext) {}
    fn set_mode_and_nodes(&mut self, mode: GraphMode, nodes: Vec<NodeWidget>, _context: &ScreenContext) {}

    /// Returns the rectangle for the given node either in relative or absolute coordinates
    fn get_node_rect(&self, node_index: usize, relative: bool) -> (isize, isize, usize, usize) { (0,0,0,0) }

    /// Updates a node value from the dialog
    fn update_from_dialog(&mut self, context: &mut ScreenContext) {}

    /// Marks the two nodes as dirty
    fn changed_selection(&mut self, old_selection: usize, new_selection: usize) {}

    /// Mark all nodes as dirty
    fn mark_all_dirty(&mut self) {}

    /// Set the behavior id, this will take the bevhavior node data and create the node widgets
    fn set_behavior_id(&mut self, id: usize, context: &mut ScreenContext) {}

    /// Adds a node of the type identified by its name
    fn add_node_of_name(&mut self, name: String, position: (isize, isize), context: &mut ScreenContext) {}

    /// Inits the node widget (atom widgets, id)
    fn init_node_widget(&mut self, behavior_data: &GameBehaviorData, node_data: &BehaviorNode, node_widget: &mut NodeWidget, context: &ScreenContext) {}

    /// Sets up the corner node widget
    fn setup_corner_node_widget(&mut self, behavior_data: &GameBehaviorData, node_data: &BehaviorNode, node_widget: &mut NodeWidget, context: &ScreenContext) {}

    /// Converts the index of a node widget to a node id
    fn widget_index_to_node_id(&self, index: usize) -> usize { 0 }

    /// Converts the id of a node to a widget index
    fn node_id_to_widget_index(&self, id: usize) -> usize { 0 }

    /// Returns true if the node connector is a source connector (Right or Bottom)
    fn connector_is_source(&self, connector: BehaviorNodeConnector) -> bool { false }

    /// Disconnect the node from all connections
    fn disconnect_node(&mut self, id: usize, context: &mut ScreenContext) {}

    /// Disconnect the node from all connections
    fn delete_node(&mut self, id: usize, context: &mut ScreenContext) {}

    /// Sets the widget and behavior data for the given atom id
    fn set_node_atom_data(&mut self, node_atom_id: (usize, usize, String), data: (f64, f64, f64, f64, String), context: &mut ScreenContext) {}

    /// Checks the visibility of a node
    fn check_node_visibility(&mut self, context: &ScreenContext) {}

    /// Marks all connected nodes as visible
    fn mark_connections_visible(&mut self, id: usize, context: &ScreenContext) {}

    /// Checks if the given node id is part of an unconnected branch.
    fn belongs_to_standalone_branch(&mut self, id: usize, context: &ScreenContext) -> bool { false }

    /// Collects the children indices of the given node id so that they can all be dragged at once
    fn collect_drag_children_indices(&mut self, id: usize, context: &ScreenContext) {}

    /// Returns the behavior id for the current behavior and graph type
    fn get_curr_behavior_id(&self, context: &ScreenContext) -> usize { 0 }

    /// Returns the current node id for the given graph type
    fn get_curr_node_id(&self, context: &ScreenContext) -> Option<usize> { None }

    /// Marks the node graph for redraw
    fn set_dirty(&mut self) {}

    /// Gets the node vec
    fn get_nodes(&mut self) -> Option<&mut Vec<NodeWidget>> { None }

    /// Get the rect
    fn get_rect(&self) -> (usize, usize, usize, usize) { (0,0,0,0) }

    /// Get the offset
    fn get_offset(&self) -> (isize, isize) { (0,0) }

    /// Get the preview widget
    fn get_preview_widget(&mut self) -> Option<&mut NodePreviewWidget> { None }


    // For ScreenEditor

    fn get_hover_rect(&self) -> Option<(usize, usize, usize, usize)> { None }

}