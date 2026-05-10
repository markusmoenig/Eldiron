# Menu
## Menu File
menu_file = Archivo
menu_new = Nuevo
menu_close = Cerrar
menu_open = Abrir...
menu_save = Guardar...
menu_save_as = Guardar como...
new_project = Nuevo proyecto
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
status_create_cutout_failed = Crear recorte necesita al menos tres puntos de línea de superficie seleccionados en una cara 3D.
status_create_cutout_open_loop = Crear recorte necesita bucles cerrados de líneas de superficie. Termina o cierra primero la guía seleccionada.
status_create_cutout_multiple_faces = Crear recorte actualmente necesita todos los bucles guía seleccionados en una sola superficie anfitriona.
## Sidebar
status_project_add_button = Añadir al proyecto.
status_project_remove_button = Eliminar un elemento del proyecto.
status_project_duplicate_button = Duplicar el elemento actual del proyecto.
status_project_import_button = Importar al proyecto.
status_project_export_button = Exportar del proyecto.
## Dock
status_dock_action_apply = Aplicar la acción actual.
status_dock_action_auto = Aplicar acciones automáticamente.
## Effect Picker
status_effect_picker_filter_edit = Mostrar baldosas que contengan el texto dado.
## Map Editor
status_map_editor_grid_sub_div = Subdivisión de cuadrícula / paso de ajuste.
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
status_tiles_apply_tile = Aplicar el mosaico seleccionado a la ranura de icono seleccionada.
status_tiles_clear_tile = Limpiar la ranura de icono seleccionada.
## World Editor
status_world_editor_brush_radius = Controla el tamaño del pincel en unidades de mundo.
status_world_editor_brush_falloff = Controla qué tan rápido decae la intensidad del pincel desde el centro.
status_world_editor_brush_strength = Intensidad máxima del pincel en el centro.
status_world_editor_brush_fixed = Altura de terreno fija usada por el pincel 'Fixed'.

