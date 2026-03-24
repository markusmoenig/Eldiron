use crate::docks::tiles_editor_undo::*;
use crate::editor::{TOOLLIST, UNDOMANAGER};
use crate::prelude::*;
use serde::{Deserialize, Serialize};

const TILE_NODE_CANVAS_VIEW: &str = "Tile Node Skeleton Graph View";
const TILE_NODE_SETTINGS_LAYOUT: &str = "Tile Node Settings";

#[derive(Clone, Copy, PartialEq)]
enum TilesEditorMode {
    Pixel,
    NodeSkeleton,
}

#[derive(Serialize, Deserialize, Default)]
struct TileNodeGraphState {
    #[serde(default = "default_tile_node_nodes")]
    nodes: Vec<TileNodeState>,
    #[serde(default)]
    connections: Vec<(u16, u8, u16, u8)>,
    #[serde(default)]
    offset: (i32, i32),
    #[serde(default)]
    selected_node: Option<usize>,
    #[serde(default)]
    preview_mode: u8,
}

#[derive(Serialize, Deserialize, Clone)]
struct TileNodeState {
    kind: TileNodeKind,
    position: (i32, i32),
    #[serde(default)]
    bypass: bool,
    #[serde(default)]
    mute: bool,
    #[serde(default)]
    solo: bool,
}

#[derive(Serialize, Deserialize, Clone)]
enum TileNodeKind {
    OutputRoot,
    GroupUV,
    Scalar {
        value: f32,
    },
    Color {
        color: TheColor,
    },
    PaletteColor {
        index: u16,
    },
    Mix {
        factor: f32,
    },
    Checker {
        scale: u16,
    },
    Gradient {
        mode: u8,
    },
    Noise {
        scale: f32,
        seed: u32,
        wrap: bool,
    },
    Voronoi {
        scale: f32,
        seed: u32,
        jitter: f32,
    },
    Offset {
        x: f32,
        y: f32,
    },
    Scale {
        x: f32,
        y: f32,
    },
    Repeat {
        repeat_x: f32,
        repeat_y: f32,
    },
    Rotate {
        angle: f32,
    },
    Brick {
        columns: u16,
        rows: u16,
        offset: f32,
    },
    Multiply,
    MakeMaterial,
    Material {
        roughness: f32,
        metallic: f32,
        opacity: f32,
        emissive: f32,
    },
    MaterialMix {
        factor: f32,
    },
    MaskBlend {
        factor: f32,
    },
    Levels {
        low: f32,
        high: f32,
    },
    Threshold {
        cutoff: f32,
    },
    Warp {
        amount: f32,
    },
    Invert,
}

#[derive(Clone, Copy)]
struct TileEvalContext {
    cell_x: u16,
    cell_y: u16,
    group_width: u16,
    group_height: u16,
    u: f32,
    v: f32,
}

impl TileEvalContext {
    fn group_u(&self) -> f32 {
        ((self.cell_x as f32) + self.u) / (self.group_width.max(1) as f32)
    }

    fn group_v(&self) -> f32 {
        ((self.cell_y as f32) + self.v) / (self.group_height.max(1) as f32)
    }

    fn with_group_uv(&self, group_u: f32, group_v: f32) -> Self {
        let width = self.group_width.max(1) as f32;
        let height = self.group_height.max(1) as f32;
        let gx = group_u.clamp(0.0, 0.999_999) * width;
        let gy = group_v.clamp(0.0, 0.999_999) * height;
        let cell_x = gx.floor() as u16;
        let cell_y = gy.floor() as u16;
        Self {
            cell_x,
            cell_y,
            group_width: self.group_width,
            group_height: self.group_height,
            u: gx.fract(),
            v: gy.fract(),
        }
    }
}

fn default_tile_node_nodes() -> Vec<TileNodeState> {
    vec![TileNodeState {
        kind: TileNodeKind::OutputRoot,
        position: (420, 40),
        bypass: false,
        mute: false,
        solo: false,
    }]
}

impl Default for TileNodeState {
    fn default() -> Self {
        Self {
            kind: TileNodeKind::OutputRoot,
            position: (420, 40),
            bypass: false,
            mute: false,
            solo: false,
        }
    }
}

impl Default for TileNodeKind {
    fn default() -> Self {
        Self::OutputRoot
    }
}

