# Menu
## Menu File
menu_file = Файл
menu_new = Новый
menu_close = Закрыть
menu_open = Открыть...
menu_save = Сохранить...
menu_save_as = Сохранить как...
new_project = Новый проект
## Menu Edit
menu_edit = Правка
menu_undo = Отменить
menu_redo = Повторить
menu_cut = Вырезать
menu_copy = Копировать
menu_paste = Вставить
menu_apply_action = Применить действие
# Menu Game
menu_play = Запустить
menu_pause = Пауза
menu_stop = Остановить

# Widgets
## Dock
dock_auto = Автоматически
## Node Editor
node_editor_create_button = Создать граф
## Render Editor
render_editor_trace_button = Начать трассировку
## Tilemap
tilemap_add_button = Добавить тайл(ы)

# Status
## Actions
status_logo_button = Открыть сайт Eldiron ...
status_open_button = Открыть существующий проект Eldiron...
status_save_button = Сохранить текущий проект.
status_save_as_button = Сохранить текущий проект в новый файл.
status_undo_button = Отменить последнее действие.
status_redo_button = Повторить последнее действие.
status_play_button = Запустить игровой сервер для живого редактирования и отладки.
status_pause_button = Пауза. Нажмите для покадрового шага игрового сервера.
status_stop_button = Остановить игровой сервер.
status_game_input_button = Направляет ввод в игру вместо редактора, когда игра запущена.
status_time_slider = Настроить время сервера.
status_update_button = Обновить приложение.
status_patreon_button = Посетить страницу Eldiron на Patreon. Спасибо за поддержку.
status_help_button = Нажмите любой элемент интерфейса, чтобы открыть онлайн-документацию Eldiron.
status_create_cutout_failed = Для выреза нужно выбрать минимум три точки линии поверхности на одной 3D-грани.
status_create_cutout_open_loop = Для выреза нужны замкнутые контуры линий поверхности. Сначала завершите или замкните выбранную направляющую.
status_create_cutout_multiple_faces = Для выреза пока нужны все выбранные направляющие контуры на одной базовой поверхности.
## Sidebar
status_project_add_button = Добавить в проект.
status_project_remove_button = Удалить элемент из проекта.
status_project_duplicate_button = Дублировать текущий элемент проекта.
status_project_import_button = Импортировать в проект.
status_project_export_button = Экспортировать из проекта.
## Dock
status_dock_action_apply = Применить текущее действие.
status_dock_action_auto = Автоприменение действий.
## Effect Picker
status_effect_picker_filter_edit = Показать тайлы, содержащие заданный текст.
## Map Editor
status_map_editor_grid_sub_div = Разбиение сетки / шаг привязки.
## Node Editor
status_node_editor_graph_id = ID графа внутри карты.
status_node_editor_create_button = Применить источник к выбранной геометрии.
status_node_editor_fx_node_button = Узлы, создающие спецэффекты, такие как свет или частицы.
status_node_editor_render_nodes_button = Узлы глобальных и локальных графов рендеринга.
status_node_editor_mesh_nodes_button = Узлы, управляющие и изменяющие создание рельефа и мешей.
status_node_editor_shapefx_nodes_button = Узлы, прикрепляющиеся к геометрии и фигурам и создающие цвета и узоры.
## Shape Picker
status_shape_picker_filter_edit = Показать тайлы, содержащие заданный текст.
## Tilemap Editor
status_tilemap_editor_clear_button = Очистить текущий выбор.
status_tilemap_editor_add_button = Добавить текущий выбор тайлов.
## Tile Picker
status_tile_picker_filter_edit = Показать тайлы, содержащие заданный текст.
## Tilemap
status_tilemap_clear_button = Очистить текущий выбор.
status_tilemap_add_button = Добавить текущий выбор тайлов.
## Tiles
status_tiles_filter_edit = Показать тайлы, содержащие указанные теги.
status_tiles_apply_tile = Применить выбранный тайл к выбранному слоту значка.
status_tiles_clear_tile = Очистить выбранный слот значка.
## World Editor
status_world_editor_brush_radius = Управляет размером кисти в мировых единицах.
status_world_editor_brush_falloff = Определяет, как быстро сила кисти спадает от центра.
status_world_editor_brush_strength = Максимальная интенсивность кисти в центре.
status_world_editor_brush_fixed = Фиксированная высота рельефа, используемая кистью «Fixed».

