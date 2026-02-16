# Menu
## Menu File
menu_file = File
menu_new = New
menu_open = Open...
menu_save = Save...
menu_save_as = Save As...
## Menu Edit
menu_edit = Edit
menu_undo = Undo
menu_redo = Redo
menu_cut = Cut
menu_copy = Copy
menu_paste = Paste
menu_apply_action = Apply Action
# Menu Game
menu_play = Start
menu_pause = Pause
menu_stop = Stop

# Widgets
## Dock
dock_auto = Automatic
## Node Editor
node_editor_create_button = Create Graph
## Render Editor
render_editor_trace_button = Start Trace
## Tilemap
tilemap_add_button = Add Tile(s)

# Status
## Actions
status_logo_button = Open the Eldiron Website ...
status_open_button = Open an existing Eldiron project...
status_save_button = Save the current project.
status_save_as_button = Save the current project to a new file.
status_undo_button = Undo the last action.
status_redo_button = Redo the last action.
status_play_button = Start the game server for live editing and debugging.
status_pause_button = Pause. Click for single stepping the game server.
status_stop_button = Stop the game server.
status_game_input_button = Routes input to the game instead of the editor when the game is running.
status_time_slider = Adjust the server time.
status_update_button = Update application.
status_patreon_button = Visit the Eldiron Patreon page. Thanks for your support.
status_help_button = Click on any UI element to visit the Eldiron Online Documentation.
## Sidebar
status_project_add_button = Add to the project.
status_project_remove_button = Remove an item from the project.
status_project_import_button = Import to the project.
status_project_export_button = Export from the project.
## Dock
status_dock_action_apply = Apply the current action.
status_dock_action_auto = Auto apply actions.
## Effect Picker
status_effect_picker_filter_edit = Show tiles containing the given text.
## Map Editor
status_map_editor_grid_sub_div = The subdivision level of the grid.
## Node Editor
status_node_editor_graph_id = The Id of the graph inside the map.
status_node_editor_create_button = Apply the source to the selected geometry.
status_node_editor_fx_node_button = Nodes which create a special effect like lights or particles.
status_node_editor_render_nodes_button = Nodes for the global and local render graphs.
status_node_editor_mesh_nodes_button = Nodes which control and modify terrain and mesh creation.
status_node_editor_shapefx_nodes_button = Nodes which attach to geometry and shapes and create colors and patterns.
## Shape Picker
status_shape_picker_filter_edit = Show tiles containing the given text.
## Tilemap Editor
status_tilemap_editor_clear_button = Clear the current selection.
status_tilemap_editor_add_button = Adds the current tile selection.
## Tile Picker
status_tile_picker_filter_edit = Show tiles containing the given text.
## Tilemap
status_tilemap_clear_button = Clear the current selection.
status_tilemap_add_button = Adds the current tile selection.
## Tiles
status_tiles_filter_edit = Show tiles containing the given tags.
## World Editor
status_world_editor_brush_radius = Controls the size of the brush in world units.
status_world_editor_brush_falloff = Controls how quickly the brush strength fades from the center.
status_world_editor_brush_strength = Maximum intensity of the brush effect at the center.
status_world_editor_brush_fixed = Fixed terrain height used by the 'Fixed' brush.