impl TileNodeGraphState {
    fn ensure_root(&mut self) {
        if self.nodes.is_empty() {
            self.nodes = default_tile_node_nodes();
        } else if !matches!(
            self.nodes.first().map(|n| &n.kind),
            Some(TileNodeKind::OutputRoot)
        ) {
            self.nodes.insert(
                0,
                TileNodeState {
                    kind: TileNodeKind::OutputRoot,
                    position: (420, 40),
                    bypass: false,
                    mute: false,
                    solo: false,
                },
            );
        }
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

        let mut generator_button = TheTraybarButton::new(TheId::named("Tile Node Generators"));
        generator_button.set_text("Generators".to_string());
        generator_button.set_custom_color(Some(TheColor::from_u8_array_3([86, 180, 120])));
        generator_button.set_status_text("Add generator nodes.");
        generator_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new(
                    "Group UV".to_string(),
                    TheId::named("Tile Node Add Group UV"),
                ),
                TheContextMenuItem::new("Scalar".to_string(), TheId::named("Tile Node Add Scalar")),
                TheContextMenuItem::new("Color".to_string(), TheId::named("Tile Node Add Color")),
                TheContextMenuItem::new(
                    "Palette Index".to_string(),
                    TheId::named("Tile Node Add Palette Color"),
                ),
                TheContextMenuItem::new(
                    "Material".to_string(),
                    TheId::named("Tile Node Add Material"),
                ),
                TheContextMenuItem::new(
                    "Make Material".to_string(),
                    TheId::named("Tile Node Add Make Material"),
                ),
                TheContextMenuItem::new("Noise".to_string(), TheId::named("Tile Node Add Noise")),
                TheContextMenuItem::new(
                    "Voronoi".to_string(),
                    TheId::named("Tile Node Add Voronoi"),
                ),
            ],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(generator_button));

        let mut pattern_button = TheTraybarButton::new(TheId::named("Tile Node Pattern"));
        pattern_button.set_text("Pattern".to_string());
        pattern_button.set_custom_color(Some(TheColor::from_u8_array_3([87, 150, 224])));
        pattern_button.set_status_text("Add pattern nodes.");
        pattern_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new(
                    "Gradient".to_string(),
                    TheId::named("Tile Node Add Gradient"),
                ),
                TheContextMenuItem::new(
                    "Checker".to_string(),
                    TheId::named("Tile Node Add Checker"),
                ),
                TheContextMenuItem::new("Brick".to_string(), TheId::named("Tile Node Add Brick")),
                TheContextMenuItem::new("Offset".to_string(), TheId::named("Tile Node Add Offset")),
                TheContextMenuItem::new("Scale".to_string(), TheId::named("Tile Node Add Scale")),
                TheContextMenuItem::new("Rotate".to_string(), TheId::named("Tile Node Add Rotate")),
                TheContextMenuItem::new("Repeat".to_string(), TheId::named("Tile Node Add Repeat")),
            ],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(pattern_button));

        let mut compose_button = TheTraybarButton::new(TheId::named("Tile Node Compose"));
        compose_button.set_text("Compose".to_string());
        compose_button.set_custom_color(Some(TheColor::from_u8_array_3([214, 134, 96])));
        compose_button.set_status_text("Add compositing nodes.");
        compose_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Mix".to_string(), TheId::named("Tile Node Add Mix")),
                TheContextMenuItem::new(
                    "Multiply".to_string(),
                    TheId::named("Tile Node Add Multiply"),
                ),
                TheContextMenuItem::new(
                    "Mask Blend".to_string(),
                    TheId::named("Tile Node Add Mask Blend"),
                ),
                TheContextMenuItem::new(
                    "Material Mix".to_string(),
                    TheId::named("Tile Node Add Material Mix"),
                ),
                TheContextMenuItem::new("Levels".to_string(), TheId::named("Tile Node Add Levels")),
                TheContextMenuItem::new(
                    "Threshold".to_string(),
                    TheId::named("Tile Node Add Threshold"),
                ),
                TheContextMenuItem::new("Warp".to_string(), TheId::named("Tile Node Add Warp")),
                TheContextMenuItem::new("Invert".to_string(), TheId::named("Tile Node Add Invert")),
            ],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(compose_button));

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
                    && (id.name == "Tile Node Generators"
                        || id.name == "Tile Node Pattern"
                        || id.name == "Tile Node Compose")
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
                    } else if item_id.name == "Tile Node Add Voronoi" {
                        push_node(TileNodeKind::Voronoi {
                            scale: 0.2,
                            seed: 1,
                            jitter: 1.0,
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
                    } else if item_id.name == "Tile Node Add Brick" {
                        push_node(TileNodeKind::Brick {
                            columns: 6,
                            rows: 6,
                            offset: 0.5,
                        });
                        return true;
                    } else if item_id.name == "Tile Node Add Levels" {
                        push_node(TileNodeKind::Levels {
                            low: 0.2,
                            high: 0.8,
                        });
                        return true;
                    } else if item_id.name == "Tile Node Add Threshold" {
                        push_node(TileNodeKind::Threshold { cutoff: 0.5 });
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
                    } else if item_id.name == "Tile Node Add Material" {
                        push_node(TileNodeKind::Material {
                            roughness: 0.5,
                            metallic: 0.0,
                            opacity: 1.0,
                            emissive: 0.0,
                        });
                        return true;
                    } else if item_id.name == "Tile Node Add Make Material" {
                        push_node(TileNodeKind::MakeMaterial);
                        return true;
                    } else if item_id.name == "Tile Node Add Mix" {
                        push_node(TileNodeKind::Mix { factor: 0.5 });
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
                    }
                    return false;
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
                    } else if id.name == "tileNodeMaterialRoughness"
                        || id.name == "tileNodeMaterialMetallic"
                        || id.name == "tileNodeMaterialOpacity"
                        || id.name == "tileNodeMaterialEmissive"
                    {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Material {
                                roughness,
                                metallic,
                                opacity,
                                emissive,
                            } = &mut node.kind
                            && let Some(new_value) = value.to_f32()
                        {
                            let new_value = new_value.clamp(0.0, 1.0);
                            let target = if id.name == "tileNodeMaterialRoughness" {
                                roughness
                            } else if id.name == "tileNodeMaterialMetallic" {
                                metallic
                            } else if id.name == "tileNodeMaterialOpacity" {
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
                    } else if id.name == "tileNodeBrickColumns"
                        || id.name == "tileNodeBrickRows"
                        || id.name == "tileNodeBrickOffset"
                    {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Brick {
                                columns,
                                rows,
                                offset,
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
                    } else if id.name == "tileNodeLevelsLow" || id.name == "tileNodeLevelsHigh" {
                        let mut state = self.node_graph_state_for_group(project, group_id);
                        if let Some(index) = state.selected_node
                            && let Some(node) = state.nodes.get_mut(index)
                            && let TileNodeKind::Levels { low, high } = &mut node.kind
                            && let Some(new_value) = value.to_f32()
                        {
                            let new_value = new_value.clamp(0.0, 1.0);
                            let is_low = id.name == "tileNodeLevelsLow";
                            let changed = if is_low {
                                let changed = (*low - new_value).abs() > f32::EPSILON;
                                *low = new_value;
                                changed
                            } else {
                                let changed = (*high - new_value).abs() > f32::EPSILON;
                                *high = new_value;
                                changed
                            };
                            if changed {
                                if *low > *high {
                                    std::mem::swap(low, high);
                                }
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
                Some(TileNodeKind::Brick {
                    columns,
                    rows,
                    offset,
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
                Some(TileNodeKind::Material {
                    roughness,
                    metallic,
                    opacity,
                    emissive,
                }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeMaterialRoughness".into(),
                        "Roughness".into(),
                        "Material roughness.".into(),
                        *roughness,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeMaterialMetallic".into(),
                        "Metallic".into(),
                        "Material metallic.".into(),
                        *metallic,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeMaterialOpacity".into(),
                        "Opacity".into(),
                        "Material opacity.".into(),
                        *opacity,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeMaterialEmissive".into(),
                        "Emissive".into(),
                        "Material emissive.".into(),
                        *emissive,
                        0.0..=1.0,
                        false,
                    ));
                }
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
                Some(TileNodeKind::Levels { low, high }) => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeLevelsLow".into(),
                        "Low".into(),
                        "Lower levels bound.".into(),
                        *low,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "tileNodeLevelsHigh".into(),
                        "High".into(),
                        "Upper levels bound.".into(),
                        *high,
                        0.0..=1.0,
                        false,
                    ));
                }
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
    }

    fn mix_colors(a: TheColor, b: TheColor, factor: f32) -> TheColor {
        let t = factor.clamp(0.0, 1.0);
        let aa = a.to_u8_array();
        let bb = b.to_u8_array();
        let lerp = |x: u8, y: u8| -> u8 {
            ((x as f32 * (1.0 - t) + y as f32 * t).round()).clamp(0.0, 255.0) as u8
        };
        TheColor::from_u8_array([
            lerp(aa[0], bb[0]),
            lerp(aa[1], bb[1]),
            lerp(aa[2], bb[2]),
            lerp(aa[3], bb[3]),
        ])
    }

    fn multiply_colors(a: TheColor, b: TheColor) -> TheColor {
        let aa = a.to_u8_array();
        let bb = b.to_u8_array();
        let mul = |x: u8, y: u8| -> u8 { ((x as u16 * y as u16) / 255) as u8 };
        TheColor::from_u8_array([
            mul(aa[0], bb[0]),
            mul(aa[1], bb[1]),
            mul(aa[2], bb[2]),
            mul(aa[3], bb[3]),
        ])
    }

    fn color_to_mask(color: TheColor) -> f32 {
        let rgba = color.to_u8_array();
        (0.2126 * rgba[0] as f32 + 0.7152 * rgba[1] as f32 + 0.0722 * rgba[2] as f32) / 255.0
    }

    fn evaluate_node_scalar(
        &self,
        project: &Project,
        state: &TileNodeGraphState,
        node_index: usize,
        eval: TileEvalContext,
        visiting: &mut FxHashSet<usize>,
    ) -> Option<f32> {
        state
            .nodes
            .get(node_index)
            .and_then(|node| match &node.kind {
                TileNodeKind::Scalar { value } => Some(*value),
                _ => self
                    .evaluate_node_color(project, state, node_index, eval, visiting)
                    .map(Self::color_to_mask),
            })
    }

    fn evaluate_output_material(
        &self,
        project: &Project,
        state: &TileNodeGraphState,
        eval: TileEvalContext,
    ) -> (f32, f32, f32, f32) {
        if let Some(material_src) = self.input_connection_source(state, 0, 1) {
            if let Some(material) = self.evaluate_node_material(
                project,
                state,
                material_src,
                eval,
                &mut FxHashSet::default(),
            ) {
                return material;
            }
        }

        (0.5, 0.0, 1.0, 0.0)
    }

    fn solo_node_index(&self, state: &TileNodeGraphState) -> Option<usize> {
        state.nodes.iter().position(|n| n.solo)
    }

    fn evaluate_node_material(
        &self,
        project: &Project,
        state: &TileNodeGraphState,
        node_index: usize,
        eval: TileEvalContext,
        visiting: &mut FxHashSet<usize>,
    ) -> Option<(f32, f32, f32, f32)> {
        if !visiting.insert(node_index) {
            return None;
        }
        let result = state.nodes.get(node_index).and_then(|node| {
            if node.bypass
                && !matches!(node.kind, TileNodeKind::OutputRoot)
                && let Some(src) = self.input_connection_source(state, node_index, 0)
            {
                return self.evaluate_node_material(project, state, src, eval, visiting);
            }
            match &node.kind {
                TileNodeKind::OutputRoot => {
                    if let Some(solo) = self.solo_node_index(state)
                        && solo != node_index
                    {
                        self.evaluate_node_material(project, state, solo, eval, visiting)
                    } else {
                        self.input_connection_source(state, node_index, 1)
                            .and_then(|src| {
                                self.evaluate_node_material(project, state, src, eval, visiting)
                            })
                    }
                }
                TileNodeKind::Material {
                    roughness,
                    metallic,
                    opacity,
                    emissive,
                } => Some((*roughness, *metallic, *opacity, *emissive)),
                TileNodeKind::MakeMaterial => {
                    let mut channel = |input_terminal: u8, default: f32| -> f32 {
                        self.input_connection_source(state, node_index, input_terminal)
                            .and_then(|src| {
                                self.evaluate_node_scalar(project, state, src, eval, visiting)
                            })
                            .unwrap_or(default)
                            .clamp(0.0, 1.0)
                    };
                    Some((
                        channel(0, 0.5),
                        channel(1, 0.0),
                        channel(2, 1.0),
                        channel(3, 0.0),
                    ))
                }
                TileNodeKind::MaterialMix { factor } => {
                    let a = self
                        .input_connection_source(state, node_index, 0)
                        .and_then(|src| {
                            self.evaluate_node_material(project, state, src, eval, visiting)
                        });
                    let b = self
                        .input_connection_source(state, node_index, 1)
                        .and_then(|src| {
                            self.evaluate_node_material(project, state, src, eval, visiting)
                        });
                    let mask = self
                        .input_connection_source(state, node_index, 2)
                        .and_then(|src| {
                            self.evaluate_node_scalar(project, state, src, eval, visiting)
                        })
                        .unwrap_or(0.0)
                        .clamp(0.0, 1.0)
                        * factor.clamp(0.0, 1.0);
                    match (a, b) {
                        (Some(a), Some(b)) => Some((
                            a.0 * (1.0 - mask) + b.0 * mask,
                            a.1 * (1.0 - mask) + b.1 * mask,
                            a.2 * (1.0 - mask) + b.2 * mask,
                            a.3 * (1.0 - mask) + b.3 * mask,
                        )),
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    }
                }
                _ => {
                    if node.mute {
                        return Some((0.5, 0.0, 0.0, 0.0));
                    }
                    let roughness = self
                        .evaluate_node_scalar(project, state, node_index, eval, visiting)
                        .unwrap_or(0.5)
                        .clamp(0.0, 1.0);
                    Some((roughness, 0.0, 1.0, 0.0))
                }
            }
        });
        visiting.remove(&node_index);
        result
    }

    fn hash2(x: i32, y: i32, seed: u32) -> f32 {
        let mut n = x as u32;
        n = n
            .wrapping_mul(374761393)
            .wrapping_add((y as u32).wrapping_mul(668265263));
        n ^= seed.wrapping_mul(2246822519);
        n = (n ^ (n >> 13)).wrapping_mul(1274126177);
        ((n ^ (n >> 16)) & 0x00ff_ffff) as f32 / 0x00ff_ffff as f32
    }

    fn remap_unit(value: f32, low: f32, high: f32) -> f32 {
        let span = (high - low).max(0.000_1);
        ((value - low) / span).clamp(0.0, 1.0)
    }

    fn voronoi(eval: TileEvalContext, scale: f32, seed: u32, jitter: f32) -> f32 {
        let scale = (scale.clamp(0.01, 1.0) * 16.0).max(1.0);
        let x = eval.group_u() * scale;
        let y = eval.group_v() * scale;
        let cell_x = x.floor() as i32;
        let cell_y = y.floor() as i32;
        let frac_x = x.fract();
        let frac_y = y.fract();
        let jitter = jitter.clamp(0.0, 1.0);
        let mut min_dist = f32::MAX;

        for oy in -1..=1 {
            for ox in -1..=1 {
                let sx = cell_x + ox;
                let sy = cell_y + oy;
                let px = 0.5 + (Self::hash2(sx, sy, seed) - 0.5) * jitter;
                let py = 0.5 + (Self::hash2(sx, sy, seed ^ 0x9e37_79b9) - 0.5) * jitter;
                let dx = ox as f32 + px - frac_x;
                let dy = oy as f32 + py - frac_y;
                min_dist = min_dist.min((dx * dx + dy * dy).sqrt());
            }
        }

        (1.0 - (min_dist / 1.4142)).clamp(0.0, 1.0)
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
        for y in 0..height {
            for x in 0..width {
                let ctx = TileEvalContext {
                    cell_x: 0,
                    cell_y: 0,
                    group_width: 1,
                    group_height: 1,
                    u: x as f32 / (width.max(1) - 1) as f32,
                    v: y as f32 / (height.max(1) - 1) as f32,
                };
                let color = if state.preview_mode == 1 {
                    let (r, m, o, e) = self
                        .evaluate_node_material(
                            project,
                            state,
                            node_index,
                            ctx,
                            &mut FxHashSet::default(),
                        )
                        .unwrap_or((0.0, 0.0, 0.0, 0.0));
                    TheColor::from_u8_array([
                        (r * 255.0).round() as u8,
                        (m * 255.0).round() as u8,
                        (o * 255.0).round() as u8,
                        (e * 255.0).round() as u8,
                    ])
                } else {
                    self.evaluate_node_color(
                        project,
                        state,
                        node_index,
                        ctx,
                        &mut FxHashSet::default(),
                    )
                    .unwrap_or_else(|| TheColor::from_u8_array([0, 0, 0, 0]))
                };
                preview.set_pixel(x, y, &color.to_u8_array());
            }
        }
        preview
    }

    fn input_connection_source(
        &self,
        state: &TileNodeGraphState,
        node_index: usize,
        input_terminal: u8,
    ) -> Option<usize> {
        state
            .connections
            .iter()
            .find(|(_, _, dest_node, dest_terminal)| {
                *dest_node as usize == node_index && *dest_terminal == input_terminal
            })
            .map(|(src_node, _, _, _)| *src_node as usize)
    }

    fn evaluate_node_color(
        &self,
        project: &Project,
        state: &TileNodeGraphState,
        node_index: usize,
        eval: TileEvalContext,
        visiting: &mut FxHashSet<usize>,
    ) -> Option<TheColor> {
        if !visiting.insert(node_index) {
            return None;
        }
        let result = state.nodes.get(node_index).and_then(|node| {
            if node.mute {
                return Some(TheColor::from_u8_array([0, 0, 0, 0]));
            }
            if node.bypass
                && !matches!(node.kind, TileNodeKind::OutputRoot | TileNodeKind::GroupUV)
                && let Some(src) = self.input_connection_source(state, node_index, 0)
            {
                return self.evaluate_node_color(project, state, src, eval, visiting);
            }
            match &node.kind {
                TileNodeKind::OutputRoot => {
                    if let Some(solo) = self.solo_node_index(state)
                        && solo != node_index
                    {
                        self.evaluate_node_color(project, state, solo, eval, visiting)
                    } else {
                        self.input_connection_source(state, node_index, 0)
                            .and_then(|src| {
                                self.evaluate_node_color(project, state, src, eval, visiting)
                            })
                    }
                }
                TileNodeKind::GroupUV => Some(TheColor::from_u8_array([
                    (eval.group_u().clamp(0.0, 1.0) * 255.0).round() as u8,
                    (eval.group_v().clamp(0.0, 1.0) * 255.0).round() as u8,
                    0,
                    255,
                ])),
                TileNodeKind::Scalar { value } => {
                    let v = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
                    Some(TheColor::from_u8_array([v, v, v, 255]))
                }
                TileNodeKind::Gradient { mode } => {
                    let gu = eval.group_u().clamp(0.0, 1.0);
                    let gv = eval.group_v().clamp(0.0, 1.0);
                    let value = match mode {
                        0 => gu,
                        1 => gv,
                        _ => {
                            let dx = gu - 0.5;
                            let dy = gv - 0.5;
                            (1.0 - (dx * dx + dy * dy).sqrt() * 2.0).clamp(0.0, 1.0)
                        }
                    };
                    let v = (value * 255.0).round() as u8;
                    Some(TheColor::from_u8_array([v, v, v, 255]))
                }
                TileNodeKind::Color { color } => Some(color.clone()),
                TileNodeKind::PaletteColor { index } => project.palette[*index as usize].clone(),
                TileNodeKind::Mix { factor } => {
                    let a = self
                        .input_connection_source(state, node_index, 0)
                        .and_then(|src| {
                            self.evaluate_node_color(project, state, src, eval, visiting)
                        });
                    let b = self
                        .input_connection_source(state, node_index, 1)
                        .and_then(|src| {
                            self.evaluate_node_color(project, state, src, eval, visiting)
                        });
                    match (a, b) {
                        (Some(a), Some(b)) => Some(Self::mix_colors(a, b, *factor)),
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    }
                }
                TileNodeKind::Checker { scale } => {
                    let s = (*scale).max(1) as f32;
                    let cx = (eval.group_u() * s).floor() as i32;
                    let cy = (eval.group_v() * s).floor() as i32;
                    let a = self
                        .input_connection_source(state, node_index, 0)
                        .and_then(|src| {
                            self.evaluate_node_color(project, state, src, eval, visiting)
                        })
                        .unwrap_or_else(|| TheColor::from_u8_array_3([255, 255, 255]));
                    let b = self
                        .input_connection_source(state, node_index, 1)
                        .and_then(|src| {
                            self.evaluate_node_color(project, state, src, eval, visiting)
                        })
                        .unwrap_or_else(|| TheColor::from_u8_array_3([0, 0, 0]));
                    if (cx + cy) & 1 == 0 { Some(a) } else { Some(b) }
                }
                TileNodeKind::Noise { scale, seed, wrap } => {
                    let s = (*scale).clamp(0.0, 1.0).max(0.0001);
                    let repeat = (s * 64.0).round().max(1.0) as i32;
                    let mut nx = (eval.group_u() * repeat as f32).floor() as i32;
                    let mut ny = (eval.group_v() * repeat as f32).floor() as i32;
                    if *wrap {
                        nx = nx.rem_euclid(repeat);
                        ny = ny.rem_euclid(repeat);
                    }
                    let v = (Self::hash2(nx, ny, *seed) * 255.0) as u8;
                    Some(TheColor::from_u8_array([v, v, v, 255]))
                }
                TileNodeKind::Voronoi {
                    scale,
                    seed,
                    jitter,
                } => {
                    let v = (Self::voronoi(eval, *scale, *seed, *jitter) * 255.0).round() as u8;
                    Some(TheColor::from_u8_array([v, v, v, 255]))
                }
                TileNodeKind::Offset { x, y } => self
                    .input_connection_source(state, node_index, 0)
                    .and_then(|src| {
                        self.evaluate_node_color(
                            project,
                            state,
                            src,
                            eval.with_group_uv(eval.group_u() + *x, eval.group_v() + *y),
                            visiting,
                        )
                    }),
                TileNodeKind::Scale { x, y } => self
                    .input_connection_source(state, node_index, 0)
                    .and_then(|src| {
                        let gu = (eval.group_u() - 0.5) * x.max(0.1) + 0.5;
                        let gv = (eval.group_v() - 0.5) * y.max(0.1) + 0.5;
                        self.evaluate_node_color(
                            project,
                            state,
                            src,
                            eval.with_group_uv(gu, gv),
                            visiting,
                        )
                    }),
                TileNodeKind::Repeat { repeat_x, repeat_y } => self
                    .input_connection_source(state, node_index, 0)
                    .and_then(|src| {
                        let wrapped_u = (eval.group_u() * repeat_x.max(0.1)).fract();
                        let wrapped_v = (eval.group_v() * repeat_y.max(0.1)).fract();
                        self.evaluate_node_color(
                            project,
                            state,
                            src,
                            eval.with_group_uv(wrapped_u, wrapped_v),
                            visiting,
                        )
                    }),
                TileNodeKind::Rotate { angle } => self
                    .input_connection_source(state, node_index, 0)
                    .and_then(|src| {
                        let radians = angle.to_radians();
                        let s = radians.sin();
                        let c = radians.cos();
                        let dx = eval.group_u() - 0.5;
                        let dy = eval.group_v() - 0.5;
                        let ru = dx * c - dy * s + 0.5;
                        let rv = dx * s + dy * c + 0.5;
                        self.evaluate_node_color(
                            project,
                            state,
                            src,
                            eval.with_group_uv(ru, rv),
                            visiting,
                        )
                    }),
                TileNodeKind::Brick {
                    columns,
                    rows,
                    offset,
                } => {
                    let cols = (*columns).max(1) as f32;
                    let rows = (*rows).max(1) as f32;
                    let gu = eval.group_u() * cols;
                    let gv = eval.group_v() * rows;
                    let row = gv.floor() as i32;
                    let brick_x = gu + if row & 1 == 1 { *offset } else { 0.0 };
                    let a = self
                        .input_connection_source(state, node_index, 0)
                        .and_then(|src| {
                            self.evaluate_node_color(project, state, src, eval, visiting)
                        })
                        .unwrap_or_else(|| TheColor::from_u8_array_3([255, 255, 255]));
                    let b = self
                        .input_connection_source(state, node_index, 1)
                        .and_then(|src| {
                            self.evaluate_node_color(project, state, src, eval, visiting)
                        })
                        .unwrap_or_else(|| TheColor::from_u8_array_3([0, 0, 0]));
                    let local_x = brick_x.fract();
                    let local_y = gv.fract();
                    let mortar = local_x < 0.08 || local_y < 0.08;
                    if mortar { Some(b) } else { Some(a) }
                }
                TileNodeKind::Multiply => {
                    let a = self
                        .input_connection_source(state, node_index, 0)
                        .and_then(|src| {
                            self.evaluate_node_color(project, state, src, eval, visiting)
                        });
                    let b = self
                        .input_connection_source(state, node_index, 1)
                        .and_then(|src| {
                            self.evaluate_node_color(project, state, src, eval, visiting)
                        });
                    match (a, b) {
                        (Some(a), Some(b)) => Some(Self::multiply_colors(a, b)),
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    }
                }
                TileNodeKind::MakeMaterial => self
                    .evaluate_node_material(project, state, node_index, eval, visiting)
                    .map(|(r, m, o, e)| {
                        TheColor::from_u8_array([
                            (r * 255.0).round() as u8,
                            (m * 255.0).round() as u8,
                            (o * 255.0).round() as u8,
                            (e * 255.0).round() as u8,
                        ])
                    }),
                TileNodeKind::Material {
                    roughness,
                    metallic,
                    opacity,
                    emissive,
                } => Some(TheColor::from_u8_array([
                    (roughness.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (metallic.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (opacity.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (emissive.clamp(0.0, 1.0) * 255.0).round() as u8,
                ])),
                TileNodeKind::MaterialMix { .. } => self
                    .evaluate_node_material(project, state, node_index, eval, visiting)
                    .map(|(r, m, o, e)| {
                        TheColor::from_u8_array([
                            (r * 255.0).round() as u8,
                            (m * 255.0).round() as u8,
                            (o * 255.0).round() as u8,
                            (e * 255.0).round() as u8,
                        ])
                    }),
                TileNodeKind::MaskBlend { factor } => {
                    let a = self
                        .input_connection_source(state, node_index, 0)
                        .and_then(|src| {
                            self.evaluate_node_color(project, state, src, eval, visiting)
                        });
                    let b = self
                        .input_connection_source(state, node_index, 1)
                        .and_then(|src| {
                            self.evaluate_node_color(project, state, src, eval, visiting)
                        });
                    let mask = self
                        .input_connection_source(state, node_index, 2)
                        .and_then(|src| {
                            self.evaluate_node_color(project, state, src, eval, visiting)
                        })
                        .map(Self::color_to_mask)
                        .unwrap_or(0.0);
                    match (a, b) {
                        (Some(a), Some(b)) => Some(Self::mix_colors(a, b, mask * *factor)),
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    }
                }
                TileNodeKind::Levels { low, high } => self
                    .input_connection_source(state, node_index, 0)
                    .and_then(|src| self.evaluate_node_color(project, state, src, eval, visiting))
                    .map(|color| {
                        let v = (Self::remap_unit(Self::color_to_mask(color), *low, *high) * 255.0)
                            .round() as u8;
                        TheColor::from_u8_array([v, v, v, 255])
                    }),
                TileNodeKind::Threshold { cutoff } => self
                    .input_connection_source(state, node_index, 0)
                    .and_then(|src| self.evaluate_node_color(project, state, src, eval, visiting))
                    .map(|color| {
                        let v = if Self::color_to_mask(color) >= *cutoff {
                            255
                        } else {
                            0
                        };
                        TheColor::from_u8_array([v, v, v, 255])
                    }),
                TileNodeKind::Warp { amount } => self
                    .input_connection_source(state, node_index, 0)
                    .and_then(|src| {
                        let warp = self
                            .input_connection_source(state, node_index, 1)
                            .and_then(|warp_src| {
                                self.evaluate_node_color(project, state, warp_src, eval, visiting)
                            })
                            .map(Self::color_to_mask)
                            .unwrap_or(0.5);
                        let delta = (warp - 0.5) * amount * 0.5;
                        self.evaluate_node_color(
                            project,
                            state,
                            src,
                            eval.with_group_uv(eval.group_u() + delta, eval.group_v() + delta),
                            visiting,
                        )
                    }),
                TileNodeKind::Invert => self
                    .input_connection_source(state, node_index, 0)
                    .and_then(|src| self.evaluate_node_color(project, state, src, eval, visiting))
                    .map(|color| {
                        let rgba = color.to_u8_array();
                        TheColor::from_u8_array([
                            255 - rgba[0],
                            255 - rgba[1],
                            255 - rgba[2],
                            rgba[3],
                        ])
                    }),
            }
        });
        visiting.remove(&node_index);
        result
    }

    fn render_node_group_tiles(&self, project: &mut Project, group_id: Uuid) {
        let Some(node_group) = project.tile_node_groups.get(&group_id).cloned() else {
            return;
        };
        let width = node_group.tile_pixel_width.max(1) as usize;
        let height = node_group.tile_pixel_height.max(1) as usize;
        let state = self.node_graph_state_for_group(project, group_id);
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
                        render_members.push((member.tile_id, x, y));
                    }
                }
            }
        }

        for (tile_id, cell_x, cell_y) in render_members {
            let mut pixels = vec![0_u8; width * height * 4];
            let mut materials = vec![0_u8; width * height * 4];
            for py in 0..height {
                for px in 0..width {
                    let eval = TileEvalContext {
                        cell_x,
                        cell_y,
                        group_width: node_group.output_grid_width.max(1),
                        group_height: node_group.output_grid_height.max(1),
                        u: px as f32 / (width.max(1) - 1).max(1) as f32,
                        v: py as f32 / (height.max(1) - 1).max(1) as f32,
                    };
                    let color = self
                        .evaluate_node_color(project, &state, 0, eval, &mut FxHashSet::default())
                        .unwrap_or_else(|| TheColor::from_u8_array_3([96, 96, 96]))
                        .to_u8_array();
                    let (roughness, metallic, opacity, emissive) =
                        self.evaluate_output_material(project, &state, eval);
                    let i = (py * width + px) * 4;
                    pixels[i..i + 4].copy_from_slice(&color);
                    let packed_r = (roughness.clamp(0.0, 1.0) * 15.0).round() as u16;
                    let packed_m = (metallic.clamp(0.0, 1.0) * 15.0).round() as u16;
                    let packed_o = (opacity.clamp(0.0, 1.0) * 15.0).round() as u16;
                    let packed_e = (emissive.clamp(0.0, 1.0) * 15.0).round() as u16;
                    let mat = packed_r | (packed_m << 4) | (packed_o << 8) | (packed_e << 12);
                    let bytes = mat.to_le_bytes();
                    materials[i] = bytes[0];
                    materials[i + 1] = bytes[1];
                    materials[i + 2] = 127;
                    materials[i + 3] = 255;
                }
            }
            if let Some(tile) = project.tiles.get_mut(&tile_id) {
                if tile.textures.is_empty()
                    || tile.textures[0].width != width
                    || tile.textures[0].height != height
                {
                    tile.textures = vec![rusterix::Texture::alloc(width, height)];
                }
                tile.role = rusterix::TileRole::ManMade;
                tile.blocking = false;
                tile.scale = 1.0;
                let texture = &mut tile.textures[0];
                texture.data.copy_from_slice(&pixels);
                texture.data_ext = Some(materials);
            }
        }
    }

    fn node_graph_state_for_group(&self, project: &Project, group_id: Uuid) -> TileNodeGraphState {
        let mut state = project
            .tile_node_groups
            .get(&group_id)
            .and_then(|node_group| {
                serde_json::from_str::<TileNodeGraphState>(&node_group.graph_data).ok()
            })
            .or_else(|| {
                project
                    .tile_node_groups
                    .get(&group_id)
                    .and_then(|node_group| {
                        serde_json::from_str::<TileNodeGraphState>(&node_group.graph_data).ok()
                    })
            })
            .unwrap_or_default();
        state.ensure_root();
        state
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
            "Output".to_string(),
            TheColor::from_u8_array_3([200, 140, 90]),
        );
        canvas.categories.insert(
            "Generator".to_string(),
            TheColor::from_u8_array_3([86, 180, 120]),
        );
        canvas.categories.insert(
            "Pattern".to_string(),
            TheColor::from_u8_array_3([87, 150, 224]),
        );
        canvas.categories.insert(
            "Compose".to_string(),
            TheColor::from_u8_array_3([214, 134, 96]),
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
                    TileNodeKind::OutputRoot => {
                        canvas.nodes.push(TheNode {
                            name: "Tile Group Output".to_string(),
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "Color".to_string(),
                                    category_name: "Output".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Material".to_string(),
                                    category_name: "Output".to_string(),
                                },
                            ],
                            outputs: vec![],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: false,
                        });
                    }
                    TileNodeKind::GroupUV => {
                        canvas.nodes.push(TheNode {
                            name: "Group UV".to_string(),
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Generator".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Generator".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Pattern".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Generator".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Generator".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "A".to_string(),
                                    category_name: "Generator".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "B".to_string(),
                                    category_name: "Generator".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Generator".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "A".to_string(),
                                    category_name: "Pattern".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "B".to_string(),
                                    category_name: "Pattern".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Pattern".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Generator".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Pattern".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Offset { x, y } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Offset {:.2},{:.2}", x, y),
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "Pattern".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Pattern".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "Pattern".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Pattern".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "Pattern".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Pattern".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "Pattern".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Pattern".to_string(),
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
                    } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Brick {}x{} {:.2}", columns, rows, offset),
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "A".to_string(),
                                    category_name: "Pattern".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "B".to_string(),
                                    category_name: "Pattern".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Pattern".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Multiply => {
                        canvas.nodes.push(TheNode {
                            name: "Multiply".to_string(),
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "A".to_string(),
                                    category_name: "Generator".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "B".to_string(),
                                    category_name: "Compose".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Compose".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "Roughness".to_string(),
                                    category_name: "Compose".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Metallic".to_string(),
                                    category_name: "Compose".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Opacity".to_string(),
                                    category_name: "Compose".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Emissive".to_string(),
                                    category_name: "Compose".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Material".to_string(),
                                category_name: "Compose".to_string(),
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
                                "Material {:.2}/{:.2}/{:.2}/{:.2}",
                                roughness, metallic, opacity, emissive
                            ),
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![],
                            outputs: vec![TheNodeTerminal {
                                name: "Material".to_string(),
                                category_name: "Compose".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "A".to_string(),
                                    category_name: "Compose".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "B".to_string(),
                                    category_name: "Compose".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Mask".to_string(),
                                    category_name: "Compose".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Material".to_string(),
                                category_name: "Compose".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "A".to_string(),
                                    category_name: "Generator".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "B".to_string(),
                                    category_name: "Generator".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Mask".to_string(),
                                    category_name: "Compose".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Compose".to_string(),
                            }],
                            preview,
                            supports_preview: true,
                            preview_is_open: true,
                            can_be_deleted: true,
                        });
                    }
                    TileNodeKind::Levels { low, high } => {
                        canvas.nodes.push(TheNode {
                            name: format!("Levels {:.2}-{:.2}", low, high),
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "Compose".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Compose".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "Compose".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Compose".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![
                                TheNodeTerminal {
                                    name: "In".to_string(),
                                    category_name: "Compose".to_string(),
                                },
                                TheNodeTerminal {
                                    name: "Warp".to_string(),
                                    category_name: "Compose".to_string(),
                                },
                            ],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Compose".to_string(),
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
                            position: Vec2::new(node.position.0, node.position.1),
                            inputs: vec![TheNodeTerminal {
                                name: "In".to_string(),
                                category_name: "Compose".to_string(),
                            }],
                            outputs: vec![TheNodeTerminal {
                                name: "Color".to_string(),
                                category_name: "Compose".to_string(),
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