# Actions
action_apply_tile = Применить тайл
action_apply_tile_desc = Применяет текущий источник тайла к выбранным секторам или 3D-граням.
action_clear_tile = Удалить тайл
action_clear_tile_desc = Удаляет тайлы из выбранных секторов или 3D-граней.
action_copy_tile_id = Скопировать ID тайла
action_copy_tile_id_desc = Копирует ID тайла в буфер обмена для последующего использования в редакторе кода.
action_create_center_vertex = Создать центральную вершину
action_create_center_vertex_desc = Создает новую вершину в центре выбранных секторов.
action_create_linedef = Создать линедеф
action_create_linedef_desc = Создает новый линедеф между двумя вершинами.
action_create_cutout = Создать вырез
action_create_cutout_desc = Вырезает отверстие по выбранному замкнутому контуру 3D-линий поверхности через объект.
action_create_groove = Создать канавку
action_create_groove_desc = Преобразует выбранные 3D-линии поверхности в постоянную углубленную геометрию.
action_create_ridge = Создать выступ
action_create_ridge_desc = Преобразует выбранные 3D-линии поверхности в постоянную приподнятую геометрию.
action_create_sector = Создать сектор
action_create_sector_desc = Создает новый сектор / поверхность из выбранных вершин. Вершины должны образовывать замкнутую петлю (мы упорядочим их автоматически).
action_create_geometry_box = Создать коробку
action_create_geometry_box_desc = Создает напрямую редактируемый 3D-объект-коробку.
action_duplicate_tile = Дублировать тайл
action_duplicate_tile_desc = Дублирует выбранный тайл.
action_duplicate_surface_detail = Дублировать деталь поверхности
action_duplicate_surface_detail_desc = Дублирует выбранные направляющие 3D-линий поверхности на исходной грани.
action_toggle_surface_curve = Кривая поверхности
action_toggle_surface_curve_desc = Задает выбранные 3D-сегменты поверхности или сегменты между выбранными точками как линии или настраиваемые дуги.
action_edit_face_texture = Редактировать текстуру грани
action_edit_face_texture_desc = Редактирует смещение, масштаб и поворот 3D-текстуры по граням для выбранных граней или целых выбранных геометрических объектов.
action_edit_geometry = Редактировать геометрию
action_edit_geometry_desc = Изменяет позицию, размер, видимость, твердость и группу выбранной 3D-геометрии.
action_face_extrude = Выдавить грань
action_face_extrude_desc = Выдавить выбранные 3D-грани на заданную величину.
action_face_cut_opening = Вырезать проем
action_face_cut_opening_desc = Вырезать прямоугольный проем через выбранную 3D-грань и противоположную грань.
action_face_inset = Вставка грани
action_face_inset_desc = Вдавить выбранные 3D-грани на заданную величину.
action_face_delete = Удалить грань
action_face_delete_desc = Удалить выбранные 3D-грани и выбрать их граничные вершины.
action_face_merge = Объединить грани
action_face_merge_desc = Объединить выбранные соединенные 3D-грани в одну редактируемую грань.
action_face_subdivide = Разделить грань
action_face_subdivide_desc = Разделить выбранные четырехугольные грани на меньшие редактируемые грани.
action_edit_linedef = Редактировать линедеф
action_edit_linedef_desc = Изменить атрибуты выбранного линедефа.
action_editing_slice = Сечение редактирования
action_editing_slice_desc = Устанавливает позицию вертикального сечения в окне 2D-редактирования.
action_edit_maximize = Редактировать / Развернуть
action_edit_maximize_desc = Открывает редактор текущего дока или разворачивает его.
action_edit_sector = Редактировать сектор
action_edit_sector_desc = Изменить атрибуты выбранного сектора.
action_edit_tile = Редактировать метаданные тайла
action_edit_tile_desc = Изменить метаданные выбранного тайла.
action_edit_vertex = Редактировать вершину
action_edit_vertex_desc = Изменить атрибуты выбранной вершины. Позиции XZ — это координаты на земле/в 2D-плоскости. Включите вершину как контрольную точку рельефа или задайте для нее билборд-тайл.
action_editing_camera = 2D-камера
action_editing_camera_desc = Рендер сцены с использованием 2D-камеры редактирования.
action_export_vcode = Экспортировать визуальный код ...
action_export_vcode_desc = Экспортировать текущий модуль визуального кода.
action_filter_edit_geo = Фильтр геометрии
action_filter_edit_geo_desc = Фильтрует отображение редактора, чтобы можно было изолировать сгенерированную геометрию подземелья во время редактирования.
action_build_procedural = Построить процедурно
action_build_procedural_desc = Создаёт процедурную геометрию карты из настроек текущего региона.
action_build_procedural_help = Преобразует конфигурацию [procedural] текущего региона в редактируемую геометрию карты.
action_first_p_camera = 3D-камера от первого лица
action_first_p_camera_desc = Рендер сцены с использованием 3D-камеры от первого лица.
status_firstp_fly_nav_on = FirstP fly navigation on. Pointer from center turns/looks, WASD moves, Space exits.
status_firstp_fly_nav_rmb_on = FirstP fly navigation on. Hold right mouse to look, WASD moves, release right mouse or press Escape to exit.
status_firstp_fly_nav_off = FirstP fly navigation off.
status_camera_2d = Edit the map in 2D.
status_camera_orbit_macos = Edit the map with a 3D orbit camera. Wheel zooms. Right-drag or Alt-drag orbits. Cmd-drag or Shift-wheel pans. Arrow keys move the target.
status_camera_orbit_other = Edit the map with a 3D orbit camera. Wheel zooms. Right-drag or Alt-drag orbits. Ctrl-drag or Shift-wheel pans. Arrow keys move the target.
status_camera_iso_macos = Edit the map in 3D isometric view. Wheel zooms. Right-drag, Alt-drag, Cmd-drag, or Shift-wheel pans. Arrow keys move the target.
status_camera_iso_other = Edit the map in 3D isometric view. Wheel zooms. Right-drag, Alt-drag, Ctrl-drag, or Shift-wheel pans. Arrow keys move the target.
status_camera_firstp = Edit the map in 3D first person view. Hold right mouse and use WASD to fly. Space toggles fly mode for touchpads.
action_tile_procedural_style = Стиль
action_tile_procedural_kind = Тип
action_tile_procedural_weight = Вес
action_import_vcode = Импортировать визуальный код ...
action_import_vcode_desc = Импортировать модуль визуального кода.
action_iso_camera = 3D-изометрическая камера
action_iso_camera_desc = Рендер сцены с использованием 3D-изометрической камеры.
action_minimize = Свернуть
action_minimize_desc = Сворачивает редактор / док.
action_new_tile = Новый тайл
action_new_tile_desc = Создает новый тайл с кадрами указанного размера.
action_orbit_camera = 3D-орбитальная камера
action_orbit_camera_desc = Рендер сцены с использованием 3D-орбитальной камеры.
action_set_edit_surface = Задать поверхность редактирования
action_set_edit_surface_desc = Делает выбранную поверхность активным 2D-профилем для редактирования.
action_set_tile_material = Задать материал тайла
action_set_tile_material_desc = Устанавливает атрибуты материала для всех пикселей тайла.
action_split = Разделить
action_split_desc = Делит выбранный(е) линедеф(ы), добавляя середину. Новая точка добавляется во все секторы, частью которых является линедеф.
action_toggle_edit_geo = Переключить геометрию редактирования
action_toggle_edit_geo_desc = Переключает видимость наложения геометрии редактирования.
action_toggle_rect_geo = Переключить прямоугольную геометрию
action_toggle_rect_geo_desc = Геометрия, созданная инструментом Rect, по умолчанию не показывается в 2D-редакторе. Это действие переключает ее видимость.
action_import_palette = Импортировать палитру ...
action_import_palette_desc = Импортировать палитру Paint.net
action_clear_palette = Очистить палитру
action_clear_palette_desc = Очищает палитру
action_remap_tile = Перекодировать тайл
action_remap_tile_desc = Перекодирует цвета тайла согласно палитре.

