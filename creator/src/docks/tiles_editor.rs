use crate::docks::tiles_editor_undo::*;
use crate::editor::{TOOLLIST, UNDOMANAGER};
use crate::prelude::*;
use shared::tilegraph::{
    TileGraphDocument, TileGraphSubgraphResolver as SharedTileGraphSubgraphResolver,
    TileNodeGraphExchange, TileNodeGraphState, TileNodeKind, TileNodeState,
    flatten_graph_exchange_with,
};

const TILE_NODE_CANVAS_VIEW: &str = "Tile Node Skeleton Graph View";
const TILE_NODE_SETTINGS_LAYOUT: &str = "Tile Node Settings";

#[derive(Clone, Copy, PartialEq)]
enum TilesEditorMode {
    Pixel,
    NodeSkeleton,
}

struct ProjectTileSubgraphResolver<'a> {
    project: &'a Project,
}

impl SharedTileGraphSubgraphResolver for ProjectTileSubgraphResolver<'_> {
    fn resolve_subgraph_state(
        &self,
        subgraph_id: Uuid,
    ) -> Option<shared::tilegraph::TileNodeGraphState> {
        self.project
            .tile_subgraphs
            .get(&subgraph_id)
            .and_then(|subgraph| {
                serde_json::from_str::<shared::tilegraph::TileNodeGraphState>(&subgraph.graph_data)
                    .ok()
            })
            .map(|mut state| {
                state.ensure_root();
                state
            })
    }
}

pub struct TilesEditorDock {
    mode: TilesEditorMode,
    current_node_group_id: Option<Uuid>,
    zoom: f32,
    show_grid: bool,
    tile_node: Uuid,
    palette_node: Uuid,
    grid_node: Uuid,
    body_markers_node: Uuid,

    // Per-context undo stacks (keyed by tile_id for tiles, avatar_id for avatar frames)
    tile_undos: FxHashMap<Uuid, TileEditorUndo>,
    current_tile_id: Option<Uuid>,
    /// The current undo key — derived from the editing context.
    current_undo_key: Option<Uuid>,
    max_undo: usize,

    /// When true, the minimap cycles through animation frames.
    anim_preview: bool,
    paste_preview_texture: Option<rusterix::Texture>,
    paste_preview_pos: Option<Vec2<i32>>,
}

