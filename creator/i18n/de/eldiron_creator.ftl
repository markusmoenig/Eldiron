# Menu
## Menu File
menu_file = Datei
menu_new = Neu
menu_open = Öffnen...
menu_save = Speichern...
menu_save_as = Speichern unter...
## Menu Edit
menu_edit = Bearbeiten
menu_undo = Rückgängig
menu_redo = Wiederholen
menu_cut = Ausschneiden
menu_copy = Kopieren
menu_paste = Einfügen
menu_apply_action = Aktion anwenden
# Menu Game
menu_play = Starten
menu_pause = Pause
menu_stop = Stoppen

# Widgets
## Dock
dock_auto = Automatisch
## Node Editor
node_editor_create_button = Graph erstellen
## Render Editor
render_editor_trace_button = Trace starten
## Tilemap
tilemap_add_button = Kachel(en) hinzufügen

# Status
## Actions
status_logo_button = Die Eldiron-Website öffnen ...
status_open_button = Ein bestehendes Eldiron-Projekt öffnen...
status_save_button = Das aktuelle Projekt speichern.
status_save_as_button = Das aktuelle Projekt in eine neue Datei speichern.
status_undo_button = Die letzte Aktion rückgängig machen.
status_redo_button = Die letzte Aktion wiederholen.
status_play_button = Den Spielserver für Live-Bearbeitung und Debugging starten.
status_pause_button = Pausieren. Klicken zum Einzelschritt des Spielservers.
status_stop_button = Den Spielserver stoppen.
status_game_input_button = Leitet Eingaben an das Spiel statt an den Editor weiter, wenn das Spiel läuft.
status_time_slider = Die Serverzeit anpassen.
status_update_button = Anwendung aktualisieren.
status_patreon_button = Die Eldiron-Patreon-Seite besuchen. Danke für die Unterstützung.
status_help_button = Auf ein beliebiges UI-Element klicken, um die Eldiron-Online-Dokumentation zu öffnen.
## Sidebar
status_project_add_button = Zum Projekt hinzufügen.
status_project_remove_button = Ein Element aus dem Projekt entfernen.
status_project_import_button = In das Projekt importieren.
status_project_export_button = Aus dem Projekt exportieren.
## Dock
status_dock_action_apply = Die aktuelle Aktion anwenden.
status_dock_action_auto = Aktionen automatisch anwenden.
## Effect Picker
status_effect_picker_filter_edit = Kacheln anzeigen, die den angegebenen Text enthalten.
## Map Editor
status_map_editor_grid_sub_div = Der Unterteilungsgrad des Gitters.
## Node Editor
status_node_editor_graph_id = Die ID des Graphen innerhalb der Karte.
status_node_editor_create_button = Die Quelle auf die ausgewählte Geometrie anwenden.
status_node_editor_fx_node_button = Nodes, die Spezialeffekte wie Licht oder Partikel erzeugen.
status_node_editor_render_nodes_button = Nodes für die globalen und lokalen Render-Graphs.
status_node_editor_mesh_nodes_button = Nodes, die Terrain- und Mesh-Erzeugung steuern und verändern.
status_node_editor_shapefx_nodes_button = Nodes, die an Geometrie und Formen hängen und Farben sowie Muster erzeugen.
## Shape Picker
status_shape_picker_filter_edit = Kacheln anzeigen, die den angegebenen Text enthalten.
## Tilemap Editor
status_tilemap_editor_clear_button = Die aktuelle Auswahl löschen.
status_tilemap_editor_add_button = Die aktuelle Kachelauswahl hinzufügen.
## Tile Picker
status_tile_picker_filter_edit = Kacheln anzeigen, die den angegebenen Text enthalten.
## Tilemap
status_tilemap_clear_button = Die aktuelle Auswahl löschen.
status_tilemap_add_button = Die aktuelle Kachelauswahl hinzufügen.
## Tiles
status_tiles_filter_edit = Kacheln anzeigen, die die angegebenen Tags enthalten.
## World Editor
status_world_editor_brush_radius = Steuert die Größe des Pinsels in Welt-Einheiten.
status_world_editor_brush_falloff = Steuert, wie schnell die Pinselstärke vom Zentrum abfällt.
status_world_editor_brush_strength = Maximale Intensität des Pinsels im Zentrum.
status_world_editor_brush_fixed = Feste Geländehöhe, die vom 'Fixed'-Pinsel verwendet wird.