# Tools
tool_game = Инструмент игры (K). Если сервер запущен, события ввода отправляются в игру.
tool_linedef = Инструмент линедефов / ребер (L). Создание 2D-линий и редактирование ребер 3D-геометрии.
tool_object = Инструмент объектов (G). Выбирайте и перемещайте напрямую редактируемые 3D-объекты.
tool_rect = Инструмент прямоугольников (R). Клик — рисует текущий тайл. Shift-клик — удаляет. Alt/Opt-клик — взять тайл с карты.
tool_sector = Инструмент секторов / граней (E). Выбирает секторы в 2D или грани в 3D.
tool_vertex = Инструмент вершин (V). Shift + клик — создать новую вершину.
tool_entity = Инструмент сущностей (Y). Размещайте, перемещайте, выделяйте и удаляйте игровые сущности.
tool_organic = Organic Paint Tool (O). Paint volumetric organic detail using the active brush graph.
hud_geometry_op_move = MOVE
hud_geometry_op_size = SIZE
status_hud_geometry_op_move = Операция гизмо объекта: перемещение (M).
status_hud_geometry_op_size = Операция гизмо объекта: изменение размера (S).
status_geometry_empty_selection = 3D-выбор: G = объект, E = грань, V = вершина, L = ребро.
status_geometry_object_selection = Объект выбран: M = переместить, S = размер.
status_geometry_face_selection = Грань выбрана: +/- = выдавить/вдавить, [] = вверх/вниз, Delete = удалить.
status_geometry_vertex_selection = Вершина выбрана: F = заполнить, X = разделить ребро, M = объединить, L = петля ребер, [] = вверх/вниз, Delete = удалить.
status_geometry_edge_selection = Ребро выбрано: F = заполнить, X = разделить ребро, M = объединить, L = петля ребер, [] = вверх/вниз, Delete = удалить.
status_geometry_surface_selection = Деталь поверхности выбрана: Shift = добавить, Alt = убрать, L = связанный контур.
status_geometry_surface_loop_selection = Замкнутая деталь поверхности выбрана: Shift = добавить, Alt = убрать, L = связанный контур.
organic_dock_title = Органические кисти
organic_toggle_active = Активно
organic_toggle_deactive = Неактивно
organic_mode_free = Свободно
organic_mode_locked = Заблокировано
status_organic_toggle_visibility = Включает или отключает отображение слоя органической покраски.
status_organic_lock_mode = Свободно рисует по всем поверхностям. Заблокировано рисует только по выбранному сектору или активной поверхности.
status_organic_clear = Очищает нарисованные органические детали. В заблокированном режиме очищает только выбранный сектор или активную поверхность.