# Actions
action_add_arch = Add Arch
action_add_arch_desc = Add an arch (curved polyline) replacing the selected linedef(s).
action_apply_tile = Apply Tile
action_apply_tile_desc = Applies the current tile to the selected sectors.
action_clear_profile = Clear Profile
action_clear_profile_desc = Clears a potential profile feature (Recess, Relief, Gate/Door) from the sector.
action_clear_tile = Clear Tile
action_clear_tile_desc = Clears the tiles from the selected sectors.
action_copy_tile_id = Copy Tile ID
action_copy_tile_id_desc = Copies the ID of the tile to the clipboard for later use in the code editor.
action_create_center_vertex = Create Center Vertex
action_create_center_vertex_desc = Creates a new vertex in the center of the selected sectors.
action_create_linedef = Create Linedef
action_create_linedef_desc = Creates a new linedef between two vertices.
action_create_sector = Create Sector
action_create_sector_desc = Creates a new sector / surface from the selected vertices. The vertices must form a closed loop (we auto-order them).
action_duplicate_tile = Duplicate Tile
action_duplicate_tile_desc = Duplicates the currently selected tile.
action_edit_linedef = Edit Linedef
action_edit_linedef_desc = Edit the attributes of the selected linedef.
action_editing_slice = Editing Slice
action_editing_slice_desc = Sets the position of the vertical editing slice in the 2D editing view.
action_edit_maximize = Edit / Maximize
action_edit_maximize_desc = Open the editor for the current dock or maximizes it.
action_edit_sector = Edit Sector
action_edit_sector_desc = Edit the attributes of the selected sector.
action_edit_tile = Edit Tile Meta Data
action_edit_tile_desc = Edit the meta data of the currently selected tile.
action_edit_vertex = Edit Vertex
action_edit_vertex_desc = Edit the attributes of the selected vertex. The XZ positions are the ground / 2D plane positions. Enable the vertex as a terrain control point or give the vertex a billboard tile.
action_editing_camera = 2D Camera
action_editing_camera_desc = Render the scene using the 2D editing camera.
action_export_vcode = Export Visual Code ...
action_export_vcode_desc = Export the current visual code module.
action_extrude_linedef = Extrude Linedef
action_extrude_linedef_desc = Extrudes the linedef by the given distance and creates a new sector. The angle applies an optional rotation around the linedef axis.
action_extrude_sector = Extrude Sector
action_extrude_sector_desc = Sets surface extrusion on selected sectors, optionally with an open back.
action_first_p_camera = 3D First Person Camera
action_first_p_camera_desc = Render the scene using a 3D first person camera.
action_gate_door = Gate / Door
action_gate_door_desc = Creates a hole with a gate / door in the selected profile sector.
action_import_vcode = Import Visual Code ...
action_import_vcode_desc = Import a visual code module.
action_iso_camera = 3D Iso Camera
action_iso_camera_desc = Render the scene using a 3D Iso camera.
action_minimize = Minimize
action_minimize_desc = Minimizes the editor / dock.
action_new_tile = New Tile
action_new_tile_desc = Creates a new tile with frames of the given size.
action_orbit_camera = 3D Orbit Camera
action_orbit_camera_desc = Render the scene using a 3D orbit camera.
action_recess = Recess
action_recess_desc = Creates a recess in the selected profile sector.
action_relief = Relief
action_relief_desc = Creates a relief (emboss) on the selected profile sector.
action_set_edit_surface = Set Editing Surface
action_set_edit_surface_desc = Make the selected surface the active 2D profile for editing.
action_set_tile_material = Set Tile Material
action_set_tile_material_desc = Set the material attributes to all pixels of the tile.
action_split = Split
action_split_desc = Split the selected linedef(s) by adding a middle point. Thew new point gets added to all sectors the linedef is part of.
action_toggle_edit_geo = Toggle Editing Geometry
action_toggle_edit_geo_desc = Toggles visibility of the editing geometry overlay.
action_toggle_rect_geo = Toggle Rect Geometry
action_toggle_rect_geo_desc = Geometry created by the Rect tool is by default not shown in the 2D editor. This action toggles visibilty.
action_import_palette = Import Palette ...
action_import_palette_desc = Import a Paint.net palette
action_clear_palette = Clear Palette
action_clear_palette_desc = Clears the palette
action_remap_tile = Remap Tile
action_remap_tile_desc = Remaps the colors of the tile to the palette.

# Tools
tool_game = Game Tool (G). If the server is running input events are send to the game.
tool_linedef = Linedef Tool (L). Create line definitions and sectors.
tool_rect = Rect Tool (R). Click to draw the current tile. Shift-click to delete.
tool_sector = Sector Tool (E).
tool_selection = Selection Tool (S). Hold 'Shift' to add. 'Alt' to subtract. Click and drag for multi-selection. 3D: Select editing plane.
tool_selection_mac = Selection Tool (S). Hold 'Shift' to add. 'Option' to subtract. Click and drag for multi-selection. 3D: Select editing plane.
tool_vertex = Vertex Tool (V). 'Shift' + Click to create a new vertex.
tool_entity = Entity Tool (Y). Place, move, select, and delete game entities.

# Common
all = All
apply = Apply
attributes = Attributes
preview_rigging = Preview Rigging
clear = Clear
filter = Filter
frames = Frames
grid_size = Grid Size
name = Name
opacity = Opacity
eldrin_scripting = Eldrin Scripting
settings = Settings
size = Size
visual_script = Visual Scripting
region = Region
regions = Regions
characters = Characters
items = Items
tilesets = Tilesets
screens = Screens
assets = Assets
fonts = Fonts
game = Game
character_instance = Character Instance
item_instance = Item Instance
opacity = Opacity
palette = Palette
debug_log = Debug Log
avatars = Avatars
body_markers = Body Markers
anchors = Anchors
skin_light = Light Skin
skin_dark = Dark Skin
torso = Torso
legs = Legs
hair = Hair
eyes = Eyes
hands = Hands
feet = Feet
enabled = Enabled

# Info
info_server_started = Server has been started
info_update_check = Checking updates...
info_welcome = Welcome to Eldiron! Visit Eldiron.com for information and example projects.

status_tile_editor_copy_texture = Copied texture to clipboard.
status_tile_editor_copy_selection = Copied selection to clipboard.
status_tile_editor_cut_selection = Cut selection to clipboard.
status_tile_editor_paste_preview_active = Paste preview active. Move mouse, Enter to apply, click or Escape to cancel.
status_tile_editor_paste_preview_canceled = Paste preview canceled.
status_tile_editor_paste_applied = Paste applied.
status_tile_editor_paste_no_valid_target = Paste preview: no valid target pixels at this position.
avatar_anchor_main = Anchor: Main Hand
avatar_anchor_off = Anchor: Off Hand
status_avatar_anchor_set_main = Set main-hand anchor.
status_avatar_anchor_set_off = Set off-hand anchor.
status_avatar_anchor_clear_main = Cleared main-hand anchor.
status_avatar_anchor_clear_off = Cleared off-hand anchor.
