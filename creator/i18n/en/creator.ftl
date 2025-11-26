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

# Widgets
## Dock
dock_action = Action List
## Node Editor
node_editor_create_button = Create Graph
## Render Editor
render_editor_trace_button = Start Trace
## Tilemap
tilemap_add_button = Add Tile(s)

# Status
## Actions
status_action_add_arch_height = Arch bulge height in XY.
status_action_add_arch_segment = Number of segments for the arch polyline.
status_action_edit_linedef_name = Set the name of the linedef.
status_action_edit_sector_name = Set the name of the sector.
status_action_edit_tile_role = Edit the role of the tile.
status_action_edit_tile_blocking = Edit if the tile is blocking (for 2D games only).
status_action_edit_tile_tags = Edit the tags of the tile.
status_action_edit_vertex_name = Set the name of the vertex.
status_action_edit_vertex_x = The x position of the vertex.
status_action_edit_vertex_y = The y position of the vertex.
status_action_edit_vertex_z = The z position of the vertex.
status_action_extrude_linedef_distance = The extrusion distance (sign sets direction).
status_action_extrude_linedef_angle = The angle of rotation around the axis / normal of the geometry.
status_action_extrude_sector_surface_extrusion = When a sector (surface) is selected: turn on/off extrusion for that surface.
status_action_extrude_sector_depth = The extrusion depth.
status_action_extrude_sector_open_back = Leave the back side uncapped; useful for facades/interiors.
status_action_new_tile_size = Size of the new tile.
status_action_new_tile_frames = Number of frames for the new tile
status_action_recess_depth = The depth of the recess.
status_action_recess_target = The recess can be attached to the front or back face.
status_action_recess_tiles = The cap and side (jamb) tiles for the recess.
status_action_relief_height = The height of the relief (emboss).
status_action_relief_target = The relief can be attached to the front or back face.
status_action_relief_tiles = The cap and side (jamb) tiles for the relief.
## Menubar
status_logo_button = Open the Eldiron Website ...
status_open_button = Open an existing Eldiron project...
status_save_button = Save the current project.
status_save_as_button = Save the current project to a new file.
status_undo_button = Undo the last action.
status_redo_button = Redo the last action.
status_play_button = Start the server for live editing and debugging.
status_pause_button = Pause. Click for single stepping the server.
status_stop_button = Stop the server.
status_time_slider = Adjust the server time.
status_update_button = Update application.
status_patreon_button = Visit my Patreon page.
## Sidebar
status_project_add_button = Add to the project.
status_project_remove_button = Remove an item from the project.
status_project_import_button = Import to the project.
status_project_export_button = Export from the project.
## Dock
status_dock_action_apply = Apply the current action.
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
action_add_arch_desc = Add an arch (curved polyline) replacing the selected linedef(s).
action_apply_tile = Applies the current tile to the selected sectors.
action_clear_tile = Clears the tiles from the selected sectors.
action_copy_tile_id = Copies the ID of the tile to the clipboard for later use in the code editor.
action_create_center_vertex = Creates a new vertex in the center of the selected sectors.
action_create_linedef = Creates a new linedef between two vertices
action_create_sector = Creates a new sector / surface from the selected vertices. The vertices must form a closed loop (we auto-order them).
action_duplicate_tile = Duplicates the currently selected tile.
action_edit_linedef_name = Linedef Name
action_edit_linedef_desc = Edit the attributes of the selected linedef.
action_edit_maximize = Open the editor for the current dock or maximizes it.
action_edit_sector_name = Sector Name
action_edit_sector_desc = Edit the attributes of the selected sector.
action_edit_tile_desc = Edit the meta data of the currently selected tile.
action_edit_vertex_name = Vertex Name
action_edit_vertex_x = X-Position
action_edit_vertex_y = Y-Position
action_edit_vertex_z = Z-Position
action_edit_vertex_desc = Edit the attributes of the selected vertex. The XZ positions are the ground / 2D plane positions. The Y-position is up.
action_editing_camera = Render the scene using the 2D editing camera.
action_extrude_linedef_desc = Extrudes the linedef by the given distance and creates a new sector. The angle applies an optional rotation around the linedef axis.
action_extrude_sector_surface_extrusion = Surface Extrusion
action_extrude_sector_open_back = Open Back
action_extrude_sector_desc = Sets **surface extrusion** on selected sectors, optionally with an open back.
action_first_p_camera = Render the scene using a 3D first person camera.
action_iso_camera = Render the scene using a 3D Iso camera.
action_minimize = Minimizes the editor / dock.
action_new_tile_desc = Creates a new tile with frames of the given size.
action_orbit_camera = Render the scene using a 3D orbit camera.
action_recess_desc = Creates a recess in the selected profile sector.
action_relief_desc = Creates a relief (emboss) on the selected profile sector.
action_set_edit_surface = Make the selected surface the active 2D Profil for editing. Eldiron will switch to the 2D view and, if no profile exists yet, create one for this surface. To return to the Region map, click **Region** in the toolbar.
action_split = Split the selected linedef(s) by adding a middle point. Thew new point gets added to all sectors the linedef is part of.
action_toggle_edit_geo = Toggles visibility of the editing geometry overlay.
action_toggle_rect_geo = Geometry created by the Rect tool is by default not shown in the 2D editor. This action toggles visibilty.

# Tools
tool_game = Game Tool (G). If the server is running input events are send to the game.
tool_linedef = Linedef Tool (L). Create line definitions and sectors.
tool_rect = Rect Tool (R). Click to draw the current tile. Shift-click to delete.
tool_sector = Sector Tool (E).
tool_selection = Selection Tool (S). Hold 'Shift' to add. 'Alt' to subtract. Click and drag for multi-selection. 3D: Select editing plane.
tool_selection_mac = Selection Tool (S). Hold 'Shift' to add. 'Option' to subtract. Click and drag for multi-selection. 3D: Select editing plane.
tool_vertex = Vertex Tool (V).

# Common
all = All
angle = Angle
apply = Apply
attributes = Attributes
blocking = Blocking
character = Character
clear = Clear
depth = Depth
distance = Distance
dungeon = Dungeon
effect = Effect
filter = Filter
frames = Frames
grid_size = Grid Size
height = Height
icon = Icon
icons = Icons
manmade = Man Made
mountain = Mountain
name = Names
nature = Nature
opacity = Opacity
python_code = Python Code
role = Role
segments = Segments
settings = Settings
size = Size
tags = Tags
target = Target
ui = UI
visual_script = Visual Scripting
water = Water

# Info
info_server_started = Server has been started
info_update_check = Checking updates...
info_welcome = Welcome to Eldiron! Visit Eldiron.com for information and example projects.
