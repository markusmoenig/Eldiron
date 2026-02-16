# Menu
## Menu File
menu_file = Archivo
menu_new = Nuevo
menu_open = Abrir...
menu_save = Guardar...
menu_save_as = Guardar como...
## Menu Edit
menu_edit = Editar
menu_undo = Deshacer
menu_redo = Rehacer
menu_cut = Cortar
menu_copy = Copiar
menu_paste = Pegar
menu_apply_action = Aplicar acción
# Menu Game
menu_play = Iniciar
menu_pause = Pausar
menu_stop = Detener

# Widgets
## Dock
dock_auto = Automático
## Node Editor
node_editor_create_button = Crear grafo
## Render Editor
render_editor_trace_button = Iniciar traza
## Tilemap
tilemap_add_button = Añadir baldosa(s)

# Status
## Actions
status_logo_button = Abrir el sitio web de Eldiron ...
status_open_button = Abrir un proyecto existente de Eldiron...
status_save_button = Guardar el proyecto actual.
status_save_as_button = Guardar el proyecto actual en un archivo nuevo.
status_undo_button = Deshacer la última acción.
status_redo_button = Rehacer la última acción.
status_play_button = Iniciar el servidor del juego para edición y depuración en vivo.
status_pause_button = Pausar. Clic para ejecutar el servidor del juego paso a paso.
status_stop_button = Detener el servidor del juego.
status_game_input_button = Envía la entrada al juego en lugar del editor cuando el juego está en ejecución.
status_time_slider = Ajustar el tiempo del servidor.
status_update_button = Actualizar aplicación.
status_patreon_button = Visita la página de Patreon de Eldiron. Gracias por tu apoyo.
status_help_button = Haz clic en cualquier elemento de la UI para visitar la Documentación en línea de Eldiron.
## Sidebar
status_project_add_button = Añadir al proyecto.
status_project_remove_button = Eliminar un elemento del proyecto.
status_project_import_button = Importar al proyecto.
status_project_export_button = Exportar del proyecto.
## Dock
status_dock_action_apply = Aplicar la acción actual.
status_dock_action_auto = Aplicar acciones automáticamente.
## Effect Picker
status_effect_picker_filter_edit = Mostrar baldosas que contengan el texto dado.
## Map Editor
status_map_editor_grid_sub_div = Nivel de subdivisión de la cuadrícula.
## Node Editor
status_node_editor_graph_id = ID del grafo dentro del mapa.
status_node_editor_create_button = Aplicar la fuente a la geometría seleccionada.
status_node_editor_fx_node_button = Nodos que crean efectos especiales como luces o partículas.
status_node_editor_render_nodes_button = Nodos para los grafos de renderizado global y local.
status_node_editor_mesh_nodes_button = Nodos que controlan y modifican la creación de terreno y mallas.
status_node_editor_shapefx_nodes_button = Nodos que se unen a geometría y formas y crean colores y patrones.
## Shape Picker
status_shape_picker_filter_edit = Mostrar baldosas que contengan el texto dado.
## Tilemap Editor
status_tilemap_editor_clear_button = Limpiar la selección actual.
status_tilemap_editor_add_button = Añadir la selección de baldosas actual.
## Tile Picker
status_tile_picker_filter_edit = Mostrar baldosas que contengan el texto dado.
## Tilemap
status_tilemap_clear_button = Limpiar la selección actual.
status_tilemap_add_button = Añadir la selección de baldosas actual.
## Tiles
status_tiles_filter_edit = Mostrar baldosas que contengan las etiquetas dadas.
## World Editor
status_world_editor_brush_radius = Controla el tamaño del pincel en unidades de mundo.
status_world_editor_brush_falloff = Controla qué tan rápido decae la intensidad del pincel desde el centro.
status_world_editor_brush_strength = Intensidad máxima del pincel en el centro.
status_world_editor_brush_fixed = Altura de terreno fija usada por el pincel 'Fixed'.

