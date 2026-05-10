# Menu
## Menu File
menu_file = File
menu_new = New
menu_close = Close
menu_open = Open...
menu_save = Save...
menu_save_as = Save As...
new_project = New Project
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
status_create_cutout_failed = Create Cutout needs at least three selected surface-line points on one 3D face.
status_create_cutout_open_loop = Create Cutout needs closed surface-line loops. Finish or close the selected guide first.
status_create_cutout_multiple_faces = Create Cutout currently needs all selected guide loops on one host surface.
## Sidebar
status_project_add_button = Add to the project.
status_project_remove_button = Remove an item from the project.
status_project_duplicate_button = Duplicate the current project item.
status_project_import_button = Import to the project.
status_project_export_button = Export from the project.
## Dock
status_dock_action_apply = Apply the current action.
status_dock_action_auto = Auto apply actions.
## Effect Picker
status_effect_picker_filter_edit = Show tiles containing the given text.
## Map Editor
status_map_editor_grid_sub_div = The grid subdivision / snap step.
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
status_tiles_filter_edit = Show tiles containing the given alias.
status_tiles_apply_tile = Apply the selected tile to the selected icon slot.
status_tiles_clear_tile = Clear the selected icon slot.
## World Editor
status_world_editor_brush_radius = Controls the size of the brush in world units.
status_world_editor_brush_falloff = Controls how quickly the brush strength fades from the center.
status_world_editor_brush_strength = Maximum intensity of the brush effect at the center.
status_world_editor_brush_fixed = Fixed terrain height used by the 'Fixed' brush.

