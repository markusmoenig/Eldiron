# Menu
## Menu File
menu_file = 文件
menu_new = 新建
menu_open = 打开
menu_save = 保存
menu_save_as = 另存为
## Menu Edit
menu_edit = 编辑
menu_undo = 撤销
menu_redo = 重做
menu_cut = 剪切
menu_copy = 复制
menu_paste = 粘贴
menu_apply_action = 应用操作
# Menu Game
menu_play = 启动
menu_pause = 暂停
menu_stop = 停止

# Widgets
## Dock
dock_auto = 自动
## Node Editor
node_editor_create_button = 创建图形
## Render Editor
render_editor_trace_button = 开始追踪
## Tilemap
tilemap_add_button = 添加图块

# Status
## Actions
status_logo_button = 打开 Eldiron 官网
status_open_button = 打开已有的 Eldiron 项目
status_save_button = 保存当前项目
status_save_as_button = 将当前项目保存为新文件
status_undo_button = 撤销上一步操作
status_redo_button = 重做上一步操作
status_play_button = 启动游戏服务器以进行实时编辑和调试
status_pause_button = 暂停 点击以单步执行游戏服务器
status_stop_button = 停止游戏服务器
status_game_input_button = 游戏运行时，将输入转向游戏而不是编辑器
status_time_slider = 调整服务器时间
status_update_button = 更新应用程序
status_patreon_button = 访问 Eldiron 的 Patreon 页面，感谢支持。
status_help_button = 点击任意 UI 元素以访问 Eldiron 在线文档。
## Sidebar
status_project_add_button = 添加到项目
status_project_remove_button = 从项目中移除条目
status_project_import_button = 导入到项目
status_project_export_button = 从项目导出
## Dock
status_dock_action_apply = 应用当前操作
status_dock_action_auto = 自动应用操作
## Effect Picker
status_effect_picker_filter_edit = 显示包含指定文本的图块
## Map Editor
status_map_editor_grid_sub_div = 网格的细分级别
## Node Editor
status_node_editor_graph_id = 地图中图形的 ID
status_node_editor_create_button = 将来源应用于选中的几何体
status_node_editor_fx_node_button = 用于创建特殊效果（如光照或粒子）的节点
status_node_editor_render_nodes_button = 用于全局和局部渲染流程的节点
status_node_editor_mesh_nodes_button = 控制和修改地形及网格生成的节点
status_node_editor_shapefx_nodes_button = 附加到几何和形状以生成颜色和图案的节点
## Shape Picker
status_shape_picker_filter_edit = 显示包含指定文本的图块
## Tilemap Editor
status_tilemap_editor_clear_button = 清除当前选择
status_tilemap_editor_add_button = 添加当前选中的图块
## Tile Picker
status_tile_picker_filter_edit = 显示包含指定文本的图块
## Tilemap
status_tilemap_clear_button = 清除当前选择
status_tilemap_add_button = 添加当前选中的图块
## Tiles
status_tiles_filter_edit = 显示包含指定标签的图块
## World Editor
status_world_editor_brush_radius = 控制画笔在世界单位中的大小
status_world_editor_brush_falloff = 控制画笔强度从中心衰减的速度
status_world_editor_brush_strength = 画笔在中心的最大强度
status_world_editor_brush_fixed = 固定画笔使用的固定地形高度

