# Menu
## Menu File
menu_file = 檔案
menu_new = 新建
menu_open = 開啟
menu_save = 儲存
menu_save_as = 另存為
## Menu Edit
menu_edit = 編輯
menu_undo = 復原
menu_redo = 重做
menu_cut = 剪下
menu_copy = 複製
menu_paste = 貼上
menu_apply_action = 套用操作
# Menu Game
menu_play = 啟動
menu_pause = 暫停
menu_stop = 停止

# Widgets
## Dock
dock_auto = 自動
## Node Editor
node_editor_create_button = 建立圖形
## Render Editor
render_editor_trace_button = 開始追蹤
## Tilemap
tilemap_add_button = 新增圖塊

# Status
## Actions
status_logo_button = 開啟 Eldiron 官網
status_open_button = 開啟現有的 Eldiron 專案
status_save_button = 儲存目前專案
status_save_as_button = 將目前專案另存為新檔案
status_undo_button = 復原上一個動作
status_redo_button = 重做上一個動作
status_play_button = 啟動遊戲伺服器以進行即時編輯與除錯
status_pause_button = 暫停 點擊以單步執行遊戲伺服器
status_stop_button = 停止遊戲伺服器
status_game_input_button = 遊戲執行時，將輸入導向遊戲而非編輯器
status_time_slider = 調整伺服器時間
status_update_button = 更新應用程式
status_patreon_button = 造訪 Eldiron 的 Patreon 頁面，感謝您的支持。
status_help_button = 點擊任何 UI 元素以造訪 Eldiron 線上文件。
## Sidebar
status_project_add_button = 加入到專案
status_project_remove_button = 從專案中移除項目
status_project_import_button = 匯入到專案
status_project_export_button = 從專案匯出
## Dock
status_dock_action_apply = 套用目前操作
status_dock_action_auto = 自動套用操作
## Effect Picker
status_effect_picker_filter_edit = 顯示包含指定文字的圖塊
## Map Editor
status_map_editor_grid_sub_div = 網格的細分等級
## Node Editor
status_node_editor_graph_id = 地圖中圖形的 ID
status_node_editor_create_button = 將來源套用到選取的幾何體
status_node_editor_fx_node_button = 用來建立特殊效果如光照或粒子的節點
status_node_editor_render_nodes_button = 用於全域與局部渲染流程的節點
status_node_editor_mesh_nodes_button = 控制並修改地形與網格生成的節點
status_node_editor_shapefx_nodes_button = 附加到幾何與形狀以產生顏色與圖案的節點
## Shape Picker
status_shape_picker_filter_edit = 顯示包含指定文字的圖塊
## Tilemap Editor
status_tilemap_editor_clear_button = 清除目前選取
status_tilemap_editor_add_button = 新增目前選取的圖塊
## Tile Picker
status_tile_picker_filter_edit = 顯示包含指定文字的圖塊
## Tilemap
status_tilemap_clear_button = 清除目前選取
status_tilemap_add_button = 新增目前選取的圖塊
## Tiles
status_tiles_filter_edit = 顯示包含指定標籤的圖塊
## World Editor
status_world_editor_brush_radius = 控制畫筆在世界單位中的大小
status_world_editor_brush_falloff = 控制畫筆強度自中心衰減的速度
status_world_editor_brush_strength = 畫筆在中心的最大強度
status_world_editor_brush_fixed = 固定畫筆使用的固定地形高度

