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

# Widgets
## Dock
dock_action = 操作列表
## Node Editor
node_editor_create_button = 创建图形
## Render Editor
render_editor_trace_button = 开始追踪
## Tilemap
tilemap_add_button = 添加图块

# Status
## Actions
status_action_add_arch_height = 拱形在 XY 平面的凸起高度
status_action_add_arch_segment = 拱形折线的段数
status_action_edit_linedef_name = 设置线段名称
status_action_edit_sector_name = 设置区域名称
status_action_edit_tile_role = 编辑图块的角色
status_action_edit_tile_blocking = 编辑图块是否阻挡（仅适用于 2D 游戏）
status_action_edit_tile_tags = 编辑图块标签
status_action_edit_vertex_name = 设置顶点名称
status_action_edit_vertex_x = 顶点的 X 位置
status_action_edit_vertex_y = 顶点的 Y 位置
status_action_edit_vertex_z = 顶点的 Z 位置
status_action_extrude_linedef_distance = 拉伸距离（符号决定方向）
status_action_extrude_linedef_angle = 围绕几何轴 / 法线的旋转角度
status_action_extrude_sector_surface_extrusion = 当选择区域（表面）时：开启 / 关闭该表面拉伸
status_action_extrude_sector_depth = 拉伸深度
status_action_extrude_sector_open_back = 保持背面开放；用于立面 / 室内
status_action_new_tile_size = 新图块的尺寸
status_action_new_tile_frames = 新图块的帧数
status_action_recess_depth = 凹槽深度
status_action_recess_target = 凹槽可附着在前面或背面
status_action_recess_tiles = 凹槽的封盖与侧边（侧框）图块
status_action_relief_height = 浮雕高度
status_action_relief_target = 浮雕可附着在前面或背面
status_action_relief_tiles = 浮雕的封盖与侧边（侧框）图块
## Menubar
status_logo_button = 打开 Eldiron 官网
status_open_button = 打开已有的 Eldiron 项目
status_save_button = 保存当前项目
status_save_as_button = 将当前项目保存为新文件
status_undo_button = 撤销上一步操作
status_redo_button = 重做上一步操作
status_play_button = 启动服务器以进行实时编辑和调试
status_pause_button = 暂停 点击以单步执行服务器
status_stop_button = 停止服务器
status_time_slider = 调整服务器时间
status_update_button = 更新应用程序
status_patreon_button = 访问我的 Patreon 页面
## Sidebar
status_project_add_button = 添加到项目
status_project_remove_button = 从项目中移除条目
status_project_import_button = 导入到项目
status_project_export_button = 从项目导出
## Dock
status_dock_action_apply = 应用当前操作
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
action_add_arch_desc = 添加拱形（弧线折线），替换选中的线段。
action_apply_tile = 将当前图块应用到选中的区域。
action_clear_tile = 清除选中区域中的图块。
action_copy_tile_id = 将图块的 ID 复制到剪贴板，以便稍后在代码编辑器中使用。
action_create_center_vertex = 在选中区域的中心创建一个新顶点。
action_create_linedef = 在两个顶点之间创建一条新线段。
action_create_sector = 从选中的顶点创建新的区域或表面。顶点必须构成闭合回路（我们会自动排序）。
action_duplicate_tile = 复制当前选中的图块。
action_edit_linedef_name = 线段名称
action_edit_linedef_desc = 编辑选中线段的属性。
action_edit_maximize = 打开当前停靠窗口的编辑器或将其最大化。
action_edit_sector_name = 区域名称
action_edit_sector_desc = 编辑选中区域的属性。
action_edit_tile_desc = 编辑当前选中图块的元数据。
action_edit_vertex_name = 顶点名称
action_edit_vertex_x = X 位置
action_edit_vertex_y = Y 位置
action_edit_vertex_z = Z 位置
action_edit_vertex_desc = 编辑选中顶点的属性。XZ 位置是地面 / 2D 平面坐标，Y 位置是高度。
action_editing_camera = 使用 2D 编辑相机渲染场景。
action_extrude_linedef_desc = 以给定距离拉伸线段并创建一个新的区域。角度用于围绕线段轴进行可选旋转。
action_extrude_sector_surface_extrusion = 表面拉伸
action_extrude_sector_open_back = 打开背面
action_extrude_sector_desc = 在选中区域设置表面拉伸，可选启用打开背面。
action_first_p_camera = 使用 3D 第一人称视角渲染场景。
action_iso_camera = 使用 3D 等距相机渲染场景。
action_minimize = 最小化编辑器或停靠窗口。
action_new_tile_desc = 以指定帧大小创建新图块。
action_orbit_camera = 使用 3D 轨道相机渲染场景。
action_recess_desc = 在选中的轮廓区域创建凹陷。
action_relief_desc = 在选中的轮廓区域创建浮雕效果。
action_set_edit_surface = 将选中的表面设为活动的 2D 轮廓进行编辑。Eldiron 将切换到 2D 视图，如果尚未存在轮廓，则为此表面创建一个。要返回区域地图，请点击工具栏中的 Region。
action_split = 通过添加中点拆分选中的线段。新点将添加到所有包含该线段的区域中。
action_toggle_edit_geo = 切换编辑几何覆盖层的可见性。
action_toggle_rect_geo = 使用矩形工具创建的几何体默认不在 2D 编辑器中显示。此操作切换其可见性。

# Tools
tool_game = 游戏工具 (G)。服务器运行时输入事件会发送到游戏
tool_linedef = 线段工具 (L)。创建线段定义和区域
tool_rect = 矩形工具 (R)。点击绘制当前图块，按住 Shift 点击删除
tool_sector = 区域工具 (E)
tool_selection = 选择工具 (S)。按住 Shift 添加，按住 Alt 减去，点击并拖动进行多选。3D：选择编辑平面
tool_selection_mac = 选择工具 (S)。按住 Shift 添加，按住 Option 减去，点击并拖动进行多选。3D：选择编辑平面
tool_vertex = 顶点工具 (V)

# Common
all = 全部
angle = 角度
apply = 应用
attributes = 属性
blocking = 阻挡
character = 角色
clear = 清除
depth = 深度
distance = 距离
dungeon = 地牢
effect = 效果
filter = 筛选
frames = 帧数
grid_size = 网格大小
height = 高度
icon = 图标
icons = 图标
manmade = 人造
mountain = 山脉
name = 名称
nature = 自然
opacity = 不透明度
python_code = Python 代码
role = 角色类型
segments = 段数
settings = 设置
size = 尺寸
tags = 标签
target = 目标
ui = 界面
visual_script = 可视化脚本
water = 水体

# Info
info_server_started = 服务器已启动
info_update_check = 正在检查更新
info_welcome = 欢迎使用 Eldiron 访问 Eldiron.com 获取信息和示例项目