# Actions
action_add_arch = 添加拱形
action_add_arch_desc = 添加拱形（曲线折线），替换选中的线段。
action_apply_tile = 应用图块
action_apply_tile_desc = 将当前图块应用到选中的区域。
action_clear_profile = 清除轮廓
action_clear_profile_desc = 清除区域中的轮廓特性（凹槽、浮雕、门 / 门洞）。
action_clear_tile = 清除图块
action_clear_tile_desc = 清除选中区域中的图块。
action_copy_tile_id = 复制图块 ID
action_copy_tile_id_desc = 将图块的 ID 复制到剪贴板，以便稍后在代码编辑器中使用。
action_create_center_vertex = 创建中心顶点
action_create_center_vertex_desc = 在选中区域的中心创建一个新顶点。
action_create_linedef = 创建线段
action_create_linedef_desc = 在两个顶点之间创建一条新线段。
action_create_sector = 创建区域
action_create_sector_desc = 从选中的顶点创建新的区域 / 表面。顶点必须构成闭合回路（我们会自动排序）。
action_duplicate_tile = 复制图块
action_duplicate_tile_desc = 复制当前选中的图块。
action_edit_linedef = 编辑线段
action_edit_linedef_desc = 编辑选中线段的属性。
action_edit_maximize = 编辑 / 最大化
action_edit_maximize_desc = 打开当前停靠窗口的编辑器或将其最大化。
action_edit_sector = 编辑区域
action_edit_sector_desc = 编辑选中区域的属性。
action_edit_tile = 编辑图块元数据
action_edit_tile_desc = 编辑当前选中图块的元数据。
action_edit_vertex = 编辑顶点
action_edit_vertex_desc = 编辑选中顶点的属性。XZ 位置是地面 / 2D 平面坐标。Y 位置是高度。可选将顶点设为地形控制点或为顶点指定公告板图块。
action_editing_camera = 2D 相机
action_editing_camera_desc = 使用 2D 编辑相机渲染场景。
action_editing_slice = 编辑切片
action_editing_slice_desc = 设置 2D 编辑视图中垂直编辑切片的位置。
action_export_vcode = 导出可视化代码 ...
action_export_vcode_desc = 导出当前的可视化代码模块。
action_extrude_linedef = 拉伸线段
action_extrude_linedef_desc = 以给定距离拉伸线段并创建一个新的区域。角度用于围绕线段轴的可选旋转。
action_extrude_sector = 拉伸区域
action_extrude_sector_desc = 在选中区域设置表面拉伸，可选启用打开背面。
action_first_p_camera = 3D 第一人称相机
action_first_p_camera_desc = 使用 3D 第一人称相机渲染场景。
action_gate_door = 门 / 门洞
action_gate_door_desc = 在选中的轮廓区域创建带门 / 门洞的开口。
action_import_vcode = 导入可视化代码 ...
action_import_vcode_desc = 导入一个可视化代码模块。
action_iso_camera = 3D 等距相机
action_iso_camera_desc = 使用 3D 等距相机渲染场景。
action_minimize = 最小化
action_minimize_desc = 最小化编辑器 / 停靠窗口。
action_new_tile = 新建图块
action_new_tile_desc = 以指定帧大小创建新图块。
action_orbit_camera = 3D 轨道相机
action_orbit_camera_desc = 使用 3D 轨道相机渲染场景。
action_recess = 凹槽
action_recess_desc = 在选中的轮廓区域创建凹槽。
action_relief = 浮雕
action_relief_desc = 在选中的轮廓区域创建浮雕效果。
action_set_edit_surface = 设置编辑面
action_set_edit_surface_desc = 将选中的表面设为活动 2D 轮廓进行编辑。若尚无轮廓，会为该表面创建一个。要返回区域地图，请点击工具栏中的 Region。
action_set_tile_material = 设置图块材质
action_set_tile_material_desc = 将材质属性应用到图块的所有像素。
action_split = 拆分
action_split_desc = 通过添加中点拆分选中的线段。新点会添加到所有包含该线段的区域中。
action_toggle_edit_geo = 切换编辑几何
action_toggle_edit_geo_desc = 切换编辑几何覆盖层的可见性。
action_toggle_rect_geo = 切换矩形几何
action_toggle_rect_geo_desc = 矩形工具创建的几何默认不在 2D 编辑器中显示，此操作切换其可见性。
action_import_palette = 导入调色板 ...
action_import_palette_desc = 导入 Paint.net 调色板
action_clear_palette = 清空调色板
action_clear_palette_desc = 清空当前调色板
action_remap_tile = 重新映射图块
action_remap_tile_desc = 将图块的颜色映射到调色板

# Tools
tool_game = 游戏工具 (G)。服务器运行时输入事件会发送到游戏
tool_linedef = 线段工具 (L)。创建线段定义和区域
tool_rect = 矩形工具 (R)。点击绘制当前图块，按住 Shift 点击删除
tool_sector = 区域工具 (E)
tool_selection = 选择工具 (S)。按住 Shift 添加，按住 Alt 减去，点击并拖动进行多选。3D：选择编辑平面
tool_selection_mac = 选择工具 (S)。按住 Shift 添加，按住 Option 减去，点击并拖动进行多选。3D：选择编辑平面
tool_vertex = 顶点工具 (V)。按住 'Shift' + 单击可创建新顶点。
tool_entity = 实体工具 (Y)。放置、移动、选择和删除游戏实体。

# Common
all = 全部
apply = 应用
attributes = 属性
preview_rigging = Preview Rigging
characters = 角色
clear = 清除
filter = 筛选
frames = 帧数
grid_size = 网格大小
name = 名称
opacity = 不透明度
eldrin_scripting = Eldrin Scripting
settings = 设置
size = 尺寸
visual_script = 可视化脚本
region = 区域
regions = 区域
items = 物品
tilesets = 图块集
screens = 屏幕
assets = 资源
fonts = 字体
game = 游戏
character_instance = 角色实例
item_instance = 物品实例
palette = 调色板
debug_log = 调试日志
avatars = 头像
body_markers = 身体标记
anchors = 锚点
skin_light = 浅色皮肤
skin_dark = 深色皮肤
torso = 躯干
legs = 腿部
hair = 头发
eyes = 眼睛
hands = 手部
feet = 脚部
enabled = 已启用

# Info
info_server_started = 服务器已启动
info_update_check = 正在检查更新
info_welcome = 欢迎使用 Eldiron 访问 Eldiron.com 获取信息和示例项目

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