# Common
all = Все
apply = Применить
attributes = Атрибуты
preview_rigging = Preview Rigging
clear = Очистить
filter = Фильтр
frames = Кадры
grid_size = Размер сетки
name = Имя
opacity = Непрозрачность
roughness = Шероховатость
metallic = Металличность
emissive = Свечение
eldrin_scripting = Скрипты Eldrin
settings = Настройки
size = Размер
visual_script = Визуальное скриптование
region = Регион
regions = Регионы
characters = Персонажи
items = Предметы
tilesets = Тайлсеты
screens = Экраны
assets = Ресурсы
fonts = Шрифты
game = Игра
character_instance = Экземпляр персонажа
item_instance = Экземпляр предмета
opacity = Непрозрачность
palette = Палитра
debug_log = Журнал отладки
avatars = Аватары
body_markers = Маркеры тела
anchors = Якоря
skin_light = Светлая кожа
skin_dark = Тёмная кожа
torso = Торс
arms = Руки
legs = Ноги
hair = Волосы
eyes = Глаза
hands = Руки
feet = Ступни
enabled = Включено

# Info
info_server_started = Сервер запущен
info_update_check = Проверка обновлений...
info_welcome = Добро пожаловать в Eldiron! Посетите Eldiron.com за информацией и примерами проектов.