# Actions
action_add_arch = Añadir arco
action_add_arch_desc = Añade un arco (polilínea curvada) sustituyendo la(s) linedef(s) seleccionada(s).
action_apply_tile = Aplicar baldosa
action_apply_tile_desc = Aplica la baldosa actual a los sectores seleccionados.
action_clear_profile = Limpiar perfil
action_clear_profile_desc = Elimina un posible perfil (Hueco, Relieve, Puerta/Portón) del sector.
action_clear_tile = Limpiar baldosa
action_clear_tile_desc = Elimina las baldosas de los sectores seleccionados.
action_copy_tile_id = Copiar ID de baldosa
action_copy_tile_id_desc = Copia el ID de la baldosa al portapapeles para usarlo después en el editor de código.
action_create_center_vertex = Crear vértice central
action_create_center_vertex_desc = Crea un nuevo vértice en el centro de los sectores seleccionados.
action_create_linedef = Crear linedef
action_create_linedef_desc = Crea una nueva linedef entre dos vértices.
action_create_sector = Crear sector
action_create_sector_desc = Crea un nuevo sector/superficie a partir de los vértices seleccionados. Deben formar un bucle cerrado (lo ordenamos automáticamente).
action_duplicate_tile = Duplicar baldosa
action_duplicate_tile_desc = Duplica la baldosa actualmente seleccionada.
action_edit_linedef = Editar linedef
action_edit_linedef_desc = Edita los atributos de la linedef seleccionada.
action_edit_maximize = Editar / Maximizar
action_edit_maximize_desc = Abre el editor del dock actual o lo maximiza.
action_edit_sector = Editar sector
action_edit_sector_desc = Edita los atributos del sector seleccionado.
action_edit_tile = Editar metadatos de baldosa
action_edit_tile_desc = Edita los metadatos de la baldosa seleccionada.
action_edit_vertex = Editar vértice
action_edit_vertex_desc = Edita los atributos del vértice seleccionado. Las posiciones XZ son el plano del suelo/2D. La posición Y es la altura. Opcionalmente activa el vértice como punto de control de terreno o asigna una baldosa billboard al vértice.
action_editing_camera = Cámara 2D
action_editing_camera_desc = Renderiza la escena con la cámara de edición 2D.
action_editing_slice = Corte de edición
action_editing_slice_desc = Define la posición del corte vertical de edición en la vista 2D.
action_export_vcode = Exportar Visual Code ...
action_export_vcode_desc = Exporta el módulo de código visual actual.
action_extrude_linedef = Extruir linedef
action_extrude_linedef_desc = Extruye la linedef la distancia dada y crea un nuevo sector. El ángulo aplica una rotación opcional alrededor del eje de la linedef.
action_extrude_sector = Extruir sector
action_extrude_sector_desc = Activa la extrusión de superficie en sectores seleccionados, opcionalmente con cara trasera abierta.
action_first_p_camera = Cámara 3D en primera persona
action_first_p_camera_desc = Renderiza la escena con una cámara 3D en primera persona.
action_gate_door = Puerta / Portón
action_gate_door_desc = Crea un hueco con puerta / portón en el sector de perfil seleccionado.
action_import_vcode = Importar Visual Code ...
action_import_vcode_desc = Importa un módulo de código visual.
action_iso_camera = Cámara 3D isométrica
action_iso_camera_desc = Renderiza la escena con una cámara 3D isométrica.
action_minimize = Minimizar
action_minimize_desc = Minimiza el editor / dock.
action_new_tile = Nueva baldosa
action_new_tile_desc = Crea una nueva baldosa con fotogramas del tamaño indicado.
action_orbit_camera = Cámara 3D orbital
action_orbit_camera_desc = Renderiza la escena con una cámara 3D orbital.
action_recess = Hueco
action_recess_desc = Crea un hueco en el sector de perfil seleccionado.
action_relief = Relieve
action_relief_desc = Crea un relieve en el sector de perfil seleccionado.
action_set_edit_surface = Fijar superficie de edición
action_set_edit_surface_desc = Hace que la superficie seleccionada sea el perfil 2D activo para editar.
action_set_tile_material = Establecer material de la baldosa
action_set_tile_material_desc = Aplica los atributos de material a todos los píxeles de la baldosa.
action_split = Dividir
action_split_desc = Divide las linedefs seleccionadas añadiendo un punto medio. El nuevo punto se añade a todos los sectores de la linedef.
action_toggle_edit_geo = Alternar geometría de edición
action_toggle_edit_geo_desc = Alterna la visibilidad de la superposición de geometría de edición.
action_toggle_rect_geo = Alternar geometría de rectángulo
action_toggle_rect_geo_desc = La geometría creada por la herramienta Rect no se muestra por defecto en el editor 2D. Esta acción alterna su visibilidad.
action_import_palette = Importar paleta ...
action_import_palette_desc = Importa una paleta de Paint.net
action_clear_palette = Limpiar paleta
action_clear_palette_desc = Limpia la paleta
action_remap_tile = Remapear baldosa
action_remap_tile_desc = Remapea los colores de la baldosa a la paleta