# Actions
action_add_arch = Bogen hinzufügen
action_add_arch_desc = Fügt einen Bogen (gebogene Polylinie) hinzu und ersetzt die ausgewählte(n) Linedef(s).
action_apply_tile = Kachel anwenden
action_apply_tile_desc = Wendet die aktuelle Kachel auf die ausgewählten Sektoren an.
action_clear_profile = Profil löschen
action_clear_profile_desc = Entfernt ein mögliches Profil-Feature (Vertiefung, Relief, Tor / Tür) aus dem Sektor.
action_clear_tile = Kachel entfernen
action_clear_tile_desc = Entfernt die Kacheln aus den ausgewählten Sektoren.
action_copy_tile_id = Tile-ID kopieren
action_copy_tile_id_desc = Kopiert die ID der Kachel in die Zwischenablage für die spätere Nutzung im Code-Editor.
action_create_center_vertex = Zentralen Vertex erzeugen
action_create_center_vertex_desc = Erstellt einen neuen Vertex in der Mitte der ausgewählten Sektoren.
action_create_linedef = Linedef erstellen
action_create_linedef_desc = Erstellt eine neue Linedef zwischen zwei Vertices.
action_create_sector = Sektor erstellen
action_create_sector_desc = Erstellt einen neuen Sektor / eine neue Fläche aus den ausgewählten Vertices. Die Vertices müssen einen geschlossenen Loop bilden (wird automatisch sortiert).
action_duplicate_tile = Kachel duplizieren
action_duplicate_tile_desc = Dupliziert die aktuell ausgewählte Kachel.
action_edit_linedef = Linedef bearbeiten
action_edit_linedef_desc = Bearbeitet die Attribute der ausgewählten Linedef.
action_edit_maximize = Bearbeiten / Maximieren
action_edit_maximize_desc = Öffnet den Editor für den aktuellen Dock oder maximiert ihn.
action_edit_sector = Sektor bearbeiten
action_edit_sector_desc = Bearbeitet die Attribute des ausgewählten Sektors.
action_edit_tile = Tile-Metadaten bearbeiten
action_edit_tile_desc = Bearbeitet die Metadaten der aktuell ausgewählten Kachel.
action_edit_vertex = Vertex bearbeiten
action_edit_vertex_desc = Bearbeitet die Attribute des ausgewählten Vertex. Die XZ-Positionen sind die Boden- / 2D-Positionen. Die Y-Position ist die Höhe. Optional kann der Vertex als Terrain-Kontrollpunkt aktiviert oder eine Billboard-Kachel zugewiesen werden.
action_editing_camera = 2D-Kamera
action_editing_camera_desc = Szene mit der 2D-Bearbeitungskamera rendern.
action_editing_slice = Bearbeitungsschnitt
action_editing_slice_desc = Setzt die Position des vertikalen Bearbeitungsschnitts in der 2D-Ansicht.
action_export_vcode = Visual Code exportieren ...
action_export_vcode_desc = Das aktuelle Visual-Code-Modul exportieren.
action_extrude_linedef = Linedef extrudieren
action_extrude_linedef_desc = Extrudiert die Linedef um die angegebene Distanz und erstellt einen neuen Sektor. Der Winkel wendet optional eine Rotation um die Linedef-Achse an.
action_extrude_sector = Sektor extrudieren
action_extrude_sector_desc = Aktiviert Oberflächenextrusion auf ausgewählten Sektoren, optional mit offener Rückseite.
action_first_p_camera = 3D-Ego-Kamera
action_first_p_camera_desc = Szene mit einer 3D-Ego-Kamera rendern.
action_gate_door = Tor / Tür
action_gate_door_desc = Erstellt eine Öffnung mit Tor / Tür im ausgewählten Profilsektor.
action_import_vcode = Visual Code importieren ...
action_import_vcode_desc = Ein Visual-Code-Modul importieren.
action_iso_camera = 3D-Iso-Kamera
action_iso_camera_desc = Szene mit einer 3D-Iso-Kamera rendern.
action_minimize = Minimieren
action_minimize_desc = Minimiert den Editor / Dock.
action_new_tile = Neue Kachel
action_new_tile_desc = Erstellt eine neue Kachel mit Frames der angegebenen Größe.
action_orbit_camera = 3D-Orbit-Kamera
action_orbit_camera_desc = Szene mit einer 3D-Orbit-Kamera rendern.
action_recess = Vertiefung
action_recess_desc = Erstellt eine Vertiefung im ausgewählten Profilsektor.
action_relief = Relief
action_relief_desc = Erstellt ein Relief (Prägung) im ausgewählten Profilsektor.
action_set_edit_surface = Bearbeitungsfläche setzen
action_set_edit_surface_desc = Setzt die ausgewählte Fläche als aktive 2D-Profilfläche für die Bearbeitung.
action_set_tile_material = Kachelmaterial festlegen
action_set_tile_material_desc = Setzt die Materialattribute für alle Pixel der Kachel.
action_split = Teilen
action_split_desc = Teilt die ausgewählten Linedefs, indem ein Mittelpunkt hinzugefügt wird. Der neue Punkt wird allen Sektoren hinzugefügt, zu denen die Linedef gehört.
action_toggle_edit_geo = Bearbeitungsgeometrie umschalten
action_toggle_edit_geo_desc = Schaltet die Sichtbarkeit der Bearbeitungsgeometrie-Overlay um.
action_toggle_rect_geo = Rect-Geometrie umschalten
action_toggle_rect_geo_desc = Von der Rect-Geometrie erstellte Geometrie wird im 2D-Editor standardmäßig nicht angezeigt. Diese Aktion schaltet die Sichtbarkeit um.
action_import_palette = Palette importieren ...
action_import_palette_desc = Eine Paint.net-Palette importieren
action_clear_palette = Palette leeren
action_clear_palette_desc = Leert die Palette
action_remap_tile = Kachel neu zuordnen
action_remap_tile_desc = Ordnet die Farben der Kachel der Palette zu