status_tile_editor_copy_texture = Текстура скопирована в буфер обмена.
status_tile_editor_copy_selection = Выделение скопировано в буфер обмена.
status_tile_editor_cut_selection = Выделение вырезано в буфер обмена.
status_tile_editor_paste_preview_active = Предпросмотр вставки активен. Перемещайте мышь, нажмите Enter для применения, клик или Escape для отмены.
status_tile_editor_paste_preview_canceled = Предпросмотр вставки отменён.
status_tile_editor_paste_applied = Вставка выполнена.
status_tile_editor_paste_no_valid_target = Предпросмотр вставки: в этой позиции нет подходящих целевых пикселей.

# Avatar Anchor
status_avatar_anchor_set_main = Установлена основная якорная точка руки.
status_avatar_anchor_set_off = Установлена якорная точка второй руки.
status_avatar_anchor_clear_main = Основная якорная точка руки очищена.
status_avatar_anchor_clear_off = Якорная точка второй руки очищена.
avatar_anchor_main = Якорь: основная рука
avatar_anchor_off = Якорь: вторая рука
action_duplicate = Дублировать
action_duplicate_desc = Дублировать выбранную геометрию со смещением по XYZ.
action_copy_vcode = Копировать визуальный код
action_copy_vcode_desc = Копирует текущий модуль визуального кода в буфер обмена.
action_paste_vcode = Вставить визуальный код
action_paste_vcode_desc = Импортирует модуль визуального кода из буфера обмена.
tool_authoring = Авторинг
status_tool_authoring = Режим авторинга. Вводите метаданные для секторов, linedef, сущностей и предметов.
tool_text_play = Текстовая игра
status_tool_text_play = Текстовый игровой режим для Game Tool. Заменяет обычный игровой вид текстовым выводом и вводом команд, чтобы можно было играть через текст.
authoring_select_prompt = Режим авторинга. Выберите сектор, linedef, сущность или предмет.
authoring_title_prefix = Режим авторинга. Введите метаданные для
authoring_title = Режим авторинга. Введите метаданные для {$target}.
authoring_target_sector = Сектор
authoring_target_linedef = Linedef
authoring_target_character = Персонаж
authoring_target_item = Предмет
tool_game = Game Tool (K). Играть!
tool_builder = Builder Tool (B). Выбирайте переиспользуемые ассеты реквизита и сборок в палитре Builder.
tool_palette = Инструмент палитры (P). Редактируйте элементы палитры и применяйте цвета палитры.
tool_dungeon = Инструмент подземелий (U). Рисуйте концептуальные структуры подземелий.
builder_picker_title = Палитра Builder
builder_apply_build = Применить сборку
palette_apply_color = Применить цвет
status_palette_apply_color = Применить текущий элемент палитры к выбранной цели.
status_builder_new = Создать новый ассет графа Builder.
status_builder_collections = Коллекции Builder будут добавлены здесь позже.
status_builder_apply_build = Применить выбранный граф Builder к выбранным хостам.
status_builder_clear_build = Удалить граф Builder с выбранных хостов.
status_builder_select_asset = Выберите ассет Builder '{$asset_name}'. Дважды щёлкните или нажмите Enter, чтобы открыть.
collections = Коллекции
new = Новый