# Actions
action_add_arch = 新增拱形
action_add_arch_desc = 新增拱形（曲線折線），取代所選的線段。
action_apply_tile = 套用圖塊
action_apply_tile_desc = 將目前的圖塊套用到所選區域。
action_clear_profile = 清除輪廓
action_clear_profile_desc = 清除區域中的輪廓特性（凹槽、浮雕、門／門洞）。
action_clear_tile = 清除圖塊
action_clear_tile_desc = 清除所選區域的圖塊。
action_copy_tile_id = 複製圖塊 ID
action_copy_tile_id_desc = 將圖塊的 ID 複製到剪貼簿，方便稍後在程式碼編輯器中使用。
action_create_center_vertex = 建立中心頂點
action_create_center_vertex_desc = 在所選區域的中心建立新頂點。
action_create_linedef = 建立線段
action_create_linedef_desc = 在兩個頂點之間建立新的線段。
action_create_sector = 建立區域
action_create_sector_desc = 使用所選頂點建立新的區域／表面。頂點必須形成封閉迴圈（會自動排序）。
action_duplicate_tile = 複製圖塊
action_duplicate_tile_desc = 複製目前選中的圖塊。
action_edit_linedef = 編輯線段
action_edit_linedef_desc = 編輯所選線段的屬性。
action_edit_maximize = 編輯／最大化
action_edit_maximize_desc = 開啟目前 dock 的編輯器或將其最大化。
action_edit_sector = 編輯區域
action_edit_sector_desc = 編輯所選區域的屬性。
action_edit_tile = 編輯圖塊中繼資料
action_edit_tile_desc = 編輯目前選中的圖塊的中繼資料。
action_edit_vertex = 編輯頂點
action_edit_vertex_desc = 編輯所選頂點的屬性。XZ 為地面／2D 平面位置。Y 位置是高度。可選擇將頂點設定為地形控制點或為頂點指定廣告牌圖塊。
action_editing_camera = 2D 相機
action_editing_camera_desc = 使用 2D 編輯相機渲染場景。
action_editing_slice = 編輯切片
action_editing_slice_desc = 設定 2D 編輯檢視中垂直編輯切片的位置。
action_export_vcode = 匯出視覺化程式碼 ...
action_export_vcode_desc = 匯出目前的視覺化程式碼模組。
action_extrude_linedef = 拉伸線段
action_extrude_linedef_desc = 以指定距離拉伸線段並建立新區域。角度可選擇沿線段軸旋轉。
action_extrude_sector = 拉伸區域
action_extrude_sector_desc = 為所選區域設定表面拉伸，可選擇開放背面。
action_first_p_camera = 3D 第一人稱相機
action_first_p_camera_desc = 使用 3D 第一人稱相機渲染場景。
action_gate_door = 門／門洞
action_gate_door_desc = 在所選剖面區域建立帶門／門洞的開口。
action_import_vcode = 匯入視覺化程式碼 ...
action_import_vcode_desc = 匯入一個視覺化程式碼模組。
action_iso_camera = 3D 等距相機
action_iso_camera_desc = 使用 3D 等距相機渲染場景。
action_minimize = 最小化
action_minimize_desc = 最小化編輯器／dock。
action_new_tile = 新圖塊
action_new_tile_desc = 建立具有指定幀尺寸的新圖塊。
action_orbit_camera = 3D 環繞相機
action_orbit_camera_desc = 使用 3D 環繞相機渲染場景。
action_recess = 凹槽
action_recess_desc = 在所選剖面區域建立凹槽。
action_relief = 浮雕
action_relief_desc = 在所選剖面區域建立浮雕。
action_set_edit_surface = 設定編輯面
action_set_edit_surface_desc = 將所選表面設為活動的 2D 剖面以進行編輯。若尚無剖面會為此表面建立一個。要返回區域地圖，請點擊工具列中的 Region。
action_set_tile_material = 設定圖塊材質
action_set_tile_material_desc = 將材質屬性套用到圖塊的所有像素。
action_split = 分割
action_split_desc = 在中間新增節點以分割所選線段。新節點會加到所有包含該線段的區域中。
action_toggle_edit_geo = 切換編輯幾何
action_toggle_edit_geo_desc = 切換編輯幾何疊加層的可見性。
action_toggle_rect_geo = 切換矩形幾何
action_toggle_rect_geo_desc = 由矩形工具建立的幾何預設不在 2D 編輯器中顯示，此操作用來切換其可見性。
action_import_palette = 匯入調色盤 ...
action_import_palette_desc = 匯入 Paint.net 調色盤
action_clear_palette = 清空調色盤
action_clear_palette_desc = 清空調色盤
action_remap_tile = 重新對應圖塊
action_remap_tile_desc = 將圖塊的顏色對應到調色盤

# Tools
tool_game = 遊戲工具 (G)。伺服器運行時輸入事件會傳送到遊戲
tool_linedef = 線段工具 (L)。建立線段定義與區域
tool_rect = 矩形工具 (R)。點擊繪製當前圖塊，按住 Shift 點擊刪除
tool_sector = 區域工具 (E)
tool_selection = 選取工具 (S)。按住 Shift 新增，按住 Alt 減去，點擊並拖曳進行多重選取。3D：選取編輯平面
tool_selection_mac = 選取工具 (S)。按住 Shift 新增，按住 Option 減去，點擊並拖曳進行多重選取。3D：選取編輯平面
tool_vertex = 頂點工具 (V)。按住 'Shift' + 點擊可建立新頂點。
tool_entity = 實體工具 (Y)。放置、移動、選擇和刪除遊戲實體。

# Common
all = 全部
apply = 套用
attributes = 屬性
preview_rigging = Preview Rigging
characters = 角色
clear = 清除
filter = 篩選
frames = 幀數
grid_size = 網格大小
name = 名稱
opacity = 不透明度
eldrin_scripting = Eldrin Scripting
settings = 設定
size = 大小
visual_script = 視覺化腳本
region = 區域
regions = 區域
items = 物品
tilesets = 圖塊集
screens = 畫面
assets = 資源
fonts = 字型
game = 遊戲
character_instance = 角色實例
item_instance = 物品實例
palette = 調色盤
debug_log = 除錯日誌
avatars = 頭像
body_markers = 身體標記
anchors = 錨點
skin_light = 淺色皮膚
skin_dark = 深色皮膚
torso = 軀幹
legs = 腿部
hair = 頭髮
eyes = 眼睛
hands = 手部
feet = 腳部
enabled = 已啟用

# Info
info_server_started = 伺服器已啟動
info_update_check = 正在檢查更新
info_welcome = 歡迎使用 Eldiron 造訪 Eldiron.com 以取得資訊與示例專案

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