# Actions
action_apply_tile = Aplicar baldosa
action_apply_tile_desc = Aplica la fuente de baldosa actual a los sectores o caras 3D seleccionados.
action_clear_tile = Limpiar baldosa
action_clear_tile_desc = Elimina las baldosas de los sectores o caras 3D seleccionados.
action_copy_tile_id = Copiar ID de baldosa
action_copy_tile_id_desc = Copia el ID de la baldosa al portapapeles para usarlo después en el editor de código.
action_create_center_vertex = Crear vértice central
action_create_center_vertex_desc = Crea un nuevo vértice en el centro de los sectores seleccionados.
action_create_linedef = Crear linedef
action_create_linedef_desc = Crea una nueva linedef entre dos vértices.
action_create_cutout = Crear recorte
action_create_cutout_desc = Corta una abertura desde el bucle cerrado de líneas de superficie 3D seleccionado a través del objeto.
action_create_groove = Crear ranura
action_create_groove_desc = Convierte las líneas de superficie 3D seleccionadas en geometría hundida persistente.
action_create_ridge = Crear relieve
action_create_ridge_desc = Convierte las líneas de superficie 3D seleccionadas en geometría elevada persistente.
action_create_sector = Crear sector
action_create_sector_desc = Crea un nuevo sector/superficie a partir de los vértices seleccionados. Deben formar un bucle cerrado (lo ordenamos automáticamente).
action_create_geometry_box = Crear caja
action_create_geometry_box_desc = Crea un objeto de caja 3D editable directamente.
action_duplicate_tile = Duplicar baldosa
action_duplicate_tile_desc = Duplica la baldosa actualmente seleccionada.
action_duplicate_surface_detail = Duplicar detalle de superficie
action_duplicate_surface_detail_desc = Duplica las guías de líneas de superficie 3D seleccionadas sobre la cara anfitriona.
action_toggle_surface_curve = Curva de superficie
action_toggle_surface_curve_desc = Define segmentos de superficie 3D seleccionados, o segmentos entre puntos seleccionados, como líneas o arcos configurables.
action_edit_face_texture = Editar textura de cara
action_edit_face_texture_desc = Edita el desplazamiento, la escala y la rotación de textura por cara para caras seleccionadas u objetos de geometría completos.
action_edit_geometry = Editar geometría
action_edit_geometry_desc = Edita posición, tamaño, visibilidad, solidez y grupo de la geometría 3D seleccionada.
action_face_extrude = Extruir cara
action_face_extrude_desc = Extruye las caras 3D seleccionadas por la cantidad indicada.
action_face_cut_opening = Cortar abertura
action_face_cut_opening_desc = Corta una abertura rectangular a través de la cara 3D seleccionada y su cara opuesta.
action_face_inset = Insertar cara
action_face_inset_desc = Inserta las caras 3D seleccionadas por la cantidad indicada.
action_face_delete = Eliminar cara
action_face_delete_desc = Elimina las caras 3D seleccionadas y selecciona sus vértices de borde.
action_face_merge = Fusionar caras
action_face_merge_desc = Fusiona las caras 3D conectadas seleccionadas en una sola cara editable.
action_face_subdivide = Subdividir cara
action_face_subdivide_desc = Subdivide las caras cuadradas seleccionadas en caras editables más pequeñas.
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
action_filter_edit_geo = Filtrar geometría
action_filter_edit_geo_desc = Filtra el render del editor para poder aislar la geometría de dungeon generada mientras editas.
action_build_procedural = Construir procedural
action_build_procedural_desc = Construye geometría procedural del mapa desde la configuración de la región actual.
action_build_procedural_help = Convierte la configuración [procedural] de la región actual en geometría editable del mapa.
action_first_p_camera = Cámara 3D en primera persona
action_first_p_camera_desc = Renderiza la escena con una cámara 3D en primera persona.
status_firstp_fly_nav_on = FirstP fly navigation on. Pointer from center turns/looks, WASD moves, Space exits.
status_firstp_fly_nav_rmb_on = FirstP fly navigation on. Hold right mouse to look, WASD moves, release right mouse or press Escape to exit.
status_firstp_fly_nav_off = FirstP fly navigation off.
status_camera_2d = Edit the map in 2D.
status_camera_orbit_macos = Edit the map with a 3D orbit camera. Wheel zooms. Right-drag or Alt-drag orbits. Cmd-drag or Shift-wheel pans. Arrow keys move the target.
status_camera_orbit_other = Edit the map with a 3D orbit camera. Wheel zooms. Right-drag or Alt-drag orbits. Ctrl-drag or Shift-wheel pans. Arrow keys move the target.
status_camera_iso_macos = Edit the map in 3D isometric view. Wheel zooms. Right-drag, Alt-drag, Cmd-drag, or Shift-wheel pans. Arrow keys move the target.
status_camera_iso_other = Edit the map in 3D isometric view. Wheel zooms. Right-drag, Alt-drag, Ctrl-drag, or Shift-wheel pans. Arrow keys move the target.
status_camera_firstp = Edit the map in 3D first person view. Hold right mouse and use WASD to fly. Space toggles fly mode for touchpads.
action_tile_procedural_style = Estilo
action_tile_procedural_kind = Tipo
action_tile_procedural_weight = Peso
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
tool_game = Herramienta de juego (K). Si el servidor está en marcha, los eventos de entrada se envían al juego.
tool_linedef = Herramienta de linedef / arista (L). Crear definiciones de línea 2D y editar aristas de geometría 3D.
tool_object = Herramienta de objeto (G). Selecciona y mueve objetos 3D editables directamente.
tool_rect = Herramienta de rectángulo (R). Clic para dibujar la baldosa actual. Shift-clic para borrar. Alt/Opt-clic para tomarla del mapa.
tool_sector = Herramienta de sector / cara (E). Selecciona sectores en 2D o caras en 3D.
tool_vertex = Herramienta de vértice (V). 'Shift' + clic para crear un nuevo vértice.
tool_entity = Herramienta de entidad (Y). Coloca, mueve, selecciona y elimina entidades del juego.
tool_organic = Organic Paint Tool (O). Paint volumetric organic detail using the active brush graph.
hud_geometry_op_move = MOVE
hud_geometry_op_size = SIZE
status_hud_geometry_op_move = Operación del gizmo de objeto: mover (M).
status_hud_geometry_op_size = Operación del gizmo de objeto: redimensionar (S).
status_geometry_empty_selection = Selección 3D: G = Objeto, E = Cara, V = Vértice, L = Arista.
status_geometry_object_selection = Objeto seleccionado: M = Mover, S = Tamaño, R = Rotar 90°, T = Aplicar tile, Cmd/Ctrl+D = Duplicar, Delete = Eliminar.
status_geometry_face_selection = Cara seleccionada: +/- = Empujar/Tirar, [] = Subir/Bajar, Delete = Eliminar.
status_geometry_vertex_selection = Vértice seleccionado: F = Rellenar, X = Dividir arista, M = Fusionar, L = Bucle de aristas, [] = Subir/Bajar, Delete = Eliminar.
status_geometry_edge_selection = Arista seleccionada: F = Rellenar, X = Dividir arista, M = Fusionar, L = Bucle de aristas, [] = Subir/Bajar, Delete = Eliminar.
status_geometry_surface_selection = Detalle de superficie seleccionado: Shift = añadir, Alt = quitar, L = guía conectada.
status_geometry_surface_loop_selection = Detalle de superficie cerrado seleccionado: Shift = añadir, Alt = quitar, L = guía conectada.
organic_dock_title = Pinceles orgánicos
organic_toggle_active = Activo
organic_toggle_deactive = Desactivo
organic_mode_free = Libre
organic_mode_locked = Bloqueado
status_organic_toggle_visibility = Activa o desactiva la visualización de la capa de pintura orgánica.
status_organic_lock_mode = Libre pinta sobre todas las superficies. Bloqueado pinta solo sobre el sector seleccionado o la superficie activa.
status_organic_clear = Borra el detalle orgánico pintado. En modo bloqueado solo borra el sector seleccionado o la superficie activa.

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
roughness = Rugosidad
metallic = Metálico
emissive = Emisivo
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
arms = Brazos
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
avatar_anchor_main = Ancla: mano principal
avatar_anchor_off = Ancla: mano secundaria
action_duplicate = Duplicar
action_duplicate_desc = Duplica la geometría seleccionada con un desplazamiento XYZ.
action_copy_vcode = Copiar Visual Code
action_copy_vcode_desc = Copia el módulo actual de Visual Code al portapapeles.
action_paste_vcode = Pegar Visual Code
action_paste_vcode_desc = Importa un módulo de Visual Code desde el portapapeles.
tool_authoring = Autoría
status_tool_authoring = Modo de autoría. Introduce metadatos para sectores, linedefs, entidades y objetos.
tool_text_play = Juego de texto
status_tool_text_play = Modo de juego en texto para la herramienta Game Tool. Sustituye la vista normal del juego por salida de texto y entrada de comandos para que puedas jugar mediante texto.
authoring_select_prompt = Modo de autoría. Selecciona un sector, linedef, entidad u objeto.
authoring_title_prefix = Modo de autoría. Introduce metadatos para
authoring_title = Modo de autoría. Introduce metadatos para {$target}.
authoring_target_sector = Sector
authoring_target_linedef = Linedef
authoring_target_character = Personaje
authoring_target_item = Objeto
tool_game = Game Tool (K). ¡Juega!
tool_builder = Builder Tool (B). Selecciona recursos reutilizables de utilería y ensamblaje en el selector del constructor.
tool_palette = Herramienta de paleta (P). Edita entradas de la paleta y aplica colores de la paleta.
tool_dungeon = Herramienta de mazmorras (U). Pinta estructuras conceptuales de mazmorras.
builder_picker_title = Selector del constructor
builder_apply_build = Aplicar construcción
palette_apply_color = Aplicar color
status_palette_apply_color = Aplica la entrada actual de la paleta al objetivo seleccionado.
status_builder_new = Crear un nuevo recurso de grafo del constructor.
status_builder_collections = Las colecciones del constructor se añadirán aquí más adelante.
status_builder_apply_build = Aplicar el grafo del constructor seleccionado a los hosts seleccionados.
status_builder_clear_build = Quitar el grafo del constructor de los hosts seleccionados.
status_builder_select_asset = Selecciona el recurso del constructor '{$asset_name}'. Haz doble clic o pulsa Retorno para abrirlo.
collections = Colecciones
new = Nuevo