# Tools
tool_game = Spiel-Tool (G). Wenn der Server läuft, werden Eingabeereignisse an das Spiel gesendet.
tool_linedef = Linedef-Tool (L). Liniendefinitionen und Sektoren erstellen.
tool_rect = Rect-Tool (R). Klicken, um die aktuelle Kachel zu zeichnen. Shift-Klick zum Löschen.
tool_sector = Sektor-Tool (E).
tool_selection = Auswahl-Tool (S). Halte 'Shift', um hinzuzufügen. 'Alt', um zu subtrahieren. Klicken und ziehen für Mehrfachauswahl. 3D: Bearbeitungsebene auswählen.
tool_selection_mac = Auswahl-Tool (S). Halte 'Shift', um hinzuzufügen. 'Option', um zu subtrahieren. Klicken und ziehen für Mehrfachauswahl. 3D: Bearbeitungsebene auswählen.
tool_vertex = Vertex-Tool (V). 'Shift' + Klick erstellt einen neuen Vertex.
tool_entity = Entity-Tool (Y). Spiel-Entitäten platzieren, verschieben, auswählen und löschen.

# Common
all = Alle
apply = Anwenden
attributes = Attribute
preview_rigging = Preview Rigging
clear = Leeren
filter = Filter
frames = Frames
grid_size = Rastergröße
name = Name
opacity = Deckkraft
eldrin_scripting = Eldrin Scripting
settings = Einstellungen
size = Größe
visual_script = Visuelles Scripting
region = Region
regions = Regionen
characters = Charaktere
items = Gegenstände
tilesets = Kachelsets
screens = Bildschirme
assets = Assets
fonts = Schriften
game = Spiel
character_instance = Charakterinstanz
item_instance = Gegenstandsinstanz
palette = Palette
debug_log = Debug-Protokoll
avatars = Avatare
body_markers = Körpermarkierungen
anchors = Anker
skin_light = Helle Haut
skin_dark = Dunkle Haut
torso = Torso
legs = Beine
hair = Haare
eyes = Augen
hands = Hände
feet = Füße
enabled = Aktiviert

# Info
info_server_started = Server wurde gestartet
info_update_check = Updates werden geprüft...
info_welcome = Willkommen bei Eldiron! Besuche Eldiron.com für Informationen und Beispielprojekte.

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