# Tools
tool_game = Herramienta de juego (G). Si el servidor está en marcha, los eventos de entrada se envían al juego.
tool_linedef = Herramienta de linedef (L). Crear definiciones de línea y sectores.
tool_rect = Herramienta de rectángulo (R). Clic para dibujar la baldosa actual. Shift-clic para borrar.
tool_sector = Herramienta de sector (E).
tool_selection = Herramienta de selección (S). Mantén 'Shift' para añadir. 'Alt' para restar. Clic y arrastra para selección múltiple. 3D: Seleccionar plano de edición.
tool_selection_mac = Herramienta de selección (S). Mantén 'Shift' para añadir. 'Option' para restar. Clic y arrastra para selección múltiple. 3D: Seleccionar plano de edición.
tool_vertex = Herramienta de vértice (V). 'Shift' + clic para crear un nuevo vértice.
tool_entity = Herramienta de entidad (Y). Coloca, mueve, selecciona y elimina entidades del juego.

# Common
all = Todo
apply = Aplicar
attributes = Atributos
preview_rigging = Preview Rigging
characters = Personajes
clear = Limpiar
filter = Filtro
frames = Fotogramas
grid_size = Tamaño de cuadrícula
name = Nombre
opacity = Opacidad
eldrin_scripting = Eldrin Scripting
settings = Ajustes
size = Tamaño
palette = Paleta
visual_script = Programación visual
region = Región
regions = Regiones
items = Objetos
tilesets = Conjuntos de baldosas
screens = Pantallas
assets = Recursos
fonts = Fuentes
game = Juego
character_instance = Instancia de personaje
item_instance = Instancia de objeto
debug_log = Registro de depuración
avatars = Avatares
body_markers = Marcadores corporales
anchors = Anclajes
skin_light = Piel clara
skin_dark = Piel oscura
torso = Torso
legs = Piernas
hair = Cabello
eyes = Ojos
hands = Manos
feet = Pies
enabled = Activado

# Info
info_server_started = El servidor se ha iniciado
info_update_check = Comprobando actualizaciones...
info_welcome = ¡Bienvenido a Eldiron! Visita Eldiron.com para información y proyectos de ejemplo.

status_tile_editor_copy_texture = Copied texture to clipboard.
status_tile_editor_copy_selection = Copied selection to clipboard.
status_tile_editor_cut_selection = Cut selection to clipboard.
status_tile_editor_paste_preview_active = Paste preview active. Move mouse, Enter to apply, click or Escape to cancel.
status_tile_editor_paste_preview_canceled = Paste preview canceled.
status_tile_editor_paste_applied = Paste applied.
status_tile_editor_paste_no_valid_target = Paste preview: no valid target pixels at this position.

# Avatar Anchor
status_avatar_anchor_set_main = Set main-hand anchor.
status_avatar_anchor_set_off = Set off-hand anchor.
status_avatar_anchor_clear_main = Cleared main-hand anchor.
status_avatar_anchor_clear_off = Cleared off-hand anchor.
avatar_anchor_main = Anchor: Main Hand
avatar_anchor_off = Anchor: Off Hand