impl Dock for TilesEditorDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            mode: TilesEditorMode::Pixel,
            current_node_group_id: None,
            zoom: 5.0,
            show_grid: true,
            tile_node: Uuid::new_v4(),
            palette_node: Uuid::new_v4(),
            grid_node: Uuid::new_v4(),
            body_markers_node: Uuid::new_v4(),
            tile_undos: FxHashMap::default(),
            current_tile_id: None,
            current_undo_key: None,
            max_undo: 30,
            anim_preview: false,
            paste_preview_texture: None,
            paste_preview_pos: None,
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut rgba_layout = TheRGBALayout::new(TheId::named("Tile Editor Dock RGBA Layout"));
        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
            rgba_view.set_supports_external_zoom(true);
            rgba_view.set_background([116, 116, 116, 255]);
            // rgba_view.set_grid(Some(1));
            // rgba_view.set_grid_color([20, 20, 20, 255]);
            // rgba_view.set_dont_show_grid(true);
            rgba_view.set_dont_show_grid(!self.show_grid);
            rgba_view.set_show_transparency(true);
            rgba_view.set_mode(TheRGBAViewMode::TileEditor);
            let mut c = WHITE;
            c[3] = 128;
            rgba_view.set_hover_color(Some(c));
        }

        canvas.set_layout(rgba_layout);

        let mut stack_canvas = TheCanvas::new();
        let mut stack_layout = TheStackLayout::new(TheId::named("Pixel Editor Stack Layout"));
        stack_layout.limiter_mut().set_max_width(305);

        // Tree

        let mut palette_canvas = TheCanvas::default();
        let mut palette_tree_layout = TheTreeLayout::new(TheId::named("Tile Editor Tree"));
        palette_tree_layout.limiter_mut().set_max_width(305);
        let root = palette_tree_layout.get_root();

        // Tile
        let mut tile_node: TheTreeNode =
            TheTreeNode::new(TheId::named_with_id("Tile", self.tile_node));
        tile_node.set_open(true);

        let mut item = TheTreeItem::new(TheId::named("Tile Size"));
        item.set_text(fl!("size"));

        let mut edit = TheTextLineEdit::new(TheId::named("Tile Size Edit"));
        edit.set_value(TheValue::Int(0));
        item.add_widget_column(150, Box::new(edit));
        tile_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Tile Frames"));
        item.set_text(fl!("frames"));

        let mut edit = TheTextLineEdit::new(TheId::named("Tile Frame Edit"));
        edit.set_value(TheValue::Int(0));
        item.add_widget_column(150, Box::new(edit));
        tile_node.add_widget(Box::new(item));

        let mut item = TheTreeIcons::new(TheId::named("Tile Frame Icons"));
        item.set_icon_size(40);
        item.set_icon_count(1);
        item.set_selected_index(Some(0));
        tile_node.add_widget(Box::new(item));

        root.add_child(tile_node);

        // Palette

        let mut palette_node: TheTreeNode =
            TheTreeNode::new(TheId::named_with_id("Color", self.palette_node));
        palette_node.set_open(true);

        let mut item = TheTreeItem::new(TheId::named("Palette Opacity"));
        item.set_text(fl!("opacity"));

        let mut edit = TheTextLineEdit::new(TheId::named("Palette Opacity Edit"));
        edit.set_value(TheValue::Float(1.0));
        edit.set_range(TheValue::RangeF32(0.0..=1.0));
        item.add_widget_column(150, Box::new(edit));
        palette_node.add_widget(Box::new(item));

        // let mut item = TheTreeIcons::new(TheId::named("Palette Item"));
        // item.set_icon_count(256);
        // item.set_icons_per_row(14);
        // item.set_selected_index(Some(0));

        // palette_node.add_widget(Box::new(item));
        root.add_child(palette_node);

        // Grid
        let mut grid_node: TheTreeNode =
            TheTreeNode::new(TheId::named_with_id("Grid", self.grid_node));
        grid_node.set_open(true);

        let mut item = TheTreeItem::new(TheId::named("Grid Enabled"));
        let mut cb = TheCheckButton::new(TheId::named("Grid Enabled CB"));
        cb.set_state(TheWidgetState::Selected);
        item.add_widget_column(150, Box::new(cb));
        item.set_text(fl!("enabled"));

        grid_node.add_widget(Box::new(item));

        root.add_child(grid_node);

        //

        palette_canvas.set_layout(palette_tree_layout);

        stack_layout.add_canvas(palette_canvas);

        // Avatar

        let mut avatar_canvas = TheCanvas::default();
        let mut avatar_tree_layout = TheTreeLayout::new(TheId::named("Avatar Editor Tree"));
        avatar_tree_layout.limiter_mut().set_max_width(305);
        let root = avatar_tree_layout.get_root();

        let mut body_markers_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("body_markers"),
            self.body_markers_node,
        ));
        body_markers_node.set_open(true);

        // •	Skin Light – rgb(255, 0, 255)
        // •	Skin Dark – rgb(200, 0, 200)
        // •	Torso / Chest – rgb(0, 0, 255)
        // •	Arms / Sleeves – rgb(0, 120, 255)
        // •	Legs / Pants – rgb(0, 255, 0)
        // •	Hair – rgb(255, 255, 0)
        // •	Eyes / Face Detail – rgb(0, 255, 255)
        // •	Hands – rgb(255, 128, 0)
        // •	Feet – rgb(255, 80, 0)

        let mut item = TheTreeItem::new(TheId::named("Body: Skin Light"));
        item.set_text(fl!("skin_light"));
        item.set_background_color(TheColor::from_u8_array_3([255, 0, 255]));
        body_markers_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Body: Skin Dark"));
        item.set_text(fl!("skin_dark"));
        item.set_background_color(TheColor::from_u8_array_3([200, 0, 200]));
        body_markers_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Body: Torso"));
        item.set_text(fl!("torso"));
        item.set_background_color(TheColor::from_u8_array_3([0, 0, 255]));
        body_markers_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Body: Arms"));
        item.set_text(fl!("arms"));
        item.set_background_color(TheColor::from_u8_array_3([0, 120, 255]));
        body_markers_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Body: Legs"));
        item.set_text(fl!("legs"));
        item.set_background_color(TheColor::from_u8_array_3([0, 255, 0]));
        body_markers_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Body: Hair"));
        item.set_text(fl!("hair"));
        item.set_background_color(TheColor::from_u8_array_3([255, 255, 0]));
        body_markers_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Body: Eyes"));
        item.set_text(fl!("eyes"));
        item.set_background_color(TheColor::from_u8_array_3([0, 255, 255]));
        body_markers_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Body: Hands"));
        item.set_text(fl!("hands"));
        item.set_background_color(TheColor::from_u8_array_3([255, 128, 0]));
        body_markers_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Body: Feet"));
        item.set_text(fl!("feet"));
        item.set_background_color(TheColor::from_u8_array_3([255, 80, 0]));
        body_markers_node.add_widget(Box::new(item));

        root.add_child(body_markers_node);

        let mut anchors_node: TheTreeNode = TheTreeNode::new(TheId::named(&fl!("anchors")));
        anchors_node.set_open(true);

        let mut item = TheTreeItem::new(TheId::named("Anchor: Main"));
        item.set_text(fl!("avatar_anchor_main"));
        anchors_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Anchor: Off"));
        item.set_text(fl!("avatar_anchor_off"));
        anchors_node.add_widget(Box::new(item));

        // let mut item = TheTreeItem::new(TheId::named("Body: Extra"));
        // item.set_text(fl!("extra"));
        // item.set_background_color(TheColor::from_u8_array_3([255, 0, 0]));
        // body_markers_node.add_widget(Box::new(item));

        root.add_child(anchors_node);

        avatar_canvas.set_layout(avatar_tree_layout);

        stack_layout.add_canvas(avatar_canvas);

        stack_canvas.set_layout(stack_layout);
        canvas.set_left(stack_canvas);

        let pixel_canvas = canvas;

        let mut node_canvas = TheCanvas::new();
        let mut node_center = TheCanvas::new();

        let mut node_toolbar = TheCanvas::default();
        let traybar_widget = TheTraybar::new(TheId::empty());
        node_toolbar.set_widget(traybar_widget);
        let mut toolbar_hlayout = TheHLayout::new(TheId::named("Tile Node Toolbar"));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);
        toolbar_hlayout.set_reverse_index(Some(2));

        let mut add_button = TheTraybarButton::new(TheId::named("Tile Node Add"));
        add_button.set_text("Add".to_string());
        add_button.set_status_text("Add source, layout, transform, sculpt, or compose nodes.");
        let mut generator_menu = TheContextMenu::default();
        generator_menu.id = TheId::named("Tile Node Add Generators");
        generator_menu.add(TheContextMenuItem::new(
            "Group UV".to_string(),
            TheId::named("Tile Node Add Group UV"),
        ));
        generator_menu.add(TheContextMenuItem::new(
            "Scalar".to_string(),
            TheId::named("Tile Node Add Scalar"),
        ));
        generator_menu.add(TheContextMenuItem::new(
            "Color".to_string(),
            TheId::named("Tile Node Add Color"),
        ));
        generator_menu.add(TheContextMenuItem::new(
            "Palette Index".to_string(),
            TheId::named("Tile Node Add Palette Color"),
        ));
        generator_menu.add(TheContextMenuItem::new(
            "Make Material".to_string(),
            TheId::named("Tile Node Add Make Material"),
        ));
        generator_menu.add(TheContextMenuItem::new(
            "Noise".to_string(),
            TheId::named("Tile Node Add Noise"),
        ));
        generator_menu.add(TheContextMenuItem::new(
            "Id Random".to_string(),
            TheId::named("Tile Node Add Id Random"),
        ));
        generator_menu.add(TheContextMenuItem::new(
            "Gradient".to_string(),
            TheId::named("Tile Node Add Gradient"),
        ));

        let mut palette_menu = TheContextMenu::default();
        palette_menu.id = TheId::named("Tile Node Add Palette");
        palette_menu.add(TheContextMenuItem::new(
            "Palette Index".to_string(),
            TheId::named("Tile Node Add Palette Color"),
        ));
        palette_menu.add(TheContextMenuItem::new(
            "Nearest Palette".to_string(),
            TheId::named("Tile Node Add Nearest Palette"),
        ));

        let mut layout_menu = TheContextMenu::default();
        layout_menu.id = TheId::named("Tile Node Add Layouts");
        layout_menu.add(TheContextMenuItem::new(
            "Voronoi".to_string(),
            TheId::named("Tile Node Add Voronoi"),
        ));
        layout_menu.add(TheContextMenuItem::new(
            "Box Divide".to_string(),
            TheId::named("Tile Node Add Box Divide"),
        ));
        layout_menu.add(TheContextMenuItem::new(
            "Brick".to_string(),
            TheId::named("Tile Node Add Brick"),
        ));
        layout_menu.add(TheContextMenuItem::new(
            "Disc".to_string(),
            TheId::named("Tile Node Add Disc"),
        ));

        let mut transform_menu = TheContextMenu::default();
        transform_menu.id = TheId::named("Tile Node Add Transform");
        transform_menu.add(TheContextMenuItem::new(
            "Offset".to_string(),
            TheId::named("Tile Node Add Offset"),
        ));
        transform_menu.add(TheContextMenuItem::new(
            "Scale".to_string(),
            TheId::named("Tile Node Add Scale"),
        ));
        transform_menu.add(TheContextMenuItem::new(
            "Rotate".to_string(),
            TheId::named("Tile Node Add Rotate"),
        ));
        transform_menu.add(TheContextMenuItem::new(
            "Repeat".to_string(),
            TheId::named("Tile Node Add Repeat"),
        ));
        transform_menu.add(TheContextMenuItem::new(
            "Warp".to_string(),
            TheId::named("Tile Node Add Warp"),
        ));

        let mut compose_menu = TheContextMenu::default();
        compose_menu.id = TheId::named("Tile Node Add Compose");
        compose_menu.add(TheContextMenuItem::new(
            "Mix".to_string(),
            TheId::named("Tile Node Add Mix"),
        ));
        compose_menu.add(TheContextMenuItem::new(
            "Min".to_string(),
            TheId::named("Tile Node Add Min"),
        ));
        compose_menu.add(TheContextMenuItem::new(
            "Max".to_string(),
            TheId::named("Tile Node Add Max"),
        ));
        compose_menu.add(TheContextMenuItem::new(
            "Add".to_string(),
            TheId::named("Tile Node Add Add"),
        ));
        compose_menu.add(TheContextMenuItem::new(
            "Subtract".to_string(),
            TheId::named("Tile Node Add Subtract"),
        ));
        compose_menu.add(TheContextMenuItem::new(
            "Multiply".to_string(),
            TheId::named("Tile Node Add Multiply"),
        ));
        compose_menu.add(TheContextMenuItem::new(
            "Mask Blend".to_string(),
            TheId::named("Tile Node Add Mask Blend"),
        ));
        compose_menu.add(TheContextMenuItem::new(
            "Material Mix".to_string(),
            TheId::named("Tile Node Add Material Mix"),
        ));

        let mut utility_menu = TheContextMenu::default();
        utility_menu.id = TheId::named("Tile Node Add Utility");
        utility_menu.add(TheContextMenuItem::new(
            "Levels".to_string(),
            TheId::named("Tile Node Add Levels"),
        ));
        utility_menu.add(TheContextMenuItem::new(
            "Curve".to_string(),
            TheId::named("Tile Node Add Curve"),
        ));
        utility_menu.add(TheContextMenuItem::new(
            "Threshold".to_string(),
            TheId::named("Tile Node Add Threshold"),
        ));
        utility_menu.add(TheContextMenuItem::new(
            "Blur".to_string(),
            TheId::named("Tile Node Add Blur"),
        ));
        utility_menu.add(TheContextMenuItem::new(
            "Slope Blur".to_string(),
            TheId::named("Tile Node Add Slope Blur"),
        ));
        utility_menu.add(TheContextMenuItem::new(
            "Height Edge".to_string(),
            TheId::named("Tile Node Add Height Edge"),
        ));
        utility_menu.add(TheContextMenuItem::new(
            "Invert".to_string(),
            TheId::named("Tile Node Add Invert"),
        ));

        let add_items = vec![
            TheContextMenuItem::new_submenu(
                "Generators".to_string(),
                TheId::named("Tile Node Add Generators"),
                generator_menu,
            ),
            TheContextMenuItem::new_submenu(
                "Layouts".to_string(),
                TheId::named("Tile Node Add Layouts"),
                layout_menu,
            ),
            TheContextMenuItem::new_submenu(
                "Palette".to_string(),
                TheId::named("Tile Node Add Palette"),
                palette_menu,
            ),
            TheContextMenuItem::new_submenu(
                "Transform".to_string(),
                TheId::named("Tile Node Add Transform"),
                transform_menu,
            ),
            TheContextMenuItem::new_submenu(
                "Compose".to_string(),
                TheId::named("Tile Node Add Compose"),
                compose_menu,
            ),
            TheContextMenuItem::new_submenu(
                "Utility".to_string(),
                TheId::named("Tile Node Add Utility"),
                utility_menu,
            ),
        ];
        add_button.set_context_menu(Some(TheContextMenu {
            items: add_items,
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(add_button));

        let mut subgraph_button = TheTraybarButton::new(TheId::named("Tile Node Subgraphs"));
        subgraph_button.set_text("Subgraphs".to_string());
        subgraph_button
            .set_status_text("Add a saved reusable subgraph asset as a node in the current graph.");
        toolbar_hlayout.add_widget(Box::new(subgraph_button));

        let mut graph_button = TheTraybarButton::new(TheId::named("Tile Node Graph"));
        graph_button.set_text("Graph".to_string());
        graph_button.set_status_text(
            "Import, export, or save the current node graph as a reusable subgraph.",
        );
        let mut graph_menu = TheContextMenu::default();
        graph_menu.add(TheContextMenuItem::new(
            "Import Graph...".to_string(),
            TheId::named("Tile Node Import Graph"),
        ));
        graph_menu.add(TheContextMenuItem::new(
            "Export Graph...".to_string(),
            TheId::named("Tile Node Export Graph"),
        ));
        graph_menu.add_separator();
        graph_menu.add(TheContextMenuItem::new(
            "Save Subgraph".to_string(),
            TheId::named("Tile Node Save Subgraph"),
        ));
        graph_button.set_context_menu(Some(graph_menu));
        toolbar_hlayout.add_widget(Box::new(graph_button));

        let mut preview_mode = TheDropdownMenu::new(TheId::named("Tile Node Preview Mode"));
        preview_mode.add_option("Color".to_string());
        preview_mode.add_option("Material".to_string());
        preview_mode.set_selected_index(0);
        preview_mode.set_status_text(
            "Choose what node previews display: the graph's color output or the packed material channels.",
        );
        toolbar_hlayout.add_widget(Box::new(preview_mode));

        let mut debug_mode = TheDropdownMenu::new(TheId::named("Tile Node Debug Mode"));
        debug_mode.add_option("Normal".to_string());
        debug_mode.add_option("Bypass".to_string());
        debug_mode.add_option("Mute".to_string());
        debug_mode.add_option("Solo".to_string());
        debug_mode.set_selected_index(0);
        debug_mode.set_status_text(
            "Choose how the selected node behaves for debugging: Normal evaluates it normally, Bypass routes through input 0, Mute suppresses it, Solo previews it at graph output.",
        );
        toolbar_hlayout.add_widget(Box::new(debug_mode));
        node_toolbar.set_layout(toolbar_hlayout);
        node_center.set_top(node_toolbar);

        let mut material_node_canvas = TheCanvas::new();
        let node_view = TheNodeCanvasView::new(TheId::named(TILE_NODE_CANVAS_VIEW));
        material_node_canvas.set_widget(node_view);
        node_center.set_center(material_node_canvas);

        let mut node_settings_canvas = TheCanvas::default();
        let mut text_layout = TheTextLayout::new(TheId::named(TILE_NODE_SETTINGS_LAYOUT));
        text_layout.limiter_mut().set_max_width(280);
        text_layout.set_text_margin(20);
        text_layout.set_text_align(TheHorizontalAlign::Right);
        node_settings_canvas.set_layout(text_layout);

        node_canvas.set_center(node_center);
        node_canvas.set_right(node_settings_canvas);

        let mut root = TheCanvas::new();
        let mut mode_stack = TheStackLayout::new(TheId::named("Tiles Editor Mode Stack"));
        mode_stack.add_canvas(pixel_canvas);
        mode_stack.add_canvas(node_canvas);
        root.set_layout(mode_stack);
        root
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        self.mode = if server_ctx.tile_node_group_id.is_some() {
            TilesEditorMode::NodeSkeleton
        } else {
            TilesEditorMode::Pixel
        };
        self.current_node_group_id = server_ctx.tile_node_group_id;
        if let Some(stack) = ui.get_stack_layout("Tiles Editor Mode Stack") {
            stack.set_index(if self.mode == TilesEditorMode::NodeSkeleton {
                1
            } else {
                0
            });
        }
        if self.mode == TilesEditorMode::Pixel {
            self.clear_selected_node_ui(ui, ctx);
            self.editing_context_changed(ui, ctx, project, server_ctx);
        } else {
            self.refresh_node_group_ui(project, ui, ctx);
            self.set_selected_node_ui(project, ui, ctx);
        }
    }

    fn minimized(&mut self, _ui: &mut TheUI, ctx: &mut TheContext) {
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tiles"),
            TheValue::Empty,
        ));
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if server_ctx.help_mode {
            let open_tile_help = match event {
                TheEvent::TileEditorClicked(id, _) => {
                    id.name == "Tile Editor Dock RGBA Layout View"
                }
                TheEvent::StateChanged(id, state) if *state == TheWidgetState::Clicked => {
                    id.name.starts_with("Tile ")
                        || id.name == "Grid Enabled CB"
                        || id.name == "Tile Editor Tree"
                }
                TheEvent::MouseDown(coord) => ui
                    .get_widget_at_coord(*coord)
                    .map(|w| {
                        let name = &w.id().name;
                        name.starts_with("Tile ")
                            || name == "Tile Editor Dock RGBA Layout View"
                            || name == "Tile Editor Tree"
                            || name == "Grid Enabled CB"
                    })
                    .unwrap_or(false),
                _ => false,
            };

            if open_tile_help {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Show Help"),
                    TheValue::Text("docs/creator/docks/tile_picker_editor".into()),
                ));
                return true;
            }
        }

        let mut redraw = false;

        match event {
            TheEvent::ContextMenuSelected(id, item_id) => {
                if self.mode == TilesEditorMode::NodeSkeleton
                    && id.name.starts_with("Tile Node Add")
                    && let Some(group_id) = self.current_node_group_id
                {
                    let before = project.clone();
                    let mut state = self.node_graph_state_for_group(project, group_id);
                    let palette_index = project.palette.current_index;
                    let new_pos = (
                        state.offset.0 + 260,
                        state.offset.1 + 60 + (state.nodes.len() as i32 - 1) * 32,
                    );
                    let mut push_node = |kind: TileNodeKind| {
                        state.nodes.push(TileNodeState {
                            kind,
                            position: new_pos,
                            bypass: false,
                            mute: false,
                            solo: false,
                        });
                        state.selected_node = Some(state.nodes.len() - 1);
                        self.store_node_graph_state(project, group_id, &state);
                        self.render_node_group_tiles(project, group_id);
                        self.refresh_node_group_ui(project, ui, ctx);
                        self.set_selected_node_ui(project, ui, ctx);
                        self.add_node_graph_undo(before.clone(), project.clone(), ctx);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Tiles"),
                            TheValue::Empty,
                        ));
                    };
                    if item_id.name == "Tile Node Add Group UV" {
                        push_node(TileNodeKind::GroupUV);
                        return true;
                    } else if item_id.name == "Tile Node Add Scalar" {
                        push_node(TileNodeKind::Scalar { value: 0.5 });
                        return true;
                    } else if item_id.name == "Tile Node Add Gradient" {
                        push_node(TileNodeKind::Gradient { mode: 1 });
                        return true;
                    } else if item_id.name == "Tile Node Add Curve" {
                        push_node(TileNodeKind::Curve { power: 1.5 });
                        return true;
                    } else if item_id.name == "Tile Node Add Voronoi" {
                        push_node(TileNodeKind::Voronoi {
                            scale: 0.2,
                            seed: 1,
                            jitter: 1.0,
                            warp_amount: 0.0,
                            falloff: 1.0,
                        });
                        return true;
                    } else if item_id.name == "Tile Node Add Id Random" {
                        push_node(TileNodeKind::IdRandom);
                        return true;
                    } else if item_id.name == "Tile Node Add Box Divide" {
                        push_node(TileNodeKind::BoxDivide {
                            scale: 1.0,
                            gap: 1.0,
                            rotation: 0.15,
                            rounding: 0.04,
                            warp_amount: 0.0,
                            falloff: 1.0,
                        });
                        return true;
                    } else if item_id.name == "Tile Node Add Repeat" {
                        push_node(TileNodeKind::Repeat {
                            repeat_x: 2.0,
                            repeat_y: 2.0,
                        });
                        return true;
                    } else if item_id.name == "Tile Node Add Offset" {
                        push_node(TileNodeKind::Offset { x: 0.1, y: 0.1 });
                        return true;
                    } else if item_id.name == "Tile Node Add Scale" {
                        push_node(TileNodeKind::Scale { x: 2.0, y: 2.0 });
                        return true;
                    } else if item_id.name == "Tile Node Add Rotate" {
                        push_node(TileNodeKind::Rotate { angle: 45.0 });
                        return true;
                    } else if item_id.name == "Tile Node Add Directional Warp" {
                        push_node(TileNodeKind::DirectionalWarp {
                            amount: 0.08,
                            angle: 45.0,
                        });
                        return true;
                    } else if item_id.name == "Tile Node Add Brick" {
                        push_node(TileNodeKind::Brick {
                            columns: 6,
                            rows: 6,
                            offset: 0.5,
                            warp_amount: 0.0,
                            falloff: 1.0,
                        });
                        return true;
                    } else if item_id.name == "Tile Node Add Disc" {
                        push_node(TileNodeKind::Disc {
                            scale: 0.2,
                            seed: 1,
                            jitter: 1.0,
                            radius: 0.75,
                            warp_amount: 0.0,
                            falloff: 1.0,
                        });
                        return true;
                    } else if item_id.name == "Tile Node Add Levels" {
                        push_node(TileNodeKind::Levels {
                            level: 0.5,
                            width: 0.5,
                        });
                        return true;
                    } else if item_id.name == "Tile Node Add Threshold" {
                        push_node(TileNodeKind::Threshold { cutoff: 0.5 });
                        return true;
                    } else if item_id.name == "Tile Node Add Blur" {
                        push_node(TileNodeKind::Blur { radius: 0.012 });
                        return true;
                    } else if item_id.name == "Tile Node Add Slope Blur" {
                        push_node(TileNodeKind::SlopeBlur {
                            radius: 0.016,
                            amount: 0.55,
                        });
                        return true;
                    } else if item_id.name == "Tile Node Add Height Edge" {
                        push_node(TileNodeKind::HeightEdge { radius: 0.01 });
                        return true;
                    } else if item_id.name == "Tile Node Add Warp" {
                        push_node(TileNodeKind::Warp { amount: 0.1 });
                        return true;
                    } else if item_id.name == "Tile Node Add Invert" {
                        push_node(TileNodeKind::Invert);
                        return true;
                    }
                    if item_id.name == "Tile Node Add Color" {
                        push_node(TileNodeKind::Color {
                            color: TheColor::from_u8_array_3([255, 255, 255]),
                        });
                        return true;
                    } else if item_id.name == "Tile Node Add Make Material" {
                        push_node(TileNodeKind::MakeMaterial);
                        return true;
                    } else if item_id.name == "Tile Node Add Mix" {
                        push_node(TileNodeKind::Mix { factor: 0.5 });
                        return true;
                    } else if item_id.name == "Tile Node Add Min" {
                        push_node(TileNodeKind::Min);
                        return true;
                    } else if item_id.name == "Tile Node Add Max" {
                        push_node(TileNodeKind::Max);
                        return true;
                    } else if item_id.name == "Tile Node Add Add" {
                        push_node(TileNodeKind::Add);
                        return true;
                    } else if item_id.name == "Tile Node Add Subtract" {
                        push_node(TileNodeKind::Subtract);
                        return true;
                    } else if item_id.name == "Tile Node Add Checker" {
                        push_node(TileNodeKind::Checker { scale: 8 });
                        return true;
                    } else if item_id.name == "Tile Node Add Noise" {
                        push_node(TileNodeKind::Noise {
                            scale: 0.25,
                            seed: 1,
                            wrap: false,
                        });
                        return true;
                    } else if item_id.name == "Tile Node Add Multiply" {
                        push_node(TileNodeKind::Multiply);
                        return true;
                    } else if item_id.name == "Tile Node Add Mask Blend" {
                        push_node(TileNodeKind::MaskBlend { factor: 1.0 });
                        return true;
                    } else if item_id.name == "Tile Node Add Material Mix" {
                        push_node(TileNodeKind::MaterialMix { factor: 1.0 });
                        return true;
                    } else if item_id.name == "Tile Node Add Palette Color" {
                        push_node(TileNodeKind::PaletteColor {
                            index: palette_index,
                        });
                        return true;
                    } else if item_id.name == "Tile Node Add Nearest Palette" {
                        push_node(TileNodeKind::NearestPalette);
                        return true;
                    }
                    return false;
                } else if self.mode == TilesEditorMode::NodeSkeleton
                    && id.name == "Tile Node Subgraphs"
                    && let Some(group_id) = self.current_node_group_id
                    && item_id.name == "Tile Node Add Subgraph"
                {
                    let before = project.clone();
                    let mut state = self.node_graph_state_for_group(project, group_id);
                    let new_pos = (
                        state.offset.0 + 260,
                        state.offset.1 + 60 + (state.nodes.len() as i32 - 1) * 32,
                    );
                    state.nodes.push(TileNodeState {
                        kind: TileNodeKind::Subgraph {
                            subgraph_id: item_id.uuid,
                        },
                        position: new_pos,
                        bypass: false,
                        mute: false,
                        solo: false,
                    });
                    state.selected_node = Some(state.nodes.len() - 1);
                    self.store_node_graph_state(project, group_id, &state);
                    self.render_node_group_tiles(project, group_id);
                    self.refresh_node_group_ui(project, ui, ctx);
                    self.set_selected_node_ui(project, ui, ctx);
                    self.add_node_graph_undo(before, project.clone(), ctx);
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Tiles"),
                        TheValue::Empty,
                    ));
                    return true;
                } else if self.mode == TilesEditorMode::NodeSkeleton && id.name == "Tile Node Graph"
                {
                    if item_id.name == "Tile Node Save Subgraph"
                        && let Some(group_id) = self.current_node_group_id
                        && let Some(node_group) = project.tile_node_groups.get(&group_id)
                    {
                        let mut name = node_group.graph_name.trim().to_string();
                        if name.is_empty() {
                            name = "Subgraph".to_string();
                        }
                        let state = self.node_graph_state_for_group(project, group_id);
                        let mut subgraph = TileSubgraphAsset::new(format!("{name} Subgraph"));
                        subgraph.graph_data = serde_json::to_string(&state).unwrap_or_default();
                        project.add_tile_subgraph(subgraph);
                        ctx.ui.send(TheEvent::SetStatusText(
                            id.clone(),
                            "Saved the current node graph as a reusable subgraph asset."
                                .to_string(),
                        ));
                        return true;
                    } else if item_id.name == "Tile Node Import Graph" {
                        ctx.ui.open_file_requester(
                            TheId::named("Tile Node Import Graph File"),
                            "Import Tile Node Graph".into(),
                            TheFileExtension::new(
                                "Eldiron Tile Node Graph".into(),
                                vec![
                                    "tilegraph".to_string(),
                                    "eldiron_graph".to_string(),
                                    "json".to_string(),
                                ],
                            ),
                        );
                        return true;
                    } else if item_id.name == "Tile Node Export Graph"
                        && let Some(group_id) = self.current_node_group_id
                    {
                        ctx.ui.save_file_requester(
                            TheId::named_with_id("Tile Node Export Graph File", group_id),
                            "Export Tile Node Graph".into(),
                            TheFileExtension::new(
                                "Eldiron Tile Node Graph".into(),
                                vec!["tilegraph".to_string()],
                            ),
                        );
                        return true;
                    }
                }
            }
            TheEvent::StateChanged(_id, TheWidgetState::Clicked) => {
                if self.mode == TilesEditorMode::NodeSkeleton {}
            }
            TheEvent::FileRequesterResult(id, paths) => {
                if self.mode == TilesEditorMode::NodeSkeleton
                    && id.name == "Tile Node Import Graph File"
                    && let Some(group_id) = self.current_node_group_id
                    && let Some(path) = paths.first()
                {
                    match self.import_node_graph_file(project, group_id, path) {
                        Ok(()) => {
                            self.render_node_group_tiles(project, group_id);
                            self.refresh_node_group_ui(project, ui, ctx);
                            self.set_selected_node_ui(project, ui, ctx);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Update Tiles"),
                                TheValue::Empty,
                            ));
                            ctx.ui.send(TheEvent::SetStatusText(
                                id.clone(),
                                format!("Imported graph from {}", path.to_string_lossy()),
                            ));
                        }
                        Err(err) => {
                            ctx.ui.send(TheEvent::SetStatusText(
                                id.clone(),
                                format!("Graph import failed: {err}"),
                            ));
                        }
                    }
                    return true;
                } else if self.mode == TilesEditorMode::NodeSkeleton
                    && id.name == "Tile Node Export Graph File"
                    && let Some(group_id) = self.current_node_group_id
                    && let Some(path) = paths.first()
                {
                    match self.export_node_graph_file(project, group_id, path) {
                        Ok(saved_path) => {
                            ctx.ui.send(TheEvent::SetStatusText(
                                id.clone(),
                                format!("Exported graph to {}", saved_path.to_string_lossy()),
                            ));
                        }
                        Err(err) => {
                            ctx.ui.send(TheEvent::SetStatusText(
                                id.clone(),
                                format!("Graph export failed: {err}"),
                            ));
                        }
                    }
                    return true;
                }
            }
            TheEvent::Custom(id, value) => {
                if id.name == "Open Tile Node Editor Skeleton" {
                    self.mode = TilesEditorMode::NodeSkeleton;
                    self.current_node_group_id = if let TheValue::Id(group_id) = value {
                        Some(*group_id)
                    } else {
                        server_ctx.tile_node_group_id
                    };
                    if let Some(stack) = ui.get_stack_layout("Tiles Editor Mode Stack") {
                        stack.set_index(1);
                    }
                    if let Some(group_id) = self.current_node_group_id {
                        self.render_node_group_tiles(project, group_id);
                    }
                    self.refresh_node_group_ui(project, ui, ctx);
                    self.set_selected_node_ui(project, ui, ctx);
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Tiles"),
                        TheValue::Empty,
                    ));
                    return true;
                } else if id.name == "Close Tile Node Editor Skeleton" {
                    self.mode = TilesEditorMode::Pixel;
                    self.current_node_group_id = None;
                    if let Some(stack) = ui.get_stack_layout("Tiles Editor Mode Stack") {
                        stack.set_index(0);
                    }
                    self.clear_selected_node_ui(ui, ctx);
                    return true;
                } else if id.name == "Update Tilepicker" {
                    if self.mode == TilesEditorMode::NodeSkeleton {
                        if let Some(group_id) = self.current_node_group_id {
                            if project.tile_node_groups.contains_key(&group_id) {
                                self.refresh_node_group_ui(project, ui, ctx);
                                self.set_selected_node_ui(project, ui, ctx);
                            } else {
                                self.mode = TilesEditorMode::Pixel;
                                self.current_node_group_id = None;
                                if let Some(stack) = ui.get_stack_layout("Tiles Editor Mode Stack")
                                {
                                    stack.set_index(0);
                                }
                                self.clear_selected_node_ui(ui, ctx);
                            }
                        }
                        return true;
                    }
                }
                if let TheValue::Id(tile_id) = value
                    && id.name == "Tile Picked"
                {
                    if let Some(tile) = project.tiles.get(tile_id) {
                        self.set_tile(tile, ui, ctx, server_ctx, false);
                    }
                    self.editing_context_changed(ui, ctx, project, server_ctx);
                } else if let TheValue::Id(tile_id) = value
                    && id.name == "Tile Updated"
                {
                    if let Some(tile) = project.tiles.get(tile_id) {
                        self.set_tile(tile, ui, ctx, server_ctx, true);

                        // Update the current frame
                        if let Some(tree_layout) = ui.get_tree_layout("Tile Editor Tree") {
                            if let Some(tile_node) = tree_layout.get_node_by_id_mut(&self.tile_node)
                            {
                                // Update the frame icon
                                if let Some(widget) = tile_node.widgets[2].as_tree_icons() {
                                    if server_ctx.curr_tile_frame_index < tile.textures.len() {
                                        widget.set_icon(
                                            server_ctx.curr_tile_frame_index,
                                            tile.textures[server_ctx.curr_tile_frame_index]
                                                .to_rgba(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                } else if id.name == "Editing Texture Updated" {
                    self.refresh_from_editing_context(project, ui, ctx, server_ctx);
                } else if id.name == "Tile Editor Undo Available" {
                    if let Some(atom) = TOOLLIST
                        .write()
                        .unwrap()
                        .get_current_editor_tool()
                        .get_undo_atom(project)
                    {
                        if let Some(atom) = atom.downcast_ref::<TileEditorUndoAtom>() {
                            self.add_undo(atom.clone(), ctx);
                        }
                    }
                }
            }
            TheEvent::NodeSelectedIndexChanged(id, index) => {
                if self.mode == TilesEditorMode::NodeSkeleton
                    && id.name == TILE_NODE_CANVAS_VIEW
                    && let Some(group_id) = self.current_node_group_id
                {
                    let mut state = self.node_graph_state_for_group(project, group_id);
                    state.selected_node = *index;
                    self.store_node_graph_state(project, group_id, &state);
                    self.set_node_group_canvas(project, ui);
                    self.set_selected_node_ui(project, ui, ctx);
                    return true;
                }
            }
            TheEvent::NodeDragged(id, index, position) => {
                if self.mode == TilesEditorMode::NodeSkeleton
                    && id.name == TILE_NODE_CANVAS_VIEW
                    && let Some(group_id) = self.current_node_group_id
                {
                    let mut state = self.node_graph_state_for_group(project, group_id);
                    if let Some(node) = state.nodes.get_mut(*index) {
                        node.position = (position.x, position.y);
                    }
                    state.selected_node = Some(*index);
                    self.store_node_graph_state(project, group_id, &state);
                    return true;
                }
            }
            TheEvent::NodeConnectionAdded(id, connections)
            | TheEvent::NodeConnectionRemoved(id, connections) => {
                if self.mode == TilesEditorMode::NodeSkeleton
                    && id.name == TILE_NODE_CANVAS_VIEW
                    && let Some(group_id) = self.current_node_group_id
                {
                    let before = project.clone();
                    let mut state = self.node_graph_state_for_group(project, group_id);
                    state.connections.clone_from(connections);
                    self.store_node_graph_state(project, group_id, &state);
                    self.render_node_group_tiles(project, group_id);
                    self.refresh_node_group_ui(project, ui, ctx);
                    self.add_node_graph_undo(before, project.clone(), ctx);
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Tiles"),
                        TheValue::Empty,
                    ));
                    return true;
                }
            }
            TheEvent::NodeDeleted(id, deleted_node_index, connections) => {
                if self.mode == TilesEditorMode::NodeSkeleton
                    && id.name == TILE_NODE_CANVAS_VIEW
                    && let Some(group_id) = self.current_node_group_id
                {
                    let before = project.clone();
                    let mut state = self.node_graph_state_for_group(project, group_id);
                    if *deleted_node_index < state.nodes.len() {
                        state.nodes.remove(*deleted_node_index);
                    }
                    state.connections.clone_from(connections);
                    state.selected_node = Some(0);
                    state.ensure_root();
                    self.store_node_graph_state(project, group_id, &state);
                    self.render_node_group_tiles(project, group_id);
                    self.refresh_node_group_ui(project, ui, ctx);
                    self.set_selected_node_ui(project, ui, ctx);
                    self.add_node_graph_undo(before, project.clone(), ctx);
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Tiles"),
                        TheValue::Empty,
                    ));
                    return true;
                }
            }
            TheEvent::NodeViewScrolled(id, offset) => {
                if self.mode == TilesEditorMode::NodeSkeleton
                    && id.name == TILE_NODE_CANVAS_VIEW
                    && let Some(group_id) = self.current_node_group_id
                {
                    let mut state = self.node_graph_state_for_group(project, group_id);
                    state.offset = (offset.x, offset.y);
                    self.store_node_graph_state(project, group_id, &state);
                    return true;
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if self.mode == TilesEditorMode::NodeSkeleton
                    && let Some(group_id) = self.current_node_group_id
                {
                    let before = project.clone();
                    let mut node_group_changed = false;
                    let mut size_changed = false;
                    let mut graph_changed = false;

                    if let Some(node_group) = project.tile_node_groups.get_mut(&group_id) {
                        if id.name == "tileNodeGraphName" {
                            if let Some(text) = value.to_string() {
                                node_group.graph_name = text;
                                node_group_changed = true;
                            }
                        } else if id.name == "tileNodeGroupSize" {
                            if let Some(text) = value.to_string()
                                && let Some((width, height)) = Self::parse_size_pair(&text, 64)
                                && (node_group.output_grid_width != width
                                    || node_group.output_grid_height != height)
                            {
                                node_group.output_grid_width = width;
                                node_group.output_grid_height = height;
                                node_group_changed = true;
                                size_changed = true;
                            }
                        } else if id.name == "tileNodePixelSize" {
                            if let Some(text) = value.to_string()
                                && let Some((width, height)) = Self::parse_size_pair(&text, 2048)
                                && (node_group.tile_pixel_width != width
                                    || node_group.tile_pixel_height != height)
                            {
                                node_group.tile_pixel_width = width;
                                node_group.tile_pixel_height = height;
                                node_group_changed = true;
                            }
                        }
                    }

                    if id.name == "tileNodeColorValue" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Color { color } = &mut node.kind
                            && let Some(new_color) = value.to_color()
                            && *color != new_color
                        {
                            *color = new_color;
                            self.store_node_graph_state(project, group_id, &state);
                            graph_changed = true;
                        }
                    } else if id.name == "Tile Node Preview Mode" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(new_mode) = value.to_i32() {
                            let new_mode = new_mode.clamp(0, 1) as u8;
                            if state.preview_mode != new_mode {
                                state.preview_mode = new_mode;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "Tile Node Debug Mode" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let Some(mode) = value.to_i32()
                        {
                            let mode = mode.clamp(0, 3);
                            let mut changed = false;
                            let want_bypass = mode == 1;
                            let want_mute = mode == 2;
                            let want_solo = mode == 3;

                            if node.bypass != want_bypass {
                                node.bypass = want_bypass;
                                changed = true;
                            }
                            if node.mute != want_mute {
                                node.mute = want_mute;
                                changed = true;
                            }
                            if want_solo {
                                for (i, other) in state.nodes.iter_mut().enumerate() {
                                    let want = i == index;
                                    if other.solo != want {
                                        other.solo = want;
                                        changed = true;
                                    }
                                }
                            } else if node.solo {
                                node.solo = false;
                                changed = true;
                            }

                            if changed {
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeScalarValue" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Scalar { value: scalar } = &mut node.kind
                            && let Some(new_value) = value.to_f32()
                        {
                            let new_value = new_value.clamp(0.0, 1.0);
                            if (*scalar - new_value).abs() > f32::EPSILON {
                                *scalar = new_value;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodePaletteIndex" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::PaletteColor {
                                index: palette_index,
                            } = &mut node.kind
                            && let Some(new_index) = value.to_i32()
                        {
                            let new_index = new_index.clamp(0, 255) as u16;
                            if *palette_index != new_index {
                                *palette_index = new_index;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeBoxDivideScale"
                        || id.name == "tileNodeBoxDivideGap"
                        || id.name == "tileNodeBoxDivideRotation"
                        || id.name == "tileNodeBoxDivideRounding"
                    {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::BoxDivide {
                                scale,
                                gap,
                                rotation,
                                rounding,
                                ..
                            } = &mut node.kind
                            && let Some(new_value) = value.to_f32()
                        {
                            let (target, new_value) = if id.name == "tileNodeBoxDivideScale" {
                                (scale, new_value.clamp(0.1, 3.0))
                            } else if id.name == "tileNodeBoxDivideGap" {
                                (gap, new_value.clamp(0.0, 4.0))
                            } else if id.name == "tileNodeBoxDivideRotation" {
                                (rotation, new_value.clamp(0.0, 2.0))
                            } else {
                                (rounding, new_value.clamp(0.0, 0.5))
                            };
                            if (*target - new_value).abs() > f32::EPSILON {
                                *target = new_value;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeMixFactor" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Mix { factor } = &mut node.kind
                            && let Some(new_factor) = value.to_f32()
                        {
                            let new_factor = new_factor.clamp(0.0, 1.0);
                            if (*factor - new_factor).abs() > f32::EPSILON {
                                *factor = new_factor;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeCheckerScale" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Checker { scale, .. } = &mut node.kind
                            && let Some(new_scale) = value.to_i32()
                        {
                            let new_scale = new_scale.clamp(1, 64) as u16;
                            if *scale != new_scale {
                                *scale = new_scale;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeNoiseScale" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Noise { scale, .. } = &mut node.kind
                            && let Some(new_scale) = value.to_f32()
                        {
                            let new_scale = new_scale.clamp(0.0, 1.0);
                            if (*scale - new_scale).abs() > f32::EPSILON {
                                *scale = new_scale;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeNoiseSeed" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Noise { seed, .. } = &mut node.kind
                            && let Some(new_seed) = value.to_i32()
                        {
                            let new_seed = new_seed.clamp(0, 9999) as u32;
                            if *seed != new_seed {
                                *seed = new_seed;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeNoiseMode" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Noise { wrap, .. } = &mut node.kind
                            && let Some(mode) = value.to_i32()
                        {
                            let new_wrap = mode == 1;
                            if *wrap != new_wrap {
                                *wrap = new_wrap;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeMaskBlendFactor" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::MaskBlend { factor } = &mut node.kind
                            && let Some(new_factor) = value.to_f32()
                        {
                            let new_factor = new_factor.clamp(0.0, 1.0);
                            if (*factor - new_factor).abs() > f32::EPSILON {
                                *factor = new_factor;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeMaterialMixFactor" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::MaterialMix { factor } = &mut node.kind
                            && let Some(new_factor) = value.to_f32()
                        {
                            let new_factor = new_factor.clamp(0.0, 1.0);
                            if (*factor - new_factor).abs() > f32::EPSILON {
                                *factor = new_factor;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeOutputRoughness"
                        || id.name == "tileNodeOutputMetallic"
                        || id.name == "tileNodeOutputOpacity"
                        || id.name == "tileNodeOutputEmissive"
                    {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(node) = state.nodes.get_mut(0)
                            && let TileNodeKind::OutputRoot {
                                roughness,
                                metallic,
                                opacity,
                                emissive,
                            } = &mut node.kind
                            && let Some(new_value) = value.to_f32()
                        {
                            let new_value = new_value.clamp(0.0, 1.0);
                            let target = if id.name == "tileNodeOutputRoughness" {
                                roughness
                            } else if id.name == "tileNodeOutputMetallic" {
                                metallic
                            } else if id.name == "tileNodeOutputOpacity" {
                                opacity
                            } else {
                                emissive
                            };
                            if (*target - new_value).abs() > f32::EPSILON {
                                *target = new_value;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeGradientMode" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Gradient { mode } = &mut node.kind
                            && let Some(new_mode) = value.to_i32()
                        {
                            let new_mode = new_mode.clamp(0, 2) as u8;
                            if *mode != new_mode {
                                *mode = new_mode;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeCurvePower" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Curve { power } = &mut node.kind
                            && let Some(new_power) = value.to_f32()
                        {
                            let new_power = new_power.clamp(0.1, 4.0);
                            if (*power - new_power).abs() > f32::EPSILON {
                                *power = new_power;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeRepeatX" || id.name == "tileNodeRepeatY" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Repeat { repeat_x, repeat_y } = &mut node.kind
                            && let Some(new_value) = value.to_f32()
                        {
                            let new_value = new_value.clamp(0.1, 64.0);
                            let target = if id.name == "tileNodeRepeatX" {
                                repeat_x
                            } else {
                                repeat_y
                            };
                            if (*target - new_value).abs() > f32::EPSILON {
                                *target = new_value;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeOffsetX" || id.name == "tileNodeOffsetY" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Offset { x, y } = &mut node.kind
                            && let Some(new_value) = value.to_f32()
                        {
                            let new_value = new_value.clamp(-1.0, 1.0);
                            let target = if id.name == "tileNodeOffsetX" { x } else { y };
                            if (*target - new_value).abs() > f32::EPSILON {
                                *target = new_value;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeScaleX" || id.name == "tileNodeScaleY" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Scale { x, y } = &mut node.kind
                            && let Some(new_value) = value.to_f32()
                        {
                            let new_value = new_value.clamp(0.1, 16.0);
                            let target = if id.name == "tileNodeScaleX" { x } else { y };
                            if (*target - new_value).abs() > f32::EPSILON {
                                *target = new_value;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeRotateAngle" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Rotate { angle } = &mut node.kind
                            && let Some(new_angle) = value.to_f32()
                        {
                            let new_angle = new_angle.clamp(-180.0, 180.0);
                            if (*angle - new_angle).abs() > f32::EPSILON {
                                *angle = new_angle;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeDirectionalWarpAmount"
                        || id.name == "tileNodeDirectionalWarpAngle"
                    {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::DirectionalWarp { amount, angle } = &mut node.kind
                            && let Some(new_value) = value.to_f32()
                        {
                            let target = if id.name == "tileNodeDirectionalWarpAmount" {
                                amount
                            } else {
                                angle
                            };
                            let new_value = if id.name == "tileNodeDirectionalWarpAmount" {
                                new_value.clamp(0.0, 1.0)
                            } else {
                                new_value.clamp(-180.0, 180.0)
                            };
                            if (*target - new_value).abs() > f32::EPSILON {
                                *target = new_value;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeBrickColumns"
                        || id.name == "tileNodeBrickRows"
                        || id.name == "tileNodeBrickOffset"
                        || id.name == "tileNodeBrickWarpAmount"
                        || id.name == "tileNodeBrickFalloff"
                    {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Brick {
                                columns,
                                rows,
                                offset,
                                warp_amount,
                                falloff,
                            } = &mut node.kind
                        {
                            let mut changed = false;
                            if id.name == "tileNodeBrickOffset" {
                                if let Some(new_offset) = value.to_f32() {
                                    let new_offset = new_offset.clamp(0.0, 1.0);
                                    if (*offset - new_offset).abs() > f32::EPSILON {
                                        *offset = new_offset;
                                        changed = true;
                                    }
                                }
                            } else if id.name == "tileNodeBrickWarpAmount" {
                                if let Some(new_warp_amount) = value.to_f32() {
                                    let new_warp_amount = new_warp_amount.clamp(0.0, 0.25);
                                    if (*warp_amount - new_warp_amount).abs() > f32::EPSILON {
                                        *warp_amount = new_warp_amount;
                                        changed = true;
                                    }
                                }
                            } else if id.name == "tileNodeBrickFalloff" {
                                if let Some(new_falloff) = value.to_f32() {
                                    let new_falloff = new_falloff.clamp(0.1, 4.0);
                                    if (*falloff - new_falloff).abs() > f32::EPSILON {
                                        *falloff = new_falloff;
                                        changed = true;
                                    }
                                }
                            } else if let Some(new_value) = value.to_i32() {
                                let new_value = new_value.clamp(1, 64) as u16;
                                if id.name == "tileNodeBrickColumns" && *columns != new_value {
                                    *columns = new_value;
                                    changed = true;
                                } else if id.name == "tileNodeBrickRows" && *rows != new_value {
                                    *rows = new_value;
                                    changed = true;
                                }
                            }
                            if changed {
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeDiscScale"
                        || id.name == "tileNodeDiscSeed"
                        || id.name == "tileNodeDiscJitter"
                        || id.name == "tileNodeDiscRadius"
                        || id.name == "tileNodeDiscWarpAmount"
                        || id.name == "tileNodeDiscFalloff"
                    {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Disc {
                                scale,
                                seed,
                                jitter,
                                radius,
                                warp_amount,
                                falloff,
                            } = &mut node.kind
                        {
                            let mut changed = false;
                            if id.name == "tileNodeDiscSeed" {
                                if let Some(new_seed) = value.to_i32() {
                                    let new_seed = new_seed.clamp(0, 9999) as u32;
                                    if *seed != new_seed {
                                        *seed = new_seed;
                                        changed = true;
                                    }
                                }
                            } else if let Some(new_value) = value.to_f32() {
                                let (target, new_value) = if id.name == "tileNodeDiscScale" {
                                    (scale, new_value.clamp(0.01, 1.0))
                                } else if id.name == "tileNodeDiscJitter" {
                                    (jitter, new_value.clamp(0.0, 1.0))
                                } else if id.name == "tileNodeDiscRadius" {
                                    (radius, new_value.clamp(0.05, 1.0))
                                } else if id.name == "tileNodeDiscWarpAmount" {
                                    (warp_amount, new_value.clamp(0.0, 0.25))
                                } else {
                                    (falloff, new_value.clamp(0.1, 4.0))
                                };
                                if (*target - new_value).abs() > f32::EPSILON {
                                    *target = new_value;
                                    changed = true;
                                }
                            }
                            if changed {
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeVoronoiScale" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Voronoi { scale, .. } = &mut node.kind
                            && let Some(new_scale) = value.to_f32()
                        {
                            let new_scale = new_scale.clamp(0.01, 1.0);
                            if (*scale - new_scale).abs() > f32::EPSILON {
                                *scale = new_scale;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeVoronoiSeed" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Voronoi { seed, .. } = &mut node.kind
                            && let Some(new_seed) = value.to_i32()
                        {
                            let new_seed = new_seed.clamp(0, 9999) as u32;
                            if *seed != new_seed {
                                *seed = new_seed;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeVoronoiJitter" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Voronoi { jitter, .. } = &mut node.kind
                            && let Some(new_jitter) = value.to_f32()
                        {
                            let new_jitter = new_jitter.clamp(0.0, 1.0);
                            if (*jitter - new_jitter).abs() > f32::EPSILON {
                                *jitter = new_jitter;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeVoronoiWarpAmount" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Voronoi { warp_amount, .. } = &mut node.kind
                            && let Some(new_warp_amount) = value.to_f32()
                        {
                            let new_warp_amount = new_warp_amount.clamp(0.0, 0.25);
                            if (*warp_amount - new_warp_amount).abs() > f32::EPSILON {
                                *warp_amount = new_warp_amount;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeVoronoiFalloff" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Voronoi { falloff, .. } = &mut node.kind
                            && let Some(new_falloff) = value.to_f32()
                        {
                            let new_falloff = new_falloff.clamp(0.1, 4.0);
                            if (*falloff - new_falloff).abs() > f32::EPSILON {
                                *falloff = new_falloff;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeBoxDivideScale"
                        || id.name == "tileNodeBoxDivideGap"
                        || id.name == "tileNodeBoxDivideRotation"
                        || id.name == "tileNodeBoxDivideRounding"
                        || id.name == "tileNodeBoxDivideWarpAmount"
                        || id.name == "tileNodeBoxDivideFalloff"
                    {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::BoxDivide {
                                scale,
                                gap,
                                rotation,
                                rounding,
                                warp_amount,
                                falloff,
                            } = &mut node.kind
                            && let Some(new_value) = value.to_f32()
                        {
                            let (target, new_value) = if id.name == "tileNodeBoxDivideScale" {
                                (scale, new_value.clamp(0.1, 3.0))
                            } else if id.name == "tileNodeBoxDivideGap" {
                                (gap, new_value.clamp(0.0, 4.0))
                            } else if id.name == "tileNodeBoxDivideRotation" {
                                (rotation, new_value.clamp(0.0, 2.0))
                            } else if id.name == "tileNodeBoxDivideRounding" {
                                (rounding, new_value.clamp(0.0, 0.5))
                            } else if id.name == "tileNodeBoxDivideWarpAmount" {
                                (warp_amount, new_value.clamp(0.0, 0.25))
                            } else {
                                (falloff, new_value.clamp(0.1, 4.0))
                            };
                            if (*target - new_value).abs() > f32::EPSILON {
                                *target = new_value;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeLevelsLevel" || id.name == "tileNodeLevelsWidth" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Levels { level, width } = &mut node.kind
                            && let Some(new_value) = value.to_f32()
                        {
                            let new_value = new_value.clamp(0.0, 1.0);
                            let is_level = id.name == "tileNodeLevelsLevel";
                            let changed = if is_level {
                                let changed = (*level - new_value).abs() > f32::EPSILON;
                                *level = new_value;
                                changed
                            } else {
                                let changed = (*width - new_value).abs() > f32::EPSILON;
                                *width = new_value.max(0.001);
                                changed
                            };
                            if changed {
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeThresholdCutoff" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Threshold { cutoff } = &mut node.kind
                            && let Some(new_cutoff) = value.to_f32()
                        {
                            let new_cutoff = new_cutoff.clamp(0.0, 1.0);
                            if (*cutoff - new_cutoff).abs() > f32::EPSILON {
                                *cutoff = new_cutoff;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeBlurRadius" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Blur { radius } = &mut node.kind
                            && let Some(new_radius) = value.to_f32()
                        {
                            let new_radius = new_radius.clamp(0.001, 0.08);
                            if (*radius - new_radius).abs() > f32::EPSILON {
                                *radius = new_radius;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeSlopeBlurRadius"
                        || id.name == "tileNodeSlopeBlurAmount"
                    {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::SlopeBlur { radius, amount } = &mut node.kind
                            && let Some(new_value) = value.to_f32()
                        {
                            let is_radius = id.name == "tileNodeSlopeBlurRadius";
                            let changed = if is_radius {
                                let new_radius = new_value.clamp(0.001, 0.08);
                                let changed = (*radius - new_radius).abs() > f32::EPSILON;
                                *radius = new_radius;
                                changed
                            } else {
                                let new_amount = new_value.clamp(0.0, 1.0);
                                let changed = (*amount - new_amount).abs() > f32::EPSILON;
                                *amount = new_amount;
                                changed
                            };
                            if changed {
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeHeightEdgeRadius" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::HeightEdge { radius } = &mut node.kind
                            && let Some(new_radius) = value.to_f32()
                        {
                            let new_radius = new_radius.clamp(0.001, 0.08);
                            if (*radius - new_radius).abs() > f32::EPSILON {
                                *radius = new_radius;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    } else if id.name == "tileNodeWarpAmount" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Warp { amount } = &mut node.kind
                            && let Some(new_amount) = value.to_f32()
                        {
                            let new_amount = new_amount.clamp(0.0, 1.0);
                            if (*amount - new_amount).abs() > f32::EPSILON {
                                *amount = new_amount;
                                self.store_node_graph_state(project, group_id, &state);
                                graph_changed = true;
                            }
                        }
                    }

                    if size_changed
                        && let Some(node_group) = project.tile_node_groups.get(&group_id)
                        && let Some(group) = project.tile_groups.get_mut(&group_id)
                    {
                        group.width = node_group.output_grid_width;
                        group.height = node_group.output_grid_height;
                    }

                    if node_group_changed {
                        self.render_node_group_tiles(project, group_id);
                        self.refresh_node_group_ui(project, ui, ctx);
                        self.add_node_graph_undo(before, project.clone(), ctx);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Tiles"),
                            TheValue::Empty,
                        ));
                        return true;
                    }
                    if graph_changed {
                        self.render_node_group_tiles(project, group_id);
                        self.refresh_node_group_ui(project, ui, ctx);
                        self.add_node_graph_undo(before, project.clone(), ctx);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Tiles"),
                            TheValue::Empty,
                        ));
                        return true;
                    }
                }

                // The Size of the Tile has been edited
                if id.name == "Tile Size Edit" {
                    if let Some(size) = value.to_i32() {
                        if let Some(tile_id) = self.current_tile_id {
                            if let Some(tile) = project.tiles.get_mut(&tile_id) {
                                if !tile.is_empty() {
                                    if size != tile.textures[0].width as i32 {
                                        let new_tile = tile.resized(size as usize, size as usize);
                                        let atom = TileEditorUndoAtom::TileEdit(
                                            tile.id,
                                            tile.clone(),
                                            new_tile.clone(),
                                        );
                                        *tile = new_tile;
                                        self.add_undo(atom, ctx);
                                        self.set_tile(tile, ui, ctx, server_ctx, false);
                                    }
                                }
                            }
                        }
                    }
                } else
                // The frame count of the Tile has been edited
                if id.name == "Tile Frame Edit" {
                    if let Some(frames) = value.to_i32() {
                        if let Some(tile_id) = self.current_tile_id {
                            if let Some(tile) = project.tiles.get_mut(&tile_id) {
                                if frames != tile.textures.len() as i32 {
                                    let mut new_tile = tile.clone();
                                    new_tile.set_frames(frames as usize);
                                    let atom = TileEditorUndoAtom::TileEdit(
                                        tile.id,
                                        tile.clone(),
                                        new_tile.clone(),
                                    );
                                    *tile = new_tile;
                                    self.add_undo(atom, ctx);
                                    self.set_tile(tile, ui, ctx, server_ctx, false);
                                }
                            }
                        }
                    }
                } else
                // The palette opacity has been edited
                if id.name == "Palette Opacity Edit" {
                    if let Some(opacity) = value.to_f32() {
                        server_ctx.palette_opacity = opacity;
                    }
                }
            }
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Tile Frame Icons" {
                    // New frame index selected - update the editor display
                    self.set_frame_index(*index as usize, project, ui, ctx, server_ctx);
                }
                // else if id.name == "Palette Item" {
                //     project.palette.current_index = *index as u16;
                // }
            }
            TheEvent::StateChanged(id, state) => {
                if id.name == "Grid Enabled CB" {
                    self.show_grid = *state == TheWidgetState::Selected;
                    if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout")
                        && let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view()
                    {
                        rgba_view.set_dont_show_grid(!self.show_grid);
                        editor.relayout(ctx);
                    }
                    redraw = true;
                } else if *state == TheWidgetState::Selected && id.name.starts_with("Body: ") {
                    server_ctx.avatar_anchor_slot = AvatarAnchorEditSlot::None;
                    let color = match id.name.as_str() {
                        "Body: Skin Light" => Some([255, 0, 255, 255]),
                        "Body: Skin Dark" => Some([200, 0, 200, 255]),
                        "Body: Torso" => Some([0, 0, 255, 255]),
                        "Body: Legs" => Some([0, 255, 0, 255]),
                        "Body: Hair" => Some([255, 255, 0, 255]),
                        "Body: Eyes" => Some([0, 255, 255, 255]),
                        "Body: Hands" => Some([255, 128, 0, 255]),
                        "Body: Feet" => Some([255, 80, 0, 255]),
                        _ => None,
                    };
                    if let Some(c) = color {
                        server_ctx.body_marker_color = Some(c);
                        redraw = true;
                    }
                } else if *state == TheWidgetState::Selected && id.name == "Anchor: Main" {
                    server_ctx.avatar_anchor_slot = AvatarAnchorEditSlot::Main;
                    self.sync_anchor_overlay(project, ui, ctx, server_ctx);
                    redraw = true;
                } else if *state == TheWidgetState::Selected && id.name == "Anchor: Off" {
                    server_ctx.avatar_anchor_slot = AvatarAnchorEditSlot::Off;
                    self.sync_anchor_overlay(project, ui, ctx, server_ctx);
                    redraw = true;
                }
            }
            TheEvent::TileZoomBy(id, delta) => {
                if id.name == "Tile Editor Dock RGBA Layout View" {
                    self.zoom += *delta * 0.5;
                    self.zoom = self.zoom.clamp(1.0, 60.0);
                    if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
                        editor.set_zoom(self.zoom);
                        editor.relayout(ctx);
                    }
                }
            }
            TheEvent::TileEditorHoverChanged(id, pos) => {
                if id.name == "Tile Editor Dock RGBA Layout View"
                    && self.paste_preview_texture.is_some()
                {
                    self.paste_preview_pos = Some(*pos);
                    self.sync_paste_preview(ui, ctx);
                    redraw = true;
                }
            }
            TheEvent::TileEditorClicked(id, coord) => {
                if id.name == "Tile Editor Dock RGBA Layout View"
                    && self.paste_preview_texture.is_some()
                {
                    self.paste_preview_pos = Some(*coord);
                    if self.apply_paste_at_preview(project, ui, ctx, server_ctx) {
                        self.clear_paste_preview(ui, ctx);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_tile_editor_paste_applied"),
                        ));
                    } else {
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_tile_editor_paste_no_valid_target"),
                        ));
                    }
                    redraw = true;
                } else if id.name == "Tile Editor Dock RGBA Layout View"
                    && matches!(server_ctx.editing_ctx, PixelEditingContext::AvatarFrame(..))
                    && server_ctx.avatar_anchor_slot != AvatarAnchorEditSlot::None
                    && self.apply_avatar_anchor_at(*coord, project, ctx, server_ctx)
                {
                    self.sync_anchor_overlay(project, ui, ctx, server_ctx);
                    redraw = true;
                }
            }
            TheEvent::Copy => {
                if server_ctx.editing_ctx != PixelEditingContext::None {
                    if let Some(texture) = project.get_editing_texture(&server_ctx.editing_ctx) {
                        let selection = if let Some(editor) =
                            ui.get_rgba_layout("Tile Editor Dock RGBA Layout")
                        {
                            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                                rgba_view.selection()
                            } else {
                                FxHashSet::default()
                            }
                        } else {
                            FxHashSet::default()
                        };

                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            if selection.is_empty() {
                                let img = arboard::ImageData {
                                    width: texture.width,
                                    height: texture.height,
                                    bytes: std::borrow::Cow::Borrowed(&texture.data),
                                };
                                let _ = clipboard.set_image(img);
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    fl!("status_tile_editor_copy_texture"),
                                ));
                            } else {
                                let min_x = selection.iter().map(|(x, _)| *x).min().unwrap_or(0);
                                let max_x = selection.iter().map(|(x, _)| *x).max().unwrap_or(0);
                                let min_y = selection.iter().map(|(_, y)| *y).min().unwrap_or(0);
                                let max_y = selection.iter().map(|(_, y)| *y).max().unwrap_or(0);

                                let out_w = (max_x - min_x + 1).max(1) as usize;
                                let out_h = (max_y - min_y + 1).max(1) as usize;
                                let mut out = vec![0_u8; out_w * out_h * 4];

                                for (x, y) in selection {
                                    if x >= 0
                                        && y >= 0
                                        && (x as usize) < texture.width
                                        && (y as usize) < texture.height
                                    {
                                        let src_i =
                                            ((y as usize) * texture.width + (x as usize)) * 4;
                                        let dx = (x - min_x) as usize;
                                        let dy = (y - min_y) as usize;
                                        let dst_i = (dy * out_w + dx) * 4;
                                        out[dst_i..dst_i + 4]
                                            .copy_from_slice(&texture.data[src_i..src_i + 4]);
                                    }
                                }

                                let img = arboard::ImageData {
                                    width: out_w,
                                    height: out_h,
                                    bytes: std::borrow::Cow::Owned(out),
                                };
                                let _ = clipboard.set_image(img);
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    fl!("status_tile_editor_copy_selection"),
                                ));
                            }
                        }
                    }
                }
            }
            TheEvent::Cut => {
                if server_ctx.editing_ctx != PixelEditingContext::None {
                    let selection =
                        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
                            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                                rgba_view.selection()
                            } else {
                                FxHashSet::default()
                            }
                        } else {
                            FxHashSet::default()
                        };

                    if selection.is_empty() {
                        return redraw;
                    }

                    // Copy selected pixels first.
                    if let Some(texture) = project.get_editing_texture(&server_ctx.editing_ctx) {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let min_x = selection.iter().map(|(x, _)| *x).min().unwrap_or(0);
                            let max_x = selection.iter().map(|(x, _)| *x).max().unwrap_or(0);
                            let min_y = selection.iter().map(|(_, y)| *y).min().unwrap_or(0);
                            let max_y = selection.iter().map(|(_, y)| *y).max().unwrap_or(0);

                            let out_w = (max_x - min_x + 1).max(1) as usize;
                            let out_h = (max_y - min_y + 1).max(1) as usize;
                            let mut out = vec![0_u8; out_w * out_h * 4];

                            for (x, y) in &selection {
                                if *x >= 0
                                    && *y >= 0
                                    && (*x as usize) < texture.width
                                    && (*y as usize) < texture.height
                                {
                                    let src_i = ((*y as usize) * texture.width + (*x as usize)) * 4;
                                    let dx = (*x - min_x) as usize;
                                    let dy = (*y - min_y) as usize;
                                    let dst_i = (dy * out_w + dx) * 4;
                                    out[dst_i..dst_i + 4]
                                        .copy_from_slice(&texture.data[src_i..src_i + 4]);
                                }
                            }

                            let img = arboard::ImageData {
                                width: out_w,
                                height: out_h,
                                bytes: std::borrow::Cow::Owned(out),
                            };
                            let _ = clipboard.set_image(img);
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                fl!("status_tile_editor_cut_selection"),
                            ));
                        }
                    }

                    if self.clear_current_selection(project, ui, ctx, server_ctx) {
                        redraw = true;
                    }
                }
            }
            TheEvent::Paste(_, _) => {
                if server_ctx.editing_ctx != PixelEditingContext::None {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if let Ok(img) = clipboard.get_image() {
                            // Convert RGBA image data to a texture
                            let width = img.width;
                            let height = img.height;
                            let data: Vec<u8> = img.bytes.into_owned();

                            if width > 0 && height > 0 {
                                let pasted = rusterix::Texture::new(data, width, height);
                                self.paste_preview_texture = Some(pasted);
                                if self.paste_preview_pos.is_none() {
                                    if let Some(texture) =
                                        project.get_editing_texture(&server_ctx.editing_ctx)
                                    {
                                        self.paste_preview_pos = Some(Vec2::new(
                                            texture.width as i32 / 2,
                                            texture.height as i32 / 2,
                                        ));
                                    } else {
                                        self.paste_preview_pos = Some(Vec2::zero());
                                    }
                                }
                                self.sync_paste_preview(ui, ctx);
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    fl!("status_tile_editor_paste_preview_active"),
                                ));
                                redraw = true;
                            }
                        }
                    }
                }
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(key)) => {
                if *key == TheKeyCode::Escape && self.paste_preview_texture.is_some() {
                    self.clear_paste_preview(ui, ctx);
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        fl!("status_tile_editor_paste_preview_canceled"),
                    ));
                    redraw = true;
                } else if *key == TheKeyCode::Return && self.paste_preview_texture.is_some() {
                    if self.apply_paste_at_preview(project, ui, ctx, server_ctx) {
                        self.clear_paste_preview(ui, ctx);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_tile_editor_paste_applied"),
                        ));
                        redraw = true;
                    } else {
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_tile_editor_paste_no_valid_target"),
                        ));
                    }
                } else if *key == TheKeyCode::Delete
                    && !ui.focus_widget_supports_text_input(ctx)
                    && self.paste_preview_texture.is_none()
                {
                    if self.clear_current_selection(project, ui, ctx, server_ctx) {
                        redraw = true;
                    }
                } else if *key == TheKeyCode::Space && !ui.focus_widget_supports_text_input(ctx) {
                    if server_ctx.editing_ctx != PixelEditingContext::None {
                        self.anim_preview = !self.anim_preview;
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Minimap"),
                            TheValue::Empty,
                        ));
                        redraw = true;
                    }
                }
            }
            TheEvent::KeyDown(TheValue::Char(c)) => {
                if !ui.focus_widget_supports_text_input(ctx) {
                    let c = c.to_ascii_lowercase();
                    if c == 'h' {
                        if self.apply_flip(true, project, ui, ctx, server_ctx) {
                            redraw = true;
                        }
                    } else if c == 'v' && self.apply_flip(false, project, ui, ctx, server_ctx) {
                        redraw = true;
                    }
                }
            }
            _ => {}
        }

        redraw
    }

    fn supports_undo(&self) -> bool {
        self.mode == TilesEditorMode::Pixel
    }

    fn has_changes(&self) -> bool {
        // Check if any tile has changes (index >= 0, meaning not fully undone)
        self.tile_undos.values().any(|undo| undo.has_changes())
    }

    fn mark_saved(&mut self) {
        for undo in self.tile_undos.values_mut() {
            undo.index = -1;
        }
    }

    fn undo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) {
        if let Some(key) = self.current_undo_key {
            if let Some(undo) = self.tile_undos.get_mut(&key) {
                undo.undo(project, ui, ctx);
                self.set_undo_state_to_ui(ctx);
            }
        }
    }

    fn redo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) {
        if let Some(key) = self.current_undo_key {
            if let Some(undo) = self.tile_undos.get_mut(&key) {
                undo.redo(project, ui, ctx);
                self.set_undo_state_to_ui(ctx);
            }
        }
    }

    fn set_undo_state_to_ui(&self, ctx: &mut TheContext) {
        if let Some(key) = self.current_undo_key {
            if let Some(undo) = self.tile_undos.get(&key) {
                if undo.has_undo() {
                    ctx.ui.set_enabled("Undo");
                } else {
                    ctx.ui.set_disabled("Undo");
                }

                if undo.has_redo() {
                    ctx.ui.set_enabled("Redo");
                } else {
                    ctx.ui.set_disabled("Redo");
                }
                return;
            }
        }

        // No tile selected or no undo stack
        ctx.ui.set_disabled("Undo");
        ctx.ui.set_disabled("Redo");
    }

    fn editor_tools(&self) -> Option<Vec<Box<dyn EditorTool>>> {
        if self.mode == TilesEditorMode::Pixel {
            Some(vec![
                Box::new(TileDrawTool::new()),
                Box::new(TileFillTool::new()),
                Box::new(TilePickerTool::new()),
                Box::new(TileEraserTool::new()),
                Box::new(TileSelectTool::new()),
            ])
        } else {
            Some(vec![])
        }
    }

    fn draw_minimap(
        &self,
        buffer: &mut TheRGBABuffer,
        project: &Project,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) -> bool {
        buffer.fill(BLACK);

        // Determine which frame to display
        let display_ctx = if self.anim_preview {
            let frame_count = server_ctx.editing_ctx.get_frame_count(project);
            if frame_count > 0 {
                let frame = server_ctx.animation_counter % frame_count;
                server_ctx.editing_ctx.with_frame(frame)
            } else {
                server_ctx.editing_ctx
            }
        } else {
            server_ctx.editing_ctx
        };

        if let Some(texture) = project.get_editing_texture(&display_ctx) {
            let stride: usize = buffer.stride();

            let src_pixels = &texture.data;
            let src_w = texture.width as f32;
            let src_h = texture.height as f32;

            let dim = buffer.dim();
            let dst_w = dim.width as f32;
            let dst_h = dim.height as f32;

            let scale = (dst_w / src_w).min(dst_h / src_h);
            let draw_w = src_w * scale;
            let draw_h = src_h * scale;

            let offset_x = ((dst_w - draw_w) * 0.5).round() as usize;
            let offset_y = ((dst_h - draw_h) * 0.5).round() as usize;

            let dst_rect = (
                offset_x,
                offset_y,
                draw_w.round() as usize,
                draw_h.round() as usize,
            );

            ctx.draw.blend_scale_chunk(
                buffer.pixels_mut(),
                &dst_rect,
                stride,
                src_pixels,
                &(src_w as usize, src_h as usize),
            );

            return true;
        }
        false
    }

    fn supports_minimap_animation(&self) -> bool {
        true
    }
}

impl TilesEditorDock {
    fn add_node_graph_undo(&self, before: Project, after: Project, ctx: &mut TheContext) {
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::TilePickerEdit(Box::new(before), Box::new(after)),
            ctx,
        );
    }

    fn parse_size_pair(text: &str, max_value: i32) -> Option<(u16, u16)> {
        let normalized = text.trim().to_ascii_lowercase().replace(' ', "");
        let (w, h) = normalized.split_once('x')?;
        let width = w.parse::<i32>().ok()?.clamp(1, max_value) as u16;
        let height = h.parse::<i32>().ok()?.clamp(1, max_value) as u16;
        Some((width, height))
    }

    fn clear_selected_node_ui(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(layout) = ui.get_text_layout(TILE_NODE_SETTINGS_LAYOUT) {
            layout.clear();
            ctx.ui.relayout = true;
        }
        self.set_large_node_preview_overlay(ui, None);
    }

    fn set_selected_node_ui(&self, project: &Project, ui: &mut TheUI, ctx: &mut TheContext) {
        let mut nodeui = TheNodeUI::default();

        if self.mode == TilesEditorMode::NodeSkeleton
            && let Some(group_id) = self.current_node_group_id
            && let Some(node_group) = project.tile_node_groups.get(&group_id)
        {
            let graph_state = self.node_graph_state_for_group(project, group_id);
            match graph_state
                .selected_node
                .and_then(|index| graph_state.nodes.get(index))
                .map(|node| &node.kind)
            {
                Some(TileNodeKind::OutputRoot {
                    roughness,
                    metallic,
                    opacity,
                    emissive,
                }) => {
                    nodeui.add_item(TheNodeUIItem::Text(
                        "tileNodeGraphName".into(),
                        "Graph Name".into(),
                        "Set the name of the procedural graph.".into(),
                        node_group.graph_name.clone(),
                        None,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::OpenTree("output".into()));
                    nodeui.add_item(TheNodeUIItem::Text(
                        "tileNodeGroupSize".into(),
                        "Group Size".into(),
                        "Set the output group size as WxH, for example 2x2 or 5x10.".into(),
                        format!(
                            "{}x{}",
                            node_group.output_grid_width, node_group.output_grid_height
                        ),
                        None,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::Text(
                        "tileNodePixelSize".into(),
                        "Tile Pixel Size".into(),
                        "Set the output tile pixel size as WxH, for example 32x32.".into(),
                        format!(
                            "{}x{}",
                            node_group.tile_pixel_width, node_group.tile_pixel_height
                        ),
                        None,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeOutputRoughness".into(),
                        "Roughness".into(),
                        "Default output roughness when the input is not connected.".into(),
                        *roughness,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeOutputMetallic".into(),
                        "Metallic".into(),
                        "Default output metallic when the input is not connected.".into(),
                        *metallic,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeOutputOpacity".into(),
                        "Opacity".into(),
                        "Default output opacity when the input is not connected.".into(),
                        *opacity,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeOutputEmissive".into(),
                        "Emissive".into(),
                        "Default output emissive when the input is not connected.".into(),
                        *emissive,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::CloseTree);
                }
                Some(TileNodeKind::Subgraph { subgraph_id }) => {
                    let name = project
                        .tile_subgraphs
                        .get(subgraph_id)
                        .map(|s| {
                            if s.name.is_empty() {
                                "Subgraph".to_string()
                            } else {
                                s.name.clone()
                            }
                        })
                        .unwrap_or_else(|| "Missing Subgraph".to_string());
                    nodeui.add_item(TheNodeUIItem::Text(
                        "tileNodeSubgraphName".into(),
                        "Subgraph".into(),
                        "This node evaluates a saved reusable subgraph asset.".into(),
                        name,
                        None,
                        true,
                    ));
                }
                Some(TileNodeKind::GroupUV) => {}
                Some(TileNodeKind::Scalar { value }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeScalarValue".into(),
                        "Value".into(),
                        "Scalar value.".into(),
                        *value,
                        0.0..=1.0,
                        false,
                    ));
                }
                Some(TileNodeKind::Gradient { mode }) => {
                    nodeui.add_item(TheNodeUIItem::Selector(
                        "tileNodeGradientMode".into(),
                        "Mode".into(),
                        "Choose the gradient direction.".into(),
                        vec!["Horizontal".into(), "Vertical".into(), "Radial".into()],
                        *mode as i32,
                    ));
                }
                Some(TileNodeKind::Curve { power }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeCurvePower".into(),
                        "Power".into(),
                        "Curve power. Lower values brighten, higher values darken and tighten the mask.".into(),
                        *power,
                        0.1..=4.0,
                        false,
                    ));
                }
                Some(TileNodeKind::Color { color }) => {
                    nodeui.add_item(TheNodeUIItem::ColorPicker(
                        "tileNodeColorValue".into(),
                        "Color".into(),
                        "Set the generated color.".into(),
                        color.clone(),
                        true,
                    ));
                }
                Some(TileNodeKind::PaletteColor { index }) => {
                    nodeui.add_item(TheNodeUIItem::PaletteSlider(
                        "tileNodePaletteIndex".into(),
                        "Palette Index".into(),
                        "Set the palette index used for the generated color.".into(),
                        *index as i32,
                        project.palette.clone(),
                        false,
                    ));
                }
                Some(TileNodeKind::NearestPalette) => {}
                Some(TileNodeKind::Mix { factor }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeMixFactor".into(),
                        "Factor".into(),
                        "Mix factor between input A and input B.".into(),
                        *factor,
                        0.0..=1.0,
                        false,
                    ));
                }
                Some(TileNodeKind::Min) => {}
                Some(TileNodeKind::Max) => {}
                Some(TileNodeKind::Add) => {}
                Some(TileNodeKind::Subtract) => {}
                Some(TileNodeKind::Checker { scale }) => {
                    nodeui.add_item(TheNodeUIItem::IntEditSlider(
                        "tileNodeCheckerScale".into(),
                        "Scale".into(),
                        "Checker density. Use the A/B input terminals for colors.".into(),
                        *scale as i32,
                        1..=64,
                        false,
                    ));
                }
                Some(TileNodeKind::Noise { scale, seed, wrap }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeNoiseScale".into(),
                        "Scale".into(),
                        "Noise scale.".into(),
                        *scale,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::IntEditSlider(
                        "tileNodeNoiseSeed".into(),
                        "Seed".into(),
                        "Noise seed.".into(),
                        *seed as i32,
                        0..=9999,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::Selector(
                        "tileNodeNoiseMode".into(),
                        "Mode".into(),
                        "Choose whether the noise is free or wraps across the group bounds.".into(),
                        vec!["Free".into(), "Wrap".into()],
                        if *wrap { 1 } else { 0 },
                    ));
                }
                Some(TileNodeKind::Voronoi {
                    scale,
                    seed,
                    jitter,
                    warp_amount,
                    falloff,
                }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeVoronoiScale".into(),
                        "Scale".into(),
                        "Voronoi cell scale.".into(),
                        *scale,
                        0.01..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::IntEditSlider(
                        "tileNodeVoronoiSeed".into(),
                        "Seed".into(),
                        "Voronoi seed.".into(),
                        *seed as i32,
                        0..=9999,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeVoronoiJitter".into(),
                        "Jitter".into(),
                        "Site jitter inside each cell.".into(),
                        *jitter,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeVoronoiWarpAmount".into(),
                        "Warp".into(),
                        "How strongly the optional warp input distorts Voronoi space.".into(),
                        *warp_amount,
                        0.0..=0.25,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeVoronoiFalloff".into(),
                        "Falloff".into(),
                        "How quickly the seam-to-center height ramp rises.".into(),
                        *falloff,
                        0.1..=4.0,
                        false,
                    ));
                }
                Some(TileNodeKind::BoxDivide {
                    scale,
                    gap,
                    rotation,
                    rounding,
                    warp_amount,
                    falloff,
                }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeBoxDivideScale".into(),
                        "Scale".into(),
                        "Subdivision scale.".into(),
                        *scale,
                        0.1..=3.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeBoxDivideGap".into(),
                        "Gap".into(),
                        "Gap or mortar width.".into(),
                        *gap,
                        0.0..=4.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeBoxDivideRotation".into(),
                        "Rotation".into(),
                        "Per-cell rotation amount.".into(),
                        *rotation,
                        0.0..=2.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeBoxDivideRounding".into(),
                        "Rounding".into(),
                        "Cell corner rounding.".into(),
                        *rounding,
                        0.0..=0.5,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeBoxDivideWarpAmount".into(),
                        "Warp".into(),
                        "How strongly the optional warp input distorts box-divide space.".into(),
                        *warp_amount,
                        0.0..=0.25,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeBoxDivideFalloff".into(),
                        "Falloff".into(),
                        "How quickly the seam-to-center height ramp rises.".into(),
                        *falloff,
                        0.1..=4.0,
                        false,
                    ));
                }
                Some(TileNodeKind::Repeat { repeat_x, repeat_y }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeRepeatX".into(),
                        "Repeat X".into(),
                        "Horizontal repeat count.".into(),
                        *repeat_x,
                        0.1..=64.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeRepeatY".into(),
                        "Repeat Y".into(),
                        "Vertical repeat count.".into(),
                        *repeat_y,
                        0.1..=64.0,
                        false,
                    ));
                }
                Some(TileNodeKind::Offset { x, y }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeOffsetX".into(),
                        "Offset X".into(),
                        "Horizontal group-space offset.".into(),
                        *x,
                        -1.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeOffsetY".into(),
                        "Offset Y".into(),
                        "Vertical group-space offset.".into(),
                        *y,
                        -1.0..=1.0,
                        false,
                    ));
                }
                Some(TileNodeKind::Scale { x, y }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeScaleX".into(),
                        "Scale X".into(),
                        "Horizontal group-space scale.".into(),
                        *x,
                        0.1..=16.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeScaleY".into(),
                        "Scale Y".into(),
                        "Vertical group-space scale.".into(),
                        *y,
                        0.1..=16.0,
                        false,
                    ));
                }
                Some(TileNodeKind::Rotate { angle }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeRotateAngle".into(),
                        "Angle".into(),
                        "Rotation angle in degrees.".into(),
                        *angle,
                        -180.0..=180.0,
                        false,
                    ));
                }
                Some(TileNodeKind::DirectionalWarp { amount, angle }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeDirectionalWarpAmount".into(),
                        "Amount".into(),
                        "Directional warp amount.".into(),
                        *amount,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeDirectionalWarpAngle".into(),
                        "Angle".into(),
                        "Directional warp angle in degrees.".into(),
                        *angle,
                        -180.0..=180.0,
                        false,
                    ));
                }
                Some(TileNodeKind::Brick {
                    columns,
                    rows,
                    offset,
                    warp_amount,
                    falloff,
                }) => {
                    nodeui.add_item(TheNodeUIItem::IntEditSlider(
                        "tileNodeBrickColumns".into(),
                        "Columns".into(),
                        "Brick columns.".into(),
                        *columns as i32,
                        1..=64,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::IntEditSlider(
                        "tileNodeBrickRows".into(),
                        "Rows".into(),
                        "Brick rows.".into(),
                        *rows as i32,
                        1..=64,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeBrickOffset".into(),
                        "Offset".into(),
                        "Odd-row offset.".into(),
                        *offset,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeBrickWarpAmount".into(),
                        "Warp".into(),
                        "How strongly the optional warp input distorts brick space.".into(),
                        *warp_amount,
                        0.0..=0.25,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeBrickFalloff".into(),
                        "Falloff".into(),
                        "How quickly the seam-to-center brick height ramp rises.".into(),
                        *falloff,
                        0.1..=4.0,
                        false,
                    ));
                }
                Some(TileNodeKind::Disc {
                    scale,
                    seed,
                    jitter,
                    radius,
                    warp_amount,
                    falloff,
                }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeDiscScale".into(),
                        "Scale".into(),
                        "Disc layout scale.".into(),
                        *scale,
                        0.01..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::IntEditSlider(
                        "tileNodeDiscSeed".into(),
                        "Seed".into(),
                        "Disc layout seed.".into(),
                        *seed as i32,
                        0..=9999,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeDiscJitter".into(),
                        "Jitter".into(),
                        "Disc center jitter inside each cell.".into(),
                        *jitter,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeDiscRadius".into(),
                        "Radius".into(),
                        "Disc radius relative to the cell.".into(),
                        *radius,
                        0.05..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeDiscWarpAmount".into(),
                        "Warp".into(),
                        "How strongly the optional warp input distorts disc space.".into(),
                        *warp_amount,
                        0.0..=0.25,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeDiscFalloff".into(),
                        "Falloff".into(),
                        "How quickly the disc height ramp rises.".into(),
                        *falloff,
                        0.1..=4.0,
                        false,
                    ));
                }
                Some(TileNodeKind::MaskBlend { factor }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeMaskBlendFactor".into(),
                        "Factor".into(),
                        "Mask blend factor.".into(),
                        *factor,
                        0.0..=1.0,
                        false,
                    ));
                }
                Some(TileNodeKind::MakeMaterial) => {}
                Some(TileNodeKind::MaterialMix { factor }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeMaterialMixFactor".into(),
                        "Factor".into(),
                        "Material mix factor.".into(),
                        *factor,
                        0.0..=1.0,
                        false,
                    ));
                }
                Some(TileNodeKind::Levels { level, width }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeLevelsLevel".into(),
                        "Level".into(),
                        "Center of the selected field band.".into(),
                        *level,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeLevelsWidth".into(),
                        "Width".into(),
                        "Width of the selected field band.".into(),
                        *width,
                        0.001..=1.0,
                        false,
                    ));
                }
                Some(TileNodeKind::IdRandom) => {}
                Some(TileNodeKind::Threshold { cutoff }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeThresholdCutoff".into(),
                        "Cutoff".into(),
                        "Threshold cutoff.".into(),
                        *cutoff,
                        0.0..=1.0,
                        false,
                    ));
                }
                Some(TileNodeKind::Blur { radius }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeBlurRadius".into(),
                        "Radius".into(),
                        "Blur radius in group space.".into(),
                        *radius,
                        0.001..=0.08,
                        false,
                    ));
                }
                Some(TileNodeKind::SlopeBlur { radius, amount }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeSlopeBlurRadius".into(),
                        "Radius".into(),
                        "Sampling radius for slope-driven height smearing.".into(),
                        *radius,
                        0.001..=0.08,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeSlopeBlurAmount".into(),
                        "Amount".into(),
                        "How strongly nearby height pulls and smears the field.".into(),
                        *amount,
                        0.0..=1.0,
                        false,
                    ));
                }
                Some(TileNodeKind::HeightEdge { radius }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeHeightEdgeRadius".into(),
                        "Radius".into(),
                        "Sampling radius for deriving edges from a height field.".into(),
                        *radius,
                        0.001..=0.08,
                        false,
                    ));
                }
                Some(TileNodeKind::Warp { amount }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeWarpAmount".into(),
                        "Amount".into(),
                        "Warp amount.".into(),
                        *amount,
                        0.0..=1.0,
                        false,
                    ));
                }
                Some(TileNodeKind::Invert) => {}
                Some(TileNodeKind::Multiply) => {}
                _ => {
                    nodeui.add_item(TheNodeUIItem::Text(
                        "tileNodeGraphName".into(),
                        "Graph Name".into(),
                        "Set the name of the procedural graph.".into(),
                        node_group.graph_name.clone(),
                        None,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::OpenTree("output".into()));
                    nodeui.add_item(TheNodeUIItem::Text(
                        "tileNodeGroupSize".into(),
                        "Group Size".into(),
                        "Set the output group size as WxH, for example 2x2 or 5x10.".into(),
                        format!(
                            "{}x{}",
                            node_group.output_grid_width, node_group.output_grid_height
                        ),
                        None,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::Text(
                        "tileNodePixelSize".into(),
                        "Tile Pixel Size".into(),
                        "Set the output tile pixel size as WxH, for example 32x32.".into(),
                        format!(
                            "{}x{}",
                            node_group.tile_pixel_width, node_group.tile_pixel_height
                        ),
                        None,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::CloseTree);
                }
            }
        }

        if let Some(layout) = ui.get_text_layout(TILE_NODE_SETTINGS_LAYOUT) {
            nodeui.apply_to_text_layout(layout);
            ctx.ui.relayout = true;
        }
        self.sync_large_node_preview(project, ui);
    }

    fn set_large_node_preview_overlay(&self, ui: &mut TheUI, overlay: Option<TheRGBABuffer>) {
        if let Some(node_view) = ui.get_node_canvas_view(TILE_NODE_CANVAS_VIEW) {
            node_view.set_overlay(overlay);
        }
    }

    fn sync_large_node_preview(&self, project: &Project, ui: &mut TheUI) {
        let Some(group_id) = self.current_node_group_id else {
            self.set_large_node_preview_overlay(ui, None);
            return;
        };
        let state = self.node_graph_state_for_group(project, group_id);
        let preview_node = state.selected_node.unwrap_or(0);
        let preview = self.render_node_preview(project, &state, preview_node, 384, 240);
        self.set_large_node_preview_overlay(ui, Some(preview));
    }

    fn render_node_preview(
        &self,
        project: &Project,
        state: &TileNodeGraphState,
        node_index: usize,
        width: i32,
        height: i32,
    ) -> TheRGBABuffer {
        let mut preview = TheRGBABuffer::new(TheDim::sized(width, height));
        if let Some(exchange) =
            self.shared_preview_exchange(project, state, node_index, width.max(1), height.max(1))
        {
            let palette = project
                .palette
                .colors
                .iter()
                .filter_map(|color| color.clone())
                .collect();
            let renderer = shared::tilegraph::TileGraphRenderer::new(palette);
            let rendered = renderer.render_graph(&exchange);
            let pixels = if state.preview_mode == 1 {
                &rendered.sheet_material
            } else {
                &rendered.sheet_color
            };
            if pixels.len() == (width.max(1) * height.max(1) * 4) as usize {
                preview.pixels_mut().copy_from_slice(pixels);
                return preview;
            }
        }
        preview
    }

    fn shared_preview_exchange(
        &self,
        project: &Project,
        state: &TileNodeGraphState,
        node_index: usize,
        width: i32,
        height: i32,
    ) -> Option<shared::tilegraph::TileNodeGraphExchange> {
        let mut preview_state = state.clone();
        preview_state.ensure_root();
        preview_state
            .connections
            .retain(|(_, _, dest_node, _)| *dest_node != 0 || node_index == 0);
        if node_index != 0 {
            let (src_terminal, dest_terminal) = Self::preview_connection_for_node(
                preview_state.preview_mode,
                &preview_state,
                node_index,
            );
            preview_state
                .connections
                .retain(|(_, _, dest_node, _)| !(*dest_node == 0));
            preview_state
                .connections
                .push((node_index as u16, src_terminal, 0, dest_terminal));
        }

        let exchange = TileNodeGraphExchange {
            version: 1,
            graph_name: String::new(),
            palette_colors: vec![],
            output_grid_width: 1,
            output_grid_height: 1,
            tile_pixel_width: width.max(1) as u16,
            tile_pixel_height: height.max(1) as u16,
            graph_state: preview_state,
        };
        let resolver = ProjectTileSubgraphResolver { project };
        Some(flatten_graph_exchange_with(&exchange, &resolver))
    }

    fn preview_connection_for_node(
        preview_mode: u8,
        state: &TileNodeGraphState,
        node_index: usize,
    ) -> (u8, u8) {
        if preview_mode == 1 {
            return (0, 2);
        }
        match state.nodes.get(node_index).map(|node| &node.kind) {
            Some(
                TileNodeKind::Voronoi { .. }
                | TileNodeKind::Brick { .. }
                | TileNodeKind::Disc { .. }
                | TileNodeKind::BoxDivide { .. },
            ) => (1, 0),
            _ => (0, 0),
        }
    }

    fn render_node_group_tiles(&self, project: &mut Project, group_id: Uuid) {
        let Some(node_group) = project.tile_node_groups.get(&group_id).cloned() else {
            return;
        };
        let state = self.node_graph_state_for_group(project, group_id);

        if let Some(exchange) = self.shared_exchange_for_group(project, &node_group, &state) {
            let palette = project
                .palette
                .colors
                .iter()
                .filter_map(|color| color.clone())
                .collect();
            let renderer = shared::tilegraph::TileGraphRenderer::new(palette);
            let rendered = renderer.render_graph(&exchange);
            self.apply_rendered_node_group_tiles(project, group_id, &node_group, rendered);
        }
    }

    fn apply_rendered_node_group_tiles(
        &self,
        project: &mut Project,
        group_id: Uuid,
        node_group: &NodeGroupAsset,
        rendered: shared::tilegraph::RenderedTileGraph,
    ) {
        let sheet_width = rendered.grid_width * rendered.tile_width;
        let sheet_height = rendered.grid_height * rendered.tile_height;
        let mut render_members = Vec::new();
        if let Some(group) = project.tile_groups.get_mut(&group_id) {
            group.width = node_group.output_grid_width;
            group.height = node_group.output_grid_height;

            let required = group.width as usize * group.height as usize;
            while group.members.len() < required {
                let tile = rusterix::Tile::empty();
                let tile_id = tile.id;
                project.tiles.insert(tile_id, tile);
                let index = group.members.len();
                let x = (index % group.width as usize) as u16;
                let y = (index / group.width as usize) as u16;
                group
                    .members
                    .push(rusterix::TileGroupMemberRef { tile_id, x, y });
            }
            while group.members.len() > required {
                if let Some(member) = group.members.pop() {
                    project.tiles.shift_remove(&member.tile_id);
                }
            }

            for y in 0..group.height {
                for x in 0..group.width {
                    let idx = y as usize * group.width as usize + x as usize;
                    if let Some(member) = group.members.get_mut(idx) {
                        member.x = x;
                        member.y = y;
                        render_members.push((idx, member.tile_id));
                    }
                }
            }
        }

        for (idx, tile_id) in render_members {
            let Some(pixels) = rendered.tiles_color.get(idx).cloned() else {
                continue;
            };
            let Some(material_rgba) = rendered.tiles_material.get(idx) else {
                continue;
            };
            let mut materials = vec![0_u8; material_rgba.len()];
            for (src, dst) in material_rgba
                .chunks_exact(4)
                .zip(materials.chunks_exact_mut(4))
            {
                let packed_r = ((src[0] as f32 / 255.0) * 15.0).round() as u16;
                let packed_m = ((src[1] as f32 / 255.0) * 15.0).round() as u16;
                let packed_o = ((src[2] as f32 / 255.0) * 15.0).round() as u16;
                let packed_e = ((src[3] as f32 / 255.0) * 15.0).round() as u16;
                let mat = packed_r | (packed_m << 4) | (packed_o << 8) | (packed_e << 12);
                let bytes = mat.to_le_bytes();
                dst[0] = bytes[0];
                dst[1] = bytes[1];
                dst[2] = 127;
                dst[3] = 255;
            }

            if let Some(tile) = project.tiles.get_mut(&tile_id) {
                if tile.textures.is_empty()
                    || tile.textures[0].width != rendered.tile_width
                    || tile.textures[0].height != rendered.tile_height
                {
                    tile.textures = vec![rusterix::Texture::alloc(
                        rendered.tile_width,
                        rendered.tile_height,
                    )];
                }
                tile.role = rusterix::TileRole::ManMade;
                tile.blocking = false;
                tile.scale = 1.0;
                let texture = &mut tile.textures[0];
                texture.data.copy_from_slice(&pixels);
                texture.data_ext = Some(materials);
                let tile_x = idx % rendered.grid_width;
                let tile_y = idx / rendered.grid_width;
                let origin_x = tile_x * rendered.tile_width;
                let origin_y = tile_y * rendered.tile_height;
                let period_x = sheet_width.saturating_sub(1).max(1) as isize;
                let period_y = sheet_height.saturating_sub(1).max(1) as isize;
                let sample_h = |xx: isize, yy: isize| -> f32 {
                    let sx = xx.rem_euclid(period_x) as usize;
                    let sy = yy.rem_euclid(period_y) as usize;
                    rendered.sheet_height[sy * sheet_width + sx] as f32 / 255.0
                };
                let strength = 0.35f32;
                for py in 0..rendered.tile_height {
                    for px in 0..rendered.tile_width {
                        let gx = (origin_x + px) as isize;
                        let gy = (origin_y + py) as isize;
                        let tl = sample_h(gx - 1, gy - 1);
                        let tc = sample_h(gx, gy - 1);
                        let tr = sample_h(gx + 1, gy - 1);
                        let cl = sample_h(gx - 1, gy);
                        let cr = sample_h(gx + 1, gy);
                        let bl = sample_h(gx - 1, gy + 1);
                        let bc = sample_h(gx, gy + 1);
                        let br = sample_h(gx + 1, gy + 1);

                        let dx = ((-tl) + tr + (-2.0 * cl) + (2.0 * cr) + (-bl) + br) * strength;
                        let dy = ((-tl) + (-2.0 * tc) + (-tr) + bl + (2.0 * bc) + br) * strength;
                        let nx = -dx;
                        let ny = -dy;
                        let nz = 1.0f32;
                        let len = (nx * nx + ny * ny + nz * nz).sqrt();
                        let (nx, ny) = if len > 0.0 {
                            (nx / len, ny / len)
                        } else {
                            (0.0, 0.0)
                        };
                        texture.set_normal(px as u32, py as u32, nx, ny);
                    }
                }
            }
        }
    }

    fn shared_exchange_for_group(
        &self,
        project: &Project,
        node_group: &NodeGroupAsset,
        state: &TileNodeGraphState,
    ) -> Option<shared::tilegraph::TileNodeGraphExchange> {
        let resolver = ProjectTileSubgraphResolver { project };
        let exchange = TileNodeGraphExchange::new(
            node_group.graph_name.clone(),
            node_group.output_grid_width,
            node_group.output_grid_height,
            node_group.tile_pixel_width,
            node_group.tile_pixel_height,
            state.clone(),
        );
        Some(flatten_graph_exchange_with(&exchange, &resolver))
    }

    fn node_graph_state_for_group(&self, project: &Project, group_id: Uuid) -> TileNodeGraphState {
        project
            .tile_node_groups
            .get(&group_id)
            .map(|node_group| TileNodeGraphState::from_graph_data(&node_group.graph_data))
            .unwrap_or_default()
    }

    fn export_node_graph_file(
        &self,
        project: &Project,
        group_id: Uuid,
        path: &std::path::Path,
    ) -> Result<std::path::PathBuf, String> {
        let Some(node_group) = project.tile_node_groups.get(&group_id) else {
            return Err("Node group not found.".to_string());
        };
        let mut exchange = TileNodeGraphExchange::new(
            node_group.graph_name.clone(),
            node_group.output_grid_width,
            node_group.output_grid_height,
            node_group.tile_pixel_width,
            node_group.tile_pixel_height,
            self.node_graph_state_for_group(project, group_id),
        );
        exchange.palette_colors = project
            .palette
            .colors
            .iter()
            .filter_map(|color| color.clone())
            .collect();
        let mut document =
            TileGraphDocument::from_exchange(&exchange).map_err(|e| e.to_string())?;
        if let Some(palette) = &mut document.palette {
            palette.name = Some("Project Palette".to_string());
        }
        let text = document.to_toml_pretty().map_err(|e| e.to_string())?;
        let path = if path.extension().is_some() {
            path.to_path_buf()
        } else {
            path.with_extension("tilegraph")
        };
        std::fs::write(&path, text).map_err(|e| e.to_string())?;
        Ok(path)
    }

    fn import_node_graph_file(
        &self,
        project: &mut Project,
        group_id: Uuid,
        path: &std::path::Path,
    ) -> Result<(), String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let is_tilegraph = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tilegraph"));
        let mut exchange: TileNodeGraphExchange = if is_tilegraph {
            TileGraphDocument::from_toml_str(&text)
                .and_then(|doc| doc.to_exchange())
                .map_err(|e| e.to_string())?
        } else {
            serde_json::from_str(&text).map_err(|e| e.to_string())?
        };
        exchange.graph_state.ensure_root();

        let Some(node_group) = project.tile_node_groups.get_mut(&group_id) else {
            return Err("Node group not found.".to_string());
        };
        node_group.graph_name = exchange.graph_name;
        node_group.output_grid_width = exchange.output_grid_width.max(1);
        node_group.output_grid_height = exchange.output_grid_height.max(1);
        node_group.tile_pixel_width = exchange.tile_pixel_width.max(1);
        node_group.tile_pixel_height = exchange.tile_pixel_height.max(1);
        node_group.graph_data =
            serde_json::to_string(&exchange.graph_state).map_err(|e| e.to_string())?;

        if let Some(group) = project.tile_groups.get_mut(&group_id) {
            group.width = node_group.output_grid_width;
            group.height = node_group.output_grid_height;
        }
        Ok(())
    }

    fn store_node_graph_state(
        &self,
        project: &mut Project,
        group_id: Uuid,
        state: &TileNodeGraphState,
    ) {
        if let Some(node_group) = project.tile_node_groups.get_mut(&group_id)
            && let Ok(json) = serde_json::to_string(state)
        {
            node_group.graph_data = json;
        }
    }

    fn set_node_group_canvas(&self, project: &Project, ui: &mut TheUI) {
        let mut canvas = TheNodeCanvas {
            node_width: 136,
            ..Default::default()
        };
        canvas.categories.insert(
            "ColorSocket".to_string(),
            TheColor::from_u8_array_3([224, 154, 96]),
        );
        canvas.categories.insert(
            "MaterialSocket".to_string(),
            TheColor::from_u8_array_3([108, 178, 232]),
        );
        canvas.categories.insert(
            "FieldSocket".to_string(),
            TheColor::from_u8_array_3([196, 196, 196]),
        );
        canvas.categories.insert(
            "CoordSocket".to_string(),
            TheColor::from_u8_array_3([176, 128, 224]),
        );
        canvas.categories.insert(
            "NodeOutput".to_string(),
            TheColor::from_u8_array_3([200, 140, 90]),
        );

        if let Some(group_id) = self.current_node_group_id
            && project.tile_node_groups.contains_key(&group_id)
        {
            let state = self.node_graph_state_for_group(project, group_id);
            canvas.offset = Vec2::new(state.offset.0, state.offset.1);
            canvas.selected_node = Some(state.selected_node.unwrap_or(0));
            canvas.connections = state.connections.clone();

            for (node_index, node) in state.nodes.iter().enumerate() {
                let preview = self.render_node_preview(project, &state, node_index, 111, 104);
                match &node.kind {
                    TileNodeKind::OutputRoot { .. } => {
                        canvas.nodes.push(TheNode {
                            name: "Tile Group Output".to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "Color".to_string(),
                                    category_name: "ColorSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Height".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Roughness".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Metallic".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Opacity".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Emissive".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                            ],
                            outputs: vec![],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: false,
                        });
                    }
                    TileNodeKind::Subgraph { subgraph_id } => {
                        let name = project
                            .tile_subgraphs
                            .get(subgraph_id)
                            .map(|s| {
                                if s.name.is_empty() {
                                    "Subgraph".to_string()
                                } else {
                                    s.name.clone()
                                }
                            })
                            .unwrap_or_else(|| "Missing Subgraph".to_string());
                        canvas.nodes.push(TheNode {
                            name,
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![],
                            outputs: vec![
                                TheNodeTerminal {
                                    name: "Color".to_string(),
                                    category_name: "ColorSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Material".to_string(),
                                    category_name: "MaterialSocket".to_string(),
                                },
                            ],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::GroupUV => {
                        canvas.nodes.push(TheNode {
                            name: "Group UV".to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "CoordSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Scalar { value } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Scalar {:.2}", value),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Gradient { mode } => {
                        let name = match mode {
                            0 => "Gradient X",
                            1 => "Gradient Y",
                            _ => "Gradient Radial",
                        };
                        canvas.nodes.push(TheNode {
                            name: name.to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Curve { power } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Curve {:.2}", power),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Mask".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Color { color: _ } => {
                        canvas.nodes.push(TheNode {
                            name: "Color".to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::PaletteColor { index: _ } => {
                        canvas.nodes.push(TheNode {
                            name: "Palette Index".to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::NearestPalette => {
                        canvas.nodes.push(TheNode {
                            name: "Nearest Palette".to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Mix { factor } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Mix {:.2}", factor),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "A".to_string(),
                                    category_name: "ColorSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "B".to_string(),
                                    category_name: "ColorSocket".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Min => {
                        canvas.nodes.push(TheNode {
                            name: "Min".to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "A".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "B".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Field".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Max => {
                        canvas.nodes.push(TheNode {
                            name: "Max".to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "A".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "B".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Field".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Add => {
                        canvas.nodes.push(TheNode {
                            name: "Add".to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "A".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "B".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Field".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Subtract => {
                        canvas.nodes.push(TheNode {
                            name: "Subtract".to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "A".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "B".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Field".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Checker { .. } => {
                        canvas.nodes.push(TheNode {
                            name: "Checker".to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "A".to_string(),
                                    category_name: "ColorSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "B".to_string(),
                                    category_name: "ColorSocket".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Noise { .. } => {
                        canvas.nodes.push(TheNode {
                            name: "Noise".to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Voronoi { .. } => {
                        canvas.nodes.push(TheNode {
                            name: "Voronoi".to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "Warp".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            outputs: vec![
                                TheNodeTerminal {
                                    name: "Center".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Height".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Cell Id".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                            ],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::IdRandom => {
                        canvas.nodes.push(TheNode {
                            name: "Id Random".to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "Id".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Field".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::BoxDivide {
                        scale,
                        gap,
                        rounding,
                        warp_amount,
                        falloff,
                        ..
                    } => {
                        canvas.nodes.push(TheNode {
                            name: format!(
                                "Box Divide {:.1}/{:.1}/{:.2}/{:.2}/{:.2}",
                                scale, gap, rounding, warp_amount, falloff
                            ),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "Warp".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            outputs: vec![
                                TheNodeTerminal {
                                    name: "Center".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Height".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Cell Id".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                            ],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Offset { x, y } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Offset {:.2},{:.2}", x, y),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Scale { x, y } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Scale {:.2},{:.2}", x, y),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Repeat { repeat_x, repeat_y } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Repeat {:.1}x{:.1}", repeat_x, repeat_y),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Rotate { angle } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Rotate {:.0}", angle),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::DirectionalWarp { amount, angle } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Dir Warp {:.2}@{:.0}", amount, angle),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "In".to_string(),
                                    category_name: "ColorSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Warp".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Brick {
                        columns,
                        rows,
                        offset,
                        warp_amount,
                        falloff,
                    } => {
                        canvas.nodes.push(TheNode {
                            name: format!(
                                "Brick {}x{} {:.2}/{:.2}/{:.2}",
                                columns, rows, offset, warp_amount, falloff
                            ),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "Warp".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            outputs: vec![
                                TheNodeTerminal {
                                    name: "Center".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Height".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Cell Id".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                            ],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Disc {
                        scale,
                        radius,
                        warp_amount,
                        falloff,
                        ..
                    } => {
                        canvas.nodes.push(TheNode {
                            name: format!(
                                "Disc {:.2}/{:.2}/{:.2}/{:.2}",
                                scale, radius, warp_amount, falloff
                            ),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "Warp".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            outputs: vec![
                                TheNodeTerminal {
                                    name: "Center".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Height".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Cell Id".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                            ],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Multiply => {
                        canvas.nodes.push(TheNode {
                            name: "Multiply".to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "A".to_string(),
                                    category_name: "ColorSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "B".to_string(),
                                    category_name: "ColorSocket".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::MakeMaterial => {
                        canvas.nodes.push(TheNode {
                            name: "Make Material".to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "Roughness".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Metallic".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Opacity".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Emissive".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Material".to_string(),
                                category_name: "MaterialSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Material {
                        roughness,
                        metallic,
                        opacity,
                        emissive,
                    } => {
                        canvas.nodes.push(TheNode {
                            name: format!(
                                "Legacy Material {:.2}/{:.2}/{:.2}/{:.2}",
                                roughness, metallic, opacity, emissive
                            ),
                            status_text: Some(
                                "Legacy node. Use the output node material inputs instead."
                                    .to_string(),
                            ),
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![],
                            outputs: vec![TheNodeTerminal {
                                name: "Material".to_string(),
                                category_name: "MaterialSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::MaterialMix { factor } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Material Mix {:.2}", factor),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "A".to_string(),
                                    category_name: "MaterialSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "B".to_string(),
                                    category_name: "MaterialSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Mask".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Material".to_string(),
                                category_name: "MaterialSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::MaskBlend { factor } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Mask Blend {:.2}", factor),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "A".to_string(),
                                    category_name: "ColorSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "B".to_string(),
                                    category_name: "ColorSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Mask".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Levels { level, width } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Levels {:.2}/{:.2}", level, width),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Threshold { cutoff } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Threshold {:.2}", cutoff),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Blur { radius } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Blur {:.3}", radius),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Mask".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::SlopeBlur { radius, amount } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Slope Blur {:.3}/{:.2}", radius, amount),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Mask".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::HeightEdge { radius } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Height Edge {:.3}", radius),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Mask".to_string(),
                                category_name: "FieldSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Warp { amount } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Warp {:.2}", amount),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "In".to_string(),
                                    category_name: "ColorSocket".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Warp".to_string(),
                                    category_name: "FieldSocket".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Invert => {
                        canvas.nodes.push(TheNode {
                            name: "Invert".to_string(),
                            status_text: None,
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "ColorSocket".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                }
            }
        }

        ui.set_node_canvas(TILE_NODE_CANVAS_VIEW, canvas);
    }

    fn refresh_node_group_ui(&self, project: &Project, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(group_id) = self.current_node_group_id {
            if let Some(widget) = ui.get_widget("Tile Node Subgraphs") {
                let items = if project.tile_subgraphs.is_empty() {
                    vec![TheContextMenuItem::new(
                        "No Saved Subgraphs".to_string(),
                        TheId::named("Tile Node No Subgraph"),
                    )]
                } else {
                    project
                        .tile_subgraphs
                        .values()
                        .map(|subgraph| {
                            TheContextMenuItem::new(
                                if subgraph.name.is_empty() {
                                    "Subgraph".to_string()
                                } else {
                                    subgraph.name.clone()
                                },
                                TheId::named_with_id("Tile Node Add Subgraph", subgraph.id),
                            )
                        })
                        .collect()
                };
                widget.set_context_menu(Some(TheContextMenu {
                    items,
                    ..Default::default()
                }));
            }
            let state = self.node_graph_state_for_group(project, group_id);
            if let Some(drop_down) = ui.get_drop_down_menu("Tile Node Preview Mode") {
                drop_down.set_selected_index(state.preview_mode as i32);
            }
            if let Some(drop_down) = ui.get_drop_down_menu("Tile Node Debug Mode") {
                let index = state
                    .selected_node
                    .and_then(|i| state.nodes.get(i))
                    .map(|node| {
                        if node.solo {
                            3
                        } else if node.mute {
                            2
                        } else if node.bypass {
                            1
                        } else {
                            0
                        }
                    })
                    .unwrap_or(0);
                drop_down.set_selected_index(index);
            }
            self.sync_large_node_preview(project, ui);
        }
        self.set_node_group_canvas(project, ui);
        ctx.ui.relayout = true;
    }

    fn apply_avatar_anchor_at(
        &mut self,
        coord: Vec2<i32>,
        project: &mut Project,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) -> bool {
        let editing_ctx = server_ctx.editing_ctx;
        let Some(before) = project.get_editing_avatar_frame(&editing_ctx) else {
            return false;
        };
        let before_main = before.weapon_main_anchor;
        let before_off = before.weapon_off_anchor;

        let clicked = Some((coord.x as i16, coord.y as i16));
        if let Some(frame) = project.get_editing_avatar_frame_mut(&editing_ctx) {
            match server_ctx.avatar_anchor_slot {
                AvatarAnchorEditSlot::Main => {
                    if frame.weapon_main_anchor == clicked {
                        frame.weapon_main_anchor = None;
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_avatar_anchor_clear_main"),
                        ));
                    } else {
                        frame.weapon_main_anchor = clicked;
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_avatar_anchor_set_main"),
                        ));
                    }
                }
                AvatarAnchorEditSlot::Off => {
                    if frame.weapon_off_anchor == clicked {
                        frame.weapon_off_anchor = None;
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_avatar_anchor_clear_off"),
                        ));
                    } else {
                        frame.weapon_off_anchor = clicked;
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_avatar_anchor_set_off"),
                        ));
                    }
                }
                AvatarAnchorEditSlot::None => return false,
            }

            let after_main = frame.weapon_main_anchor;
            let after_off = frame.weapon_off_anchor;
            if before_main != after_main || before_off != after_off {
                let atom = TileEditorUndoAtom::AvatarAnchorEdit(
                    editing_ctx,
                    before_main,
                    before_off,
                    after_main,
                    after_off,
                );
                self.add_undo(atom, ctx);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Editing Texture Updated"),
                    TheValue::Empty,
                ));
                return true;
            }
        }
        false
    }

    fn sync_anchor_overlay(
        &mut self,
        project: &Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) {
        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout")
            && let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view()
        {
            let points =
                if let Some(frame) = project.get_editing_avatar_frame(&server_ctx.editing_ctx) {
                    let mut p = vec![];
                    if let Some((x, y)) = frame.weapon_main_anchor {
                        p.push((Vec2::new(x as i32, y as i32), [255, 80, 80, 255]));
                    }
                    if let Some((x, y)) = frame.weapon_off_anchor {
                        p.push((Vec2::new(x as i32, y as i32), [80, 200, 255, 255]));
                    }
                    p
                } else {
                    vec![]
                };
            rgba_view.set_anchor_points(points);
            editor.relayout(ctx);
        }
    }

    fn apply_flip(
        &mut self,
        horizontal: bool,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if self.paste_preview_texture.is_some() {
            return false;
        }

        let selection = if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                rgba_view.selection()
            } else {
                FxHashSet::default()
            }
        } else {
            FxHashSet::default()
        };

        let editing_ctx = server_ctx.editing_ctx;
        let before = project.get_editing_texture(&editing_ctx).cloned();
        let Some(texture) = project.get_editing_texture_mut(&editing_ctx) else {
            return false;
        };
        let Some(before) = before else {
            return false;
        };

        let mut after_data = texture.data.clone();
        let w = texture.width as i32;
        let h = texture.height as i32;

        if selection.is_empty() {
            for y in 0..h {
                for x in 0..w {
                    let sx = if horizontal { w - 1 - x } else { x };
                    let sy = if horizontal { y } else { h - 1 - y };
                    let src_i = ((sy as usize) * texture.width + (sx as usize)) * 4;
                    let dst_i = ((y as usize) * texture.width + (x as usize)) * 4;
                    after_data[dst_i..dst_i + 4].copy_from_slice(&texture.data[src_i..src_i + 4]);
                }
            }
        } else {
            let min_x = selection.iter().map(|(x, _)| *x).min().unwrap_or(0);
            let max_x = selection.iter().map(|(x, _)| *x).max().unwrap_or(0);
            let min_y = selection.iter().map(|(_, y)| *y).min().unwrap_or(0);
            let max_y = selection.iter().map(|(_, y)| *y).max().unwrap_or(0);

            for (x, y) in &selection {
                let sx = if horizontal { min_x + (max_x - *x) } else { *x };
                let sy = if horizontal { *y } else { min_y + (max_y - *y) };
                if sx >= 0
                    && sy >= 0
                    && sx < w
                    && sy < h
                    && selection.contains(&(sx, sy))
                    && *x >= 0
                    && *y >= 0
                    && *x < w
                    && *y < h
                {
                    let src_i = ((sy as usize) * texture.width + (sx as usize)) * 4;
                    let dst_i = ((*y as usize) * texture.width + (*x as usize)) * 4;
                    after_data[dst_i..dst_i + 4].copy_from_slice(&texture.data[src_i..src_i + 4]);
                }
            }
        }

        if after_data == texture.data {
            return false;
        }

        texture.data = after_data;
        texture.generate_normals(true);

        let after = texture.clone();
        let atom = TileEditorUndoAtom::TextureEdit(editing_ctx, before, after);
        self.add_undo(atom, ctx);

        match editing_ctx {
            PixelEditingContext::Tile(tile_id, _) => {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Tile Updated"),
                    TheValue::Id(tile_id),
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Tilepicker"),
                    TheValue::Empty,
                ));
            }
            PixelEditingContext::AvatarFrame(..) => {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Editing Texture Updated"),
                    TheValue::Empty,
                ));
            }
            PixelEditingContext::None => {}
        }
        true
    }

    /// Switch to a different tile and update undo button states
    pub fn switch_to_tile(
        &mut self,
        tile: &rusterix::Tile,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        self.current_tile_id = Some(tile.id);
        self.current_undo_key = Some(tile.id);

        // Verify frame index is valid for the new tile
        if server_ctx.curr_tile_frame_index >= tile.textures.len() {
            server_ctx.curr_tile_frame_index = 0;
        }

        server_ctx.editing_ctx =
            PixelEditingContext::Tile(tile.id, server_ctx.curr_tile_frame_index);

        self.set_undo_state_to_ui(ctx);
    }

    /// Set the current frame/texture index
    pub fn set_frame_index(
        &mut self,
        index: usize,
        project: &Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        // Verify the index is valid for current tile
        if let Some(tile_id) = self.current_tile_id {
            if let Some(tile) = project.tiles.get(&tile_id) {
                if index < tile.textures.len() {
                    server_ctx.curr_tile_frame_index = index;
                    server_ctx.editing_ctx = PixelEditingContext::Tile(tile_id, index);

                    // Update the TreeIcons selection
                    if let Some(tree_layout) = ui.get_tree_layout("Tile Editor Tree") {
                        if let Some(tile_node) = tree_layout.get_node_by_id_mut(&self.tile_node) {
                            if let Some(widget) = tile_node.widgets[2].as_tree_icons() {
                                widget.set_selected_index(Some(index));
                            }
                        }
                    }

                    // Refresh the display with the new frame
                    self.update_editor_display(tile, ui, ctx, server_ctx);
                    self.sync_anchor_overlay(project, ui, ctx, server_ctx);
                }
            }
        }
    }

    /// Update just the editor display (for when frame index changes)
    fn update_editor_display(
        &mut self,
        tile: &rusterix::Tile,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
            let view_width = editor.dim().width - 16;
            let view_height = editor.dim().height - 16;

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let frame_index = server_ctx
                    .curr_tile_frame_index
                    .min(tile.textures.len().saturating_sub(1));

                if frame_index < tile.textures.len() {
                    let buffer = tile.textures[frame_index].to_rgba();
                    let icon_width = tile.textures[frame_index].width;
                    let icon_height = tile.textures[frame_index].height;

                    self.zoom = (view_width as f32 / icon_width as f32)
                        .min(view_height as f32 / icon_height as f32);

                    rgba_view.set_buffer(buffer);
                    editor.set_zoom(self.zoom);
                    editor.relayout(ctx);
                }
            }
        }
    }

    /// Update the frame icons in the tree (call after editing a texture)
    pub fn update_frame_icons(&self, tile: &rusterix::Tile, ui: &mut TheUI) {
        if let Some(tree_layout) = ui.get_tree_layout("Tile Editor Tree") {
            if let Some(tile_node) = tree_layout.get_node_by_id_mut(&self.tile_node) {
                if let Some(widget) = tile_node.widgets[2].as_tree_icons() {
                    // Update all frame icons
                    for (index, texture) in tile.textures.iter().enumerate() {
                        widget.set_icon(index, texture.to_rgba());
                    }
                }
            }
        }
    }

    /// Add an undo atom to the appropriate undo stack (keyed by context)
    pub fn add_undo(&mut self, atom: TileEditorUndoAtom, ctx: &mut TheContext) {
        let key = match &atom {
            TileEditorUndoAtom::TileEdit(tile_id, _, _) => Some(*tile_id),
            TileEditorUndoAtom::TextureEdit(editing_ctx, _, _) => match editing_ctx {
                PixelEditingContext::Tile(tile_id, _) => Some(*tile_id),
                PixelEditingContext::AvatarFrame(avatar_id, _, _, _) => Some(*avatar_id),
                PixelEditingContext::None => None,
            },
            TileEditorUndoAtom::AvatarAnchorEdit(editing_ctx, _, _, _, _) => match editing_ctx {
                PixelEditingContext::Tile(tile_id, _) => Some(*tile_id),
                PixelEditingContext::AvatarFrame(avatar_id, _, _, _) => Some(*avatar_id),
                PixelEditingContext::None => None,
            },
        };
        if let Some(key) = key {
            let undo = self
                .tile_undos
                .entry(key)
                .or_insert_with(TileEditorUndo::new);
            undo.add(atom);
            undo.truncate_to_limit(self.max_undo);
            self.set_undo_state_to_ui(ctx);
        }
    }

    fn sync_paste_preview(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout")
            && let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view()
        {
            if let (Some(texture), Some(pos)) =
                (&self.paste_preview_texture, self.paste_preview_pos)
            {
                let top_left = Self::paste_top_left_from_center(pos, texture);
                rgba_view.set_paste_preview(Some((texture.to_rgba(), top_left)));
            } else {
                rgba_view.set_paste_preview(None);
            }
            editor.relayout(ctx);
        }
    }

    fn clear_paste_preview(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.paste_preview_texture = None;
        self.paste_preview_pos = None;
        self.sync_paste_preview(ui, ctx);
    }

    fn apply_paste_at_preview(
        &mut self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let Some(pasted) = self.paste_preview_texture.clone() else {
            return false;
        };
        let Some(anchor) = self.paste_preview_pos else {
            return false;
        };
        let paste_top_left = Self::paste_top_left_from_center(anchor, &pasted);

        let selection = if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                rgba_view.selection()
            } else {
                FxHashSet::default()
            }
        } else {
            FxHashSet::default()
        };

        let editing_ctx = server_ctx.editing_ctx;
        let before = project.get_editing_texture(&editing_ctx).cloned();
        if let Some(texture) = project.get_editing_texture_mut(&editing_ctx) {
            let before = if let Some(before) = before {
                before
            } else {
                return false;
            };
            let mut changed = false;

            if selection.is_empty() {
                for sy in 0..pasted.height {
                    for sx in 0..pasted.width {
                        let tx = paste_top_left.x + sx as i32;
                        let ty = paste_top_left.y + sy as i32;
                        if tx >= 0
                            && ty >= 0
                            && (tx as usize) < texture.width
                            && (ty as usize) < texture.height
                        {
                            let src_i = (sy * pasted.width + sx) * 4;
                            let dst_i = ((ty as usize) * texture.width + (tx as usize)) * 4;
                            texture.data[dst_i..dst_i + 4]
                                .copy_from_slice(&pasted.data[src_i..src_i + 4]);
                            changed = true;
                        }
                    }
                }
            } else {
                for sy in 0..pasted.height {
                    for sx in 0..pasted.width {
                        let tx = paste_top_left.x + sx as i32;
                        let ty = paste_top_left.y + sy as i32;
                        if tx >= 0
                            && ty >= 0
                            && (tx as usize) < texture.width
                            && (ty as usize) < texture.height
                            && selection.contains(&(tx, ty))
                        {
                            let src_i = (sy * pasted.width + sx) * 4;
                            let dst_i = ((ty as usize) * texture.width + (tx as usize)) * 4;
                            texture.data[dst_i..dst_i + 4]
                                .copy_from_slice(&pasted.data[src_i..src_i + 4]);
                            changed = true;
                        }
                    }
                }
            }

            if !changed {
                return false;
            }

            texture.generate_normals(true);
            let after = texture.clone();
            let atom = TileEditorUndoAtom::TextureEdit(editing_ctx, before, after);
            self.add_undo(atom, ctx);

            match editing_ctx {
                PixelEditingContext::Tile(tile_id, _) => {
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Tile Updated"),
                        TheValue::Id(tile_id),
                    ));
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Tilepicker"),
                        TheValue::Empty,
                    ));
                }
                PixelEditingContext::AvatarFrame(..) => {
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Editing Texture Updated"),
                        TheValue::Empty,
                    ));
                }
                PixelEditingContext::None => {}
            }
            return true;
        }
        false
    }

    fn clear_current_selection(
        &mut self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if server_ctx.editing_ctx == PixelEditingContext::None {
            return false;
        }

        let selection = if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                rgba_view.selection()
            } else {
                FxHashSet::default()
            }
        } else {
            FxHashSet::default()
        };

        if selection.is_empty() {
            return false;
        }

        let editing_ctx = server_ctx.editing_ctx;
        let before = project.get_editing_texture(&editing_ctx).cloned();
        if let Some(texture) = project.get_editing_texture_mut(&editing_ctx) {
            let before = if let Some(before) = before {
                before
            } else {
                return false;
            };
            let mut changed = false;
            for (x, y) in selection {
                if x >= 0 && y >= 0 && (x as usize) < texture.width && (y as usize) < texture.height
                {
                    let i = ((y as usize) * texture.width + (x as usize)) * 4;
                    if texture.data[i..i + 4] != [0, 0, 0, 0] {
                        texture.data[i..i + 4].copy_from_slice(&[0, 0, 0, 0]);
                        changed = true;
                    }
                }
            }
            if changed {
                texture.generate_normals(true);
                let after = texture.clone();
                let atom = TileEditorUndoAtom::TextureEdit(editing_ctx, before, after);
                self.add_undo(atom, ctx);

                match editing_ctx {
                    PixelEditingContext::Tile(tile_id, _) => {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Tile Updated"),
                            TheValue::Id(tile_id),
                        ));
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Tilepicker"),
                            TheValue::Empty,
                        ));
                    }
                    PixelEditingContext::AvatarFrame(..) => {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Editing Texture Updated"),
                            TheValue::Empty,
                        ));
                    }
                    PixelEditingContext::None => {}
                }

                return true;
            }
        }
        false
    }

    #[inline]
    fn paste_top_left_from_center(anchor: Vec2<i32>, pasted: &rusterix::Texture) -> Vec2<i32> {
        Vec2::new(
            anchor.x - pasted.width as i32 / 2,
            anchor.y - pasted.height as i32 / 2,
        )
    }

    /// Set the tile for the editor.
    pub fn set_tile(
        &mut self,
        tile: &rusterix::Tile,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        update_only: bool,
    ) {
        // Switch to this tile's undo stack
        if !update_only {
            self.switch_to_tile(tile, ctx, server_ctx);

            if let Some(tree_layout) = ui.get_tree_layout("Tile Editor Tree") {
                if let Some(tile_node) = tree_layout.get_node_by_id_mut(&self.tile_node) {
                    // Set the tile size
                    if let Some(widget) = tile_node.widgets[0].as_tree_item() {
                        if let Some(embedded) = widget.embedded_widget_mut() {
                            if !tile.is_empty() {
                                embedded.set_value(TheValue::Int(tile.textures[0].width as i32));
                            }
                        }
                    }
                    // Set the frames editor
                    if let Some(widget) = tile_node.widgets[1].as_tree_item() {
                        if let Some(embedded) = widget.embedded_widget_mut() {
                            if !tile.is_empty() {
                                embedded.set_value(TheValue::Int(tile.textures.len() as i32));
                            }
                        }
                    }
                    // Set the frames editor
                    if let Some(widget) = tile_node.widgets[2].as_tree_icons() {
                        widget.set_icon_count(tile.textures.len());
                        for (index, texture) in tile.textures.iter().enumerate() {
                            widget.set_icon(index, texture.to_rgba());
                        }
                    }
                }
            }
        }

        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
            let view_width = editor.dim().width - 16;
            let view_height = editor.dim().height - 16;

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                // Use current frame index, ensure it's valid
                let frame_index = server_ctx
                    .curr_tile_frame_index
                    .min(tile.textures.len().saturating_sub(1));

                if frame_index < tile.textures.len() {
                    let buffer = tile.textures[frame_index].to_rgba();

                    if !update_only {
                        rgba_view.set_grid(Some(1));
                        rgba_view.set_dont_show_grid(!self.show_grid);

                        let icon_width = tile.textures[frame_index].width;
                        let icon_height = tile.textures[frame_index].height;

                        self.zoom = (view_width as f32 / icon_width as f32)
                            .min(view_height as f32 / icon_height as f32);
                    }
                    rgba_view.set_buffer(buffer);
                }
            }
            if !update_only {
                editor.set_zoom(self.zoom);
                editor.relayout(ctx);
            }
        }
    }

    /// Called whenever the editing context changes (activate, tile picked, avatar frame selected).
    /// Use this to adjust UI elements based on the current PixelEditingContext.
    pub fn editing_context_changed(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        if self.paste_preview_texture.is_some() {
            self.clear_paste_preview(ui, ctx);
        }
        match server_ctx.editing_ctx {
            PixelEditingContext::Tile(tile_id, _) => {
                server_ctx.avatar_anchor_slot = AvatarAnchorEditSlot::None;
                if let Some(tile) = project.tiles.get(&tile_id) {
                    self.set_tile(tile, ui, ctx, server_ctx, false);
                    if let Some(stack) = ui.get_stack_layout("Pixel Editor Stack Layout") {
                        stack.set_index(0);
                    }
                }
            }
            PixelEditingContext::AvatarFrame(..) => {
                self.set_undo_key_from_context(&server_ctx.editing_ctx);
                self.refresh_from_editing_context(project, ui, ctx, server_ctx);
                if let Some(stack) = ui.get_stack_layout("Pixel Editor Stack Layout") {
                    stack.set_index(1);
                }
            }
            PixelEditingContext::None => {
                server_ctx.avatar_anchor_slot = AvatarAnchorEditSlot::None;
                if let Some(tile_id) = server_ctx.curr_tile_id {
                    if let Some(tile) = project.tiles.get(&tile_id) {
                        self.set_tile(tile, ui, ctx, server_ctx, false);
                    }
                }
            }
        }
        self.sync_anchor_overlay(project, ui, ctx, server_ctx);
    }

    /// Set the undo key based on the current editing context.
    pub fn set_undo_key_from_context(&mut self, editing_ctx: &PixelEditingContext) {
        self.current_undo_key = match editing_ctx {
            PixelEditingContext::None => None,
            PixelEditingContext::Tile(tile_id, _) => Some(*tile_id),
            PixelEditingContext::AvatarFrame(avatar_id, _, _, _) => Some(*avatar_id),
        };
    }

    /// Refresh the editor display from the current editing context.
    pub fn refresh_from_editing_context(
        &mut self,
        project: &Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(texture) = project.get_editing_texture(&server_ctx.editing_ctx) {
            self.set_editing_texture(texture, ui, ctx);
        }
        self.sync_anchor_overlay(project, ui, ctx, server_ctx);
    }

    /// Display the given texture in the editor.
    pub fn set_editing_texture(
        &mut self,
        texture: &rusterix::Texture,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
            let view_width = editor.dim().width - 16;
            let view_height = editor.dim().height - 16;

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let buffer = texture.to_rgba();
                let icon_width = texture.width;
                let icon_height = texture.height;

                self.zoom = (view_width as f32 / icon_width as f32)
                    .min(view_height as f32 / icon_height as f32);

                rgba_view.set_grid(Some(1));
                rgba_view.set_dont_show_grid(!self.show_grid);
                rgba_view.set_buffer(buffer);
                editor.set_zoom(self.zoom);
                editor.relayout(ctx);
            }
        }
    }
}