# Actions
action_apply_tile = Apply Tile
action_apply_tile_desc = Applies the current tile source to the selected sectors or 3D faces.
action_clear_tile = Clear Tile
action_clear_tile_desc = Clears the tiles from the selected sectors or 3D faces.
action_copy_tile_id = Copy Tile ID
action_copy_tile_id_desc = Copies the ID of the tile to the clipboard for later use in the code editor.
action_copy_vcode = Copy Visual Code
action_copy_vcode_desc = Copies the current visual code module to the clipboard.
action_create_center_vertex = Create Center Vertex
action_create_center_vertex_desc = Creates a new vertex in the center of the selected sectors.
action_create_linedef = Create Linedef
action_create_linedef_desc = Creates a new linedef between two vertices.
action_create_cutout = Create Cutout
action_create_cutout_desc = Cuts an opening from the selected closed 3D surface-line loop through the host object.
action_create_groove = Create Groove
action_create_groove_desc = Converts selected 3D surface lines into persistent recessed groove geometry.
action_create_ridge = Create Ridge
action_create_ridge_desc = Converts selected 3D surface lines into persistent raised ridge geometry.
action_create_sector = Create Sector
action_create_sector_desc = Creates a new sector / surface from the selected vertices. The vertices must form a closed loop (we auto-order them).
action_create_geometry_box = Create Box
action_create_geometry_box_desc = Create a directly editable 3D box object.
action_duplicate_tile = Duplicate Tile
action_duplicate_tile_desc = Duplicates the currently selected tile.
action_duplicate_surface_detail = Duplicate Surface Detail
action_duplicate_surface_detail_desc = Duplicates selected 3D surface-line guide geometry on the host face.
action_toggle_surface_curve = Surface Curve
action_toggle_surface_curve_desc = Sets selected 3D surface-line segments, or segments between selected points, to straight lines or configurable arcs.
action_edit_face_texture = Edit Face Texture
action_edit_face_texture_desc = Edit per-face 3D texture offset, scale, and rotation for selected faces or whole selected Geometry Objects.
action_edit_geometry = Edit Geometry
action_edit_geometry_desc = Edit selected 3D geometry position, size, visibility, solidity, and group.
action_face_extrude = Face Extrude
action_face_extrude_desc = Extrude selected 3D faces by the given amount.
action_face_cut_opening = Face Cut Opening
action_face_cut_opening_desc = Cut a rectangular opening through the selected 3D face and its opposite face.
action_face_inset = Face Inset
action_face_inset_desc = Inset selected 3D faces by the given amount.
action_face_delete = Face Delete
action_face_delete_desc = Delete selected 3D faces and select their boundary vertices.
action_face_merge = Face Merge
action_face_merge_desc = Merge selected connected 3D faces into one editable face.
action_face_subdivide = Face Subdivide
action_face_subdivide_desc = Subdivide selected quad faces into smaller editable faces.
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
action_filter_edit_geo = Filter Geometry
action_filter_edit_geo_desc = Filters editor rendering so you can isolate generated dungeon geometry while editing.
action_build_procedural = Build Procedural
action_build_procedural_desc = Builds procedural map geometry from the current region settings.
action_build_procedural_help = Builds the current region's [procedural] configuration into editable map geometry.
action_first_p_camera = 3D First Person Camera
action_first_p_camera_desc = Render the scene using a 3D first person camera.
status_firstp_fly_nav_on = FirstP fly navigation on. Pointer from center turns/looks, WASD moves, Space exits.
status_firstp_fly_nav_rmb_on = FirstP fly navigation on. Hold right mouse to look, WASD moves, release right mouse or press Escape to exit.
status_firstp_fly_nav_off = FirstP fly navigation off.
status_camera_2d = Edit the map in 2D.
status_camera_orbit_macos = Edit the map with a 3D orbit camera. Wheel zooms. Right-drag or Alt-drag orbits. Cmd-drag or Shift-wheel pans. Arrow keys move the target.
status_camera_orbit_other = Edit the map with a 3D orbit camera. Wheel zooms. Right-drag or Alt-drag orbits. Ctrl-drag or Shift-wheel pans. Arrow keys move the target.
status_camera_iso_macos = Edit the map in 3D isometric view. Wheel zooms. Right-drag, Alt-drag, Cmd-drag, or Shift-wheel pans. Arrow keys move the target.
status_camera_iso_other = Edit the map in 3D isometric view. Wheel zooms. Right-drag, Alt-drag, Ctrl-drag, or Shift-wheel pans. Arrow keys move the target.
status_camera_firstp = Edit the map in 3D first person view. Hold right mouse and use WASD to fly. Space toggles fly mode for touchpads.
action_tile_procedural_style = Style
action_tile_procedural_kind = Kind
action_tile_procedural_weight = Weight
action_import_vcode = Import Visual Code ...
action_import_vcode_desc = Import a visual code module.
action_paste_vcode = Paste Visual Code
action_paste_vcode_desc = Imports a visual code module from the clipboard.
tool_authoring = Authoring
status_tool_authoring = Authoring mode. Enter meta-data for sectors, linedefs, entities, and items.
tool_text_play = Text Play
status_tool_text_play = Text gameplay mode for the Game Tool. Replaces the normal game view with text output and command input so you can play the game through text.
authoring_select_prompt = Authoring mode. Select a sector, linedef, entity or item.
authoring_title_prefix = Authoring mode. Enter meta-data for
authoring_title = Authoring mode. Enter meta-data for {$target}.
authoring_target_sector = Sector
authoring_target_linedef = Linedef
authoring_target_character = Character
authoring_target_item = Item
action_iso_camera = 3D Iso Camera
action_iso_camera_desc = Render the scene using a 3D Iso camera.
action_minimize = Minimize
action_minimize_desc = Minimizes the editor / dock.
action_new_tile = New Tile
action_new_tile_desc = Creates a new tile with frames of the given size.
action_orbit_camera = 3D Orbit Camera
action_orbit_camera_desc = Render the scene using a 3D orbit camera.
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
tool_game = Game Tool (K). Play the game!
tool_builder = Builder Tool (B). Select reusable prop and assembly assets from the builder picker.
tool_palette = Palette Tool (P). Edit palette entries and apply palette colors.
tool_dungeon = Dungeon Tool (U). Paint conceptual dungeon structures.
tool_linedef = Linedef / Edge Tool (L). Create 2D line definitions and edit 3D geometry edges.
tool_object = Object Tool (G). Select and move directly editable 3D objects.
tool_rect = Rect Tool (R). Click to draw the current tile. Shift-click to delete. Alt/Opt-click to pick from the map.
tool_sector = Sector / Face Tool (E). Select sectors in 2D or faces in 3D.
tool_vertex = Vertex Tool (V). 'Shift' + Click to create a new vertex.
tool_entity = Entity Tool (Y). Place, move, select, and delete game entities.
tool_organic = Organic Paint Tool (O). Paint volumetric organic detail using the active brush graph.
hud_geometry_op_move = MOVE
hud_geometry_op_size = SIZE
status_hud_geometry_op_move = Object gizmo operation: move (M).
status_hud_geometry_op_size = Object gizmo operation: resize (S).
status_geometry_empty_selection = 3D selection: G = Object, E = Face, V = Vertex, L = Edge.
status_geometry_object_selection = Object selected: M = Move, S = Size.
status_geometry_face_selection = Face selected: +/- = Push/Pull, [] = Move Up/Down, Delete = Delete.
status_geometry_vertex_selection = Vertex selected: F = Fill, X = Split Edge, M = Merge, L = Edge Loop, [] = Move Up/Down, Delete = Delete.
status_geometry_edge_selection = Edge selected: F = Fill, X = Split Edge, M = Merge, L = Edge Loop, [] = Move Up/Down, Delete = Delete.
status_geometry_surface_selection = Surface detail selected: Shift = add, Alt = remove, L = connected guide.
status_geometry_surface_loop_selection = Closed surface detail selected: Shift = add, Alt = remove, L = connected guide.
organic_dock_title = Organic Brushes
organic_toggle_active = Active
organic_toggle_deactive = Deactive
organic_mode_free = Free
organic_mode_locked = Locked
status_organic_toggle_visibility = Toggle organic paint rendering on or off.
status_organic_lock_mode = Free paints every surface. Locked paints only the selected sector or active surface.
status_organic_clear = Clear painted organic detail. In locked mode this clears only the selected sector or active surface.

builder_picker_title = Builder Picker
builder_apply_build = Apply Build
palette_apply_color = Apply Color
status_palette_apply_color = Apply the current palette entry to the selected target.
status_builder_new = Create a new builder graph asset.
status_builder_collections = Builder collections will be added here later.
status_builder_apply_build = Apply the selected builder graph to the selected hosts.
status_builder_clear_build = Clear the builder graph from the selected hosts.
status_builder_select_asset = Select builder asset '{$asset_name}'. Double-click or press Return to open.

# Common
all = All
apply = Apply
attributes = Attributes
preview_rigging = Preview Rigging
clear = Clear
collections = Collections
filter = Filter
frames = Frames
grid_size = Grid Size
name = Name
new = New
opacity = Opacity
roughness = Roughness
metallic = Metallic
emissive = Emissive
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
arms = Arms
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
action_duplicate = Duplicate
action_duplicate_desc = Duplicate selected geometry with an XYZ offset.
