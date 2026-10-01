//! Construction branches use the shared graph editor and Node List workflow.
use crate::editor::{RUSTERIX, UNDOMANAGER};
use crate::prelude::*;
use shared::construction_graph::{self, ConstructionPatternAsset, ConstructionPatternKind};
use std::collections::{HashMap, HashSet};
use theframework::thegraph::*;

const VIEW: &str = "Construction Graph View";
const BRANCH_LIST: &str = "Construction Branches List";
const CATALOG: &str = "Node Catalog List";
const ASSIGN_BRANCH: &str = "Construction Branch Assign";
const REMOVE_BRANCH: &str = "Construction Branch Remove";
const TIDY_GRAPH: &str = "Construction Graph Tidy";

fn branch_list_canvas() -> TheCanvas {
    let mut canvas = TheCanvas::new();
    let mut list = TheListLayout::new(TheId::named(BRANCH_LIST));
    list.limiter_mut().set_min_width(190);
    list.limiter_mut().set_max_width(190);
    canvas.set_layout(list);
    let mut top = TheCanvas::new();
    top.set_widget(TheTraybar::new(TheId::empty()));
    let mut layout = TheHLayout::new(TheId::empty());
    layout.set_margin(Vec4::new(10, 1, 5, 1));
    layout.set_background_color(None);
    let mut heading = TheText::new(TheId::named("Construction Branch Heading"));
    heading.set_text(fl!("node_branches"));
    heading.set_status_text(&fl!("construction_branch_help"));
    layout.add_widget(Box::new(heading));
    top.set_layout(layout);
    canvas.set_top(top);
    canvas.top_is_expanding = false;
    canvas
}

pub fn node_available(key: &str) -> bool {
    matches!(
        key,
        "wall"
            | "surface"
            | "pattern"
            | "pattern_ref"
            | "surface_pattern_ref"
            | "pattern_output"
            | "pattern_scale"
            | "masonry"
            | "wall_finish"
            | "wall_output"
            | "output"
            | "inset"
            | "subdivide"
            | "color"
            | "noise"
            | "gradient"
            | "tile"
    )
}

pub fn sync_node_list(ui: &mut TheUI, ctx: &mut TheContext, kind: Option<ConstructionPatternKind>) {
    let Some(list) = ui.get_list_layout(CATALOG) else {
        return;
    };
    list.clear();
    for (id, label, color, available) in [
        (
            "wall",
            fl!("construction_node_wall"),
            [47, 124, 136, 255],
            true,
        ),
        (
            "surface",
            fl!("construction_node_surface"),
            [47, 124, 136, 255],
            true,
        ),
        (
            "pattern",
            fl!("construction_node_pattern"),
            [91, 86, 151, 255],
            true,
        ),
        (
            "pattern_ref",
            fl!("construction_node_pattern_ref"),
            [52, 112, 158, 255],
            kind == Some(ConstructionPatternKind::Wall),
        ),
        (
            "surface_pattern_ref",
            fl!("construction_node_pattern_ref"),
            [52, 112, 158, 255],
            kind == Some(ConstructionPatternKind::Surface),
        ),
        (
            "wall_output",
            fl!("construction_node_wall_output"),
            [45, 130, 99, 255],
            kind == Some(ConstructionPatternKind::Wall),
        ),
        (
            "masonry",
            fl!("construction_node_masonry"),
            [52, 112, 158, 255],
            kind == Some(ConstructionPatternKind::Pattern),
        ),
        (
            "wall_finish",
            fl!("construction_node_wall_finish"),
            [52, 112, 158, 255],
            kind == Some(ConstructionPatternKind::Pattern),
        ),
        (
            "pattern_output",
            fl!("construction_node_pattern_output"),
            [45, 130, 99, 255],
            kind == Some(ConstructionPatternKind::Pattern),
        ),
        (
            "pattern_scale",
            fl!("construction_node_pattern_scale"),
            [52, 112, 158, 255],
            kind == Some(ConstructionPatternKind::Pattern),
        ),
        (
            "inset",
            fl!("construction_node_inset"),
            [52, 112, 158, 255],
            kind == Some(ConstructionPatternKind::Surface),
        ),
        (
            "subdivide",
            fl!("construction_node_subdivide"),
            [52, 112, 158, 255],
            kind == Some(ConstructionPatternKind::Surface),
        ),
        (
            "output",
            fl!("construction_node_output"),
            [45, 130, 99, 255],
            kind == Some(ConstructionPatternKind::Surface),
        ),
        (
            "noise",
            fl!("construction_node_noise"),
            [132, 95, 152, 255],
            matches!(
                kind,
                Some(ConstructionPatternKind::Surface | ConstructionPatternKind::Pattern)
            ),
        ),
        (
            "gradient",
            fl!("construction_node_gradient"),
            [132, 95, 152, 255],
            matches!(
                kind,
                Some(ConstructionPatternKind::Surface | ConstructionPatternKind::Pattern)
            ),
        ),
        (
            "color",
            fl!("construction_node_color"),
            [132, 95, 152, 255],
            matches!(
                kind,
                Some(ConstructionPatternKind::Surface | ConstructionPatternKind::Pattern)
            ),
        ),
        (
            "tile",
            fl!("construction_node_tile"),
            [132, 95, 152, 255],
            matches!(
                kind,
                Some(ConstructionPatternKind::Surface | ConstructionPatternKind::Pattern)
            ),
        ),
    ] {
        if !available {
            continue;
        }
        let mut item = TheListItem::new(TheId::named(&format!("Node Catalog/{id}")));
        item.set_text(label.clone());
        let help = match id {
            "wall" => fl!("construction_help_wall"),
            "surface" => fl!("construction_help_surface"),
            "pattern" => fl!("construction_help_pattern"),
            "noise" => fl!("construction_help_noise"),
            "gradient" => fl!("construction_help_gradient"),
            "pattern_ref" | "surface_pattern_ref" => fl!("construction_help_pattern_ref"),
            _ => label.clone(),
        };
        item.set_status_text(&help);
        item.set_background_color(TheColor::from_u8_array([
            color[0] / 2,
            color[1] / 2,
            color[2] / 2,
            255,
        ]));
        item.set_text_color([235, 235, 235, 255]);
        list.add_item(item, ctx);
    }
    ctx.ui.redraw_all = true;
}

#[derive(Default)]
struct MaterialControls {
    images: HashMap<String, TheRGBABuffer>,
    names: HashMap<String, String>,
    tiles: Vec<GraphPickerItem>,
    palette: Vec<GraphPickerItem>,
    patterns: Vec<GraphPickerItem>,
}

impl MaterialControls {
    fn refresh(&mut self, project: &Project) {
        self.images.clear();
        self.names.clear();
        self.tiles.clear();
        self.palette.clear();
        self.patterns.clear();
        let generated: std::collections::HashSet<_> = project
            .procedural_recipes
            .values()
            .filter_map(|recipe| recipe.tile_id)
            .collect();
        for (id, tile) in &project.tiles {
            if generated.contains(id) || tile.module.is_some() || tile.textures.is_empty() {
                continue;
            }
            let id = id.to_string();
            let label = if tile.alias.trim().is_empty() {
                fl!("entity_tile")
            } else {
                tile.alias.clone()
            };
            if let Some(texture) = tile.textures.first() {
                self.images.insert(
                    id.clone(),
                    TheRGBABuffer::from(
                        texture.data.clone(),
                        texture.width as u32,
                        texture.height as u32,
                    ),
                );
            }
            self.names.insert(id.clone(), label.clone());
            self.tiles.push(GraphPickerItem { id, label });
        }
        for (index, color) in project.art_palette.colors.iter().enumerate() {
            let Some(color) = color else {
                continue;
            };
            let id = format!("palette:{index}");
            self.images.insert(
                id.clone(),
                TheRGBABuffer::from(color.to_u8_array().repeat(32 * 32), 32, 32),
            );
            let label = format!("{} {}", fl!("construction_palette_color"), index + 1);
            self.names.insert(id.clone(), label.clone());
            self.palette.push(GraphPickerItem { id, label });
        }
        for (id, asset) in &project.construction_patterns {
            if asset.kind != ConstructionPatternKind::Pattern {
                continue;
            }
            let id = format!("pattern:{id}");
            self.names.insert(id.clone(), asset.name.clone());
            self.patterns.push(GraphPickerItem {
                id,
                label: asset.name.clone(),
            });
        }
    }

    fn preview_key<'a>(&self, kind: &str, data: &'a str) -> String {
        if kind == "material" {
            if data.starts_with("palette:") || data.starts_with("tile:") {
                return data.trim_start_matches("tile:").to_string();
            }
            return self.preview_key("color", data);
        } else if kind == "palette" {
            format!("palette:{data}")
        } else if kind == "color" {
            let hex = data.trim_start_matches('#');
            if hex.len() == 6 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..2], 16),
                    u8::from_str_radix(&hex[2..4], 16),
                    u8::from_str_radix(&hex[4..6], 16),
                ) {
                    return format!("rgb:{r}:{g}:{b}");
                }
            }
            String::new()
        } else {
            data.to_string()
        }
    }

    fn add_legacy_swatches(&mut self, graph: &GraphDocument) {
        for row in graph.nodes.iter().flat_map(|node| &node.rows) {
            let GraphControlValue::Custom { kind, data } = &row.value else {
                continue;
            };
            if kind != "color" && kind != "material" {
                continue;
            }
            let key = self.preview_key(kind, data.as_str().unwrap_or_default());
            let parts: Vec<_> = key
                .strip_prefix("rgb:")
                .unwrap_or_default()
                .split(':')
                .collect();
            if parts.len() == 3 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    parts[0].parse::<u8>(),
                    parts[1].parse::<u8>(),
                    parts[2].parse::<u8>(),
                ) {
                    self.images.entry(key).or_insert_with(|| {
                        TheRGBABuffer::from([r, g, b, 255].repeat(32 * 32), 32, 32)
                    });
                }
            }
        }
    }
}

impl GraphAssets for MaterialControls {
    fn image(&self, key: &str) -> Option<&TheRGBABuffer> {
        self.images.get(key)
    }
}

impl GraphControls for MaterialControls {
    fn label(&self, value: &GraphControlValue) -> String {
        if let GraphControlValue::Custom { kind, data } = value {
            let data = data.as_str().unwrap_or_default();
            if kind == "tile" {
                return self
                    .names
                    .get(data)
                    .cloned()
                    .unwrap_or_else(|| fl!("entity_select_tile"));
            }
            if kind == "palette" {
                return self
                    .names
                    .get(&format!("palette:{data}"))
                    .cloned()
                    .unwrap_or_else(|| fl!("construction_select_color"));
            }
            if kind == "color" {
                return fl!("construction_select_color");
            }
            if kind == "pattern" {
                return self
                    .names
                    .get(&format!("pattern:{data}"))
                    .cloned()
                    .unwrap_or_else(|| fl!("construction_select_pattern"));
            }
            if kind == "material" {
                if data.starts_with("palette:") {
                    return self
                        .names
                        .get(data)
                        .cloned()
                        .unwrap_or_else(|| fl!("construction_select_color"));
                }
                if let Some(id) = data.strip_prefix("tile:") {
                    return self
                        .names
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| fl!("entity_select_tile"));
                }
                return fl!("construction_select_material");
            }
        }
        BasicGraphControls.label(value)
    }
    fn interact(
        &self,
        value: &GraphControlValue,
        input: GraphControlInput,
    ) -> Option<GraphControlValue> {
        BasicGraphControls.interact(value, input)
    }
    fn paint(
        &self,
        value: &GraphControlValue,
        rect: GraphRect,
        painter: &mut dyn GraphPainter,
    ) -> bool {
        let GraphControlValue::Custom { kind, data } = value else {
            return false;
        };
        if !matches!(
            kind.as_str(),
            "tile" | "color" | "palette" | "pattern" | "material"
        ) {
            return false;
        }
        let key = self.preview_key(kind, data.as_str().unwrap_or_default());
        if matches!(kind.as_str(), "color" | "palette") {
            let color = self
                .images
                .get(&key)
                .and_then(|image| image.get_pixel(0, 0))
                .unwrap_or([75, 78, 80, 255]);
            painter.round_rect(rect, rect.size[1] * 0.32, color);
            return true;
        }
        if kind == "tile" && self.images.contains_key(&key) {
            painter.preview(rect, &key);
            return true;
        }
        let size = (rect.size[1] - 4.).max(1.).min(32.);
        let offset = if self.images.contains_key(&key) {
            painter.preview(
                GraphRect {
                    origin: [
                        rect.origin[0] + 4.,
                        rect.origin[1] + (rect.size[1] - size) * 0.5,
                    ],
                    size: [size, size],
                },
                &key,
            );
            size + 10.
        } else {
            8.
        };
        // `rect` is already in screen coordinates. A fixed font size makes the text grow
        // relative to the node when zooming out and shrink relative to it when zooming in.
        let font_size = 13. * (rect.size[1] / 44.).max(0.01);
        let available = (rect.size[0] - offset - 6.).max(1.);
        let full_label = self.label(value);
        let mut label = full_label.clone();
        if painter.text_width(&label, font_size) > available {
            while !label.is_empty()
                && painter.text_width(&format!("{label}…"), font_size) > available
            {
                label.pop();
            }
            label.push('…');
        }
        painter.text(
            GraphRect {
                origin: [rect.origin[0] + offset, rect.origin[1]],
                size: [(rect.size[0] - offset - 6.).max(1.), rect.size[1]],
            },
            &label,
            font_size,
            [233, 235, 230, 255],
        );
        true
    }
}

struct ConstructionContext;
impl GraphContext for ConstructionContext {
    fn observe(&self, _: &GraphNode) -> GraphObservation {
        GraphObservation::default()
    }
    fn node_title(&self, node: &GraphNode) -> Option<String> {
        Some(match node.definition.as_deref()? {
            "surface" => fl!("construction_node_surface"),
            "inset" => fl!("construction_node_inset"),
            "subdivide" => fl!("construction_node_subdivide"),
            "color" => fl!("construction_node_color"),
            "noise" => fl!("construction_node_noise"),
            "gradient" => fl!("construction_node_gradient"),
            "tile" => fl!("construction_node_tile"),
            "output" => fl!("construction_node_output"),
            "wall" => fl!("construction_node_wall"),
            "masonry" => fl!("construction_node_masonry"),
            "wall_finish" => fl!("construction_node_wall_finish"),
            "pattern_ref" | "surface_pattern_ref" => fl!("construction_node_pattern_ref"),
            "pattern" => fl!("construction_node_pattern"),
            "pattern_output" => fl!("construction_node_pattern_output"),
            "pattern_scale" => fl!("construction_node_pattern_scale"),
            "stone_variants" => fl!("construction_node_stone_variants"),
            "wall_output" => fl!("construction_node_wall_output"),
            _ => return None,
        })
    }
    fn parameter_label(&self, node: &GraphNode, row: &GraphRow) -> Option<String> {
        if matches!(node.definition.as_deref(), Some("color" | "tile")) {
            return Some(String::new());
        }
        Some(match row.key.as_deref()? {
            "noise_mode" => fl!("construction_row_noise_mode"),
            "noise_scale" => fl!("construction_row_noise_scale"),
            "noise_seed" => fl!("construction_row_noise_seed"),
            "color_low" => fl!("construction_row_color_low"),
            "color_high" => fl!("construction_row_color_high"),
            "noise_low" => fl!("construction_row_noise_low"),
            "noise_high" => fl!("construction_row_noise_high"),
            "name" => fl!("construction_row_name"),
            "clearance" => fl!("construction_row_clearance"),
            "color" | "hex" => fl!("construction_select_color"),
            "tile_id" => fl!("construction_row_tile"),
            "texture_scale" => fl!("construction_row_texture_scale"),
            "cell_size" => fl!("construction_row_cell_size"),
            "gap" => fl!("construction_row_gap"),
            "elevation" => fl!("construction_row_elevation"),
            "thickness" => fl!("construction_row_thickness"),
            "height" => fl!("construction_row_height"),
            "width" => fl!("construction_row_width"),
            "amount" => fl!("construction_row_amount"),
            "brick_width" => fl!("construction_row_brick_width"),
            "brick_height" => fl!("construction_row_brick_height"),
            "mortar_gap" => fl!("construction_row_mortar_gap"),
            "course_offset" => fl!("construction_row_course_offset"),
            "masonry" => fl!("construction_row_masonry"),
            "pattern" => fl!("construction_row_pattern"),
            "stone" => fl!("construction_port_stone"),
            "mortar" => fl!("construction_port_mortar"),
            "frame" => fl!("construction_port_frame"),
            "bevel" => fl!("construction_row_bevel"),
            "irregularity" => fl!("construction_row_irregularity"),
            "damage" => fl!("construction_row_damage"),
            "stone_variation" => fl!("construction_row_stone_variation"),
            "frame_width" => fl!("construction_row_frame_width"),
            "frame_depth" => fl!("construction_row_frame_depth"),
            "arch_stones" => fl!("construction_row_arch_stones"),
            _ => return None,
        })
    }
    fn port_label(&self, _: &GraphNode, port: &GraphPort) -> Option<String> {
        Some(match port.key.as_deref()? {
            "geometry" | "geometry_in" => fl!("construction_port_geometry"),
            "material" => fl!("construction_port_material"),
            "value" | "value_in" => fl!("construction_port_value"),
            "top" => fl!("construction_port_top"),
            "sides" => fl!("construction_port_sides"),
            "alternate" => fl!("construction_port_alternate"),
            "alternate_2" => format!("{} 2", fl!("construction_port_alternate")),
            "alternate_3" => format!("{} 3", fl!("construction_port_alternate")),
            "base" => fl!("construction_port_base"),
            "grout" => fl!("construction_port_grout"),
            "stone" => fl!("construction_port_stone"),
            "mortar" => fl!("construction_port_mortar"),
            "frame" => fl!("construction_port_frame"),
            "style" | "build" | "build_in" => fl!("construction_port_build"),
            _ => return None,
        })
    }
}

pub struct ConstructionDock {
    kind: ConstructionPatternKind,
    selected: Option<Uuid>,
    doc: GraphDocument,
    committed: GraphDocument,
    editor: GraphEditor,
    resources: GraphRasterResources,
    materials: MaterialControls,
    popup: Option<GraphPicker>,
    choice_target: Option<(Uuid, Uuid)>,
    dirty: bool,
    initial_layouts: HashMap<Uuid, (GraphDocument, GraphDocument)>,
}

impl ConstructionDock {
    fn graph_toolbar() -> TheCanvas {
        let mut canvas = TheCanvas::new();
        canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut layout = TheHLayout::new(TheId::named("Construction Graph Actions"));
        layout.set_background_color(None);
        layout.set_margin(Vec4::new(6, 2, 6, 2));
        layout.set_padding(5);
        for (id, label, help) in [
            (
                ASSIGN_BRANCH,
                fl!("construction_assign_branch"),
                fl!("construction_branch_help"),
            ),
            (
                REMOVE_BRANCH,
                fl!("construction_remove_branch"),
                fl!("construction_remove_branch_help"),
            ),
            (
                TIDY_GRAPH,
                fl!("construction_tidy_graph"),
                fl!("construction_tidy_graph_help"),
            ),
        ] {
            let mut button = TheTraybarButton::new(TheId::named(id));
            button.set_text(label);
            button.set_status_text(&help);
            button.set_fixed_size(false);
            layout.add_widget(Box::new(button));
        }
        canvas.set_layout(layout);
        canvas
    }

    fn linked_pattern(project: &Project, server: &ServerContext) -> Option<Uuid> {
        let map = project.get_map(server)?;
        let assembly = map.wall_assembly(map.selected_wall_assembly?)?;
        if let Some(surface_id) = map.selected_wall_surface {
            assembly.area_surface(surface_id)?.pattern_id
        } else if let Some(span_id) = map.selected_wall_spans.first() {
            assembly.span(*span_id)?.pattern_id.or(assembly.pattern_id)
        } else {
            assembly.pattern_id
        }
    }

    fn select(&mut self, id: Option<Uuid>, project: &Project) {
        self.selected = id.filter(|id| project.construction_patterns.contains_key(id));
        if let Some(asset) = self
            .selected
            .and_then(|id| project.construction_patterns.get(&id))
        {
            self.kind = asset.kind;
        }
        self.doc = self
            .selected
            .and_then(|id| project.construction_patterns.get(&id))
            .map(|asset| asset.graph.clone())
            .unwrap_or_default();
        construction_graph::normalize_noise_graph(&mut self.doc);
        for node in &mut self.doc.nodes {
            if node.definition.as_deref() == Some("noise") {
                for row in &mut node.rows {
                    if row.key.as_deref() == Some("noise_mode") {
                        if let GraphControlValue::Choice { options, .. } = &mut row.value {
                            *options = vec![
                                fl!("construction_noise_value"),
                                fl!("construction_noise_voronoi"),
                            ];
                        }
                    }
                }
            }

            if matches!(
                node.definition.as_deref(),
                Some("pattern_ref" | "surface_pattern_ref")
            ) {
                node.width = node.width.max(260.);
            } else if matches!(node.definition.as_deref(), Some("color" | "tile")) {
                node.width = 155.;
            }
        }
        if let Some(id) = self.selected {
            if let Some((original, arranged)) = self.initial_layouts.get(&id) {
                if self.doc == *original {
                    self.doc = arranged.clone();
                }
            } else {
                let original = self.doc.clone();
                self.tidy_graph();
                self.initial_layouts
                    .insert(id, (original, self.doc.clone()));
            }
        }
        self.committed = self.doc.clone();
        self.editor = GraphEditor::default();
        self.editor.viewport.zoom_at([0.0, 0.0], 0.9);
    }

    fn tidy_graph(&mut self) -> bool {
        construction_graph::tidy_graph(&mut self.doc)
    }

    fn fit_graph(&mut self, ui: &mut TheUI) {
        let Some(view) = ui.get_render_view(VIEW) else {
            return;
        };
        let dim = *view.dim();
        let nodes: HashSet<_> = self.doc.nodes.iter().map(|node| node.id).collect();
        self.editor
            .viewport
            .fit_to_nodes(&self.doc, [dim.width as f32, dim.height as f32], &nodes);
    }

    fn sync_branches(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server: &ServerContext,
    ) {
        self.materials.refresh(project);
        let linked = Self::linked_pattern(project, server);
        if let Some(list) = ui.get_list_layout(BRANCH_LIST) {
            list.clear();
            for (id, asset) in &project.construction_patterns {
                let mut item = TheListItem::new(TheId::named(&format!("Construction Branch/{id}")));
                let label = match asset.kind {
                    ConstructionPatternKind::Wall => fl!("construction_node_wall"),
                    ConstructionPatternKind::Surface => fl!("construction_node_surface"),
                    ConstructionPatternKind::Pattern => fl!("construction_node_pattern"),
                };
                let linked_marker = if linked == Some(*id) { "● " } else { "" };
                item.set_text(format!("{linked_marker}{label}: {}", asset.name));
                item.set_status_text(&fl!("construction_branch_help"));
                let color = if self.selected == Some(*id) {
                    [64, 96, 120, 255]
                } else {
                    [40, 42, 44, 255]
                };
                item.set_background_color(TheColor::from_u8_array(color));
                item.set_text_color([235, 235, 235, 255]);
                list.add_item(item, ctx);
            }
        }
        sync_node_list(ui, ctx, self.selected.map(|_| self.kind));
    }

    fn render(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        let Some(view) = ui.get_render_view(VIEW) else {
            return;
        };
        let d = *view.dim();
        if d.width <= 0 || d.height <= 0 {
            return;
        }
        view.set_text_input(self.editor.text_focus().is_some() || self.popup.is_some());
        let buffer = view.render_buffer_mut();
        if buffer.dim().width != d.width || buffer.dim().height != d.height {
            *buffer = TheRGBABuffer::new(TheDim::new(0, 0, d.width, d.height));
        }
        buffer.set_render_scale(ctx.ui_render_scale);
        let width = buffer.pixel_width();
        let height = buffer.pixel_height();
        let scale = buffer.render_scale();
        self.materials.add_legacy_swatches(&self.doc);
        let mut painter = RasterGraphPainter::new(
            buffer.pixels_mut(),
            width,
            height,
            scale,
            &mut self.resources,
            &self.materials,
        );
        self.editor.paint(
            &self.doc,
            &ConstructionContext,
            &self.materials,
            &mut painter,
            [d.width as f32, d.height as f32],
            &GraphTheme::default(),
        );
        if let Some(popup) = &self.popup {
            popup.paint(&mut painter);
        }
        if self.selected.is_none() {
            painter.text(
                GraphRect {
                    origin: [20.0, 25.0],
                    size: [d.width as f32 - 40.0, 35.0],
                },
                &fl!("construction_empty"),
                14.0,
                [210, 210, 210, 255],
            );
        }
        ctx.ui.redraw_all = true;
    }

    fn store(&mut self, project: &mut Project, ctx: &mut TheContext, _server: &ServerContext) {
        let Some(id) = self.selected else {
            return;
        };
        if self.doc == self.committed {
            return;
        }
        let previous = project.clone();
        let changed_projection = match self.kind {
            ConstructionPatternKind::Surface => {
                construction_graph::compile_surface_with_patterns(
                    &self.committed,
                    &project.construction_patterns,
                ) != construction_graph::compile_surface_with_patterns(
                    &self.doc,
                    &project.construction_patterns,
                )
            }
            ConstructionPatternKind::Pattern => {
                construction_graph::compile_pattern(
                    &self.committed,
                    &rusterix::map::wall::WallStyle::default(),
                ) != construction_graph::compile_pattern(
                    &self.doc,
                    &rusterix::map::wall::WallStyle::default(),
                )
            }
            ConstructionPatternKind::Wall => {
                construction_graph::compile_wall_with_patterns(
                    &self.committed,
                    &rusterix::map::wall::WallStyle::default(),
                    &project.construction_patterns,
                ) != construction_graph::compile_wall_with_patterns(
                    &self.doc,
                    &rusterix::map::wall::WallStyle::default(),
                    &project.construction_patterns,
                )
            }
        };
        if let Some(asset) = project.construction_patterns.get_mut(&id) {
            asset.graph = self.doc.clone();
            if let Some(name) = self
                .doc
                .nodes
                .iter()
                .filter(|node| {
                    matches!(
                        node.definition.as_deref(),
                        Some("wall" | "surface" | "pattern")
                    )
                })
                .flat_map(|node| &node.rows)
                .find(|row| row.key.as_deref() == Some("name"))
                .and_then(|row| match &row.value {
                    GraphControlValue::Text(value) => Some(value.trim()),
                    _ => None,
                })
            {
                if !name.is_empty() {
                    asset.name = name.into();
                }
            }
        }
        self.committed = self.doc.clone();
        if changed_projection {
            let errors = construction_graph::synchronize(project);
            let mut touched = false;
            for region in &mut project.regions {
                if self.kind != ConstructionPatternKind::Surface
                    || region.map.wall_assemblies.iter().any(|assembly| {
                        assembly.pattern_id == Some(id)
                            || assembly
                                .spans
                                .iter()
                                .any(|span| span.pattern_id == Some(id))
                            || assembly
                                .area_surfaces
                                .iter()
                                .any(|surface| surface.pattern_id == Some(id))
                    })
                {
                    region.map.rebuild_wall_geometry();
                    touched = true;
                }
            }
            if touched {
                RUSTERIX.write().unwrap().set_dirty();
                crate::undo::project_helper::update_region(ctx);
            }
            if !errors.is_empty() {
                ctx.ui
                    .send(TheEvent::SetStatusText(TheId::empty(), errors.join("; ")));
            }
        }
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::ProjectEdit(
                fl!("construction_edit"),
                Box::new(previous),
                Box::new(project.clone()),
            ),
            ctx,
        );
        self.dirty = true;
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Construction Branches"),
            TheValue::Empty,
        ));
    }

    fn apply(&mut self, project: &mut Project, ctx: &mut TheContext, server: &ServerContext) {
        if self.kind == ConstructionPatternKind::Pattern {
            return;
        }
        let Some(id) = self.selected else {
            return;
        };
        let Some(asset) = project.construction_patterns.get(&id) else {
            return;
        };
        let graph = asset.graph.clone();
        let patterns = project.construction_patterns.clone();
        let previous = project.clone();
        let Some(map) = project.get_map_mut(server) else {
            return;
        };
        let Some(assembly_id) = map.selected_wall_assembly else {
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                fl!("construction_select_surface"),
            ));
            return;
        };
        if self.kind == ConstructionPatternKind::Wall && map.selected_wall_surface.is_some() {
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                fl!("construction_select_wall"),
            ));
            return;
        }
        if self.kind == ConstructionPatternKind::Surface {
            let Some(surface_id) = map.selected_wall_surface else {
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("construction_select_surface"),
                ));
                return;
            };
            let projection =
                match construction_graph::compile_surface_with_patterns(&graph, &patterns) {
                    Ok(value) => value,
                    Err(error) => {
                        ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                        return;
                    }
                };
            let Some(surface) = map
                .wall_assembly_mut(assembly_id)
                .and_then(|assembly| assembly.area_surface_mut(surface_id))
            else {
                return;
            };
            surface.pattern_id = Some(id);
            construction_graph::apply_to_surface(surface, &projection);
        } else {
            let selected = map.selected_wall_spans.clone();
            let Some(assembly) = map.wall_assembly_mut(assembly_id) else {
                return;
            };
            if selected.is_empty() {
                let style = match construction_graph::compile_wall_with_patterns(
                    &graph,
                    &assembly.style,
                    &patterns,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                        return;
                    }
                };
                assembly.pattern_id = Some(id);
                assembly.style = style;
            } else {
                let styles = assembly
                    .spans
                    .iter()
                    .filter(|span| selected.contains(&span.id))
                    .map(|span| {
                        let base = span.style_override.as_ref().unwrap_or(&assembly.style);
                        construction_graph::compile_wall_with_patterns(&graph, base, &patterns)
                            .map(|style| (span.id, style))
                    })
                    .collect::<Result<Vec<_>, _>>();
                let styles = match styles {
                    Ok(value) => value,
                    Err(error) => {
                        ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                        return;
                    }
                };
                for (span_id, style) in styles {
                    let Some(span) = assembly.span_mut(span_id) else {
                        continue;
                    };
                    span.pattern_id = Some(id);
                    span.style_override = Some(style);
                }
            }
        }
        map.rebuild_wall_geometry();
        RUSTERIX.write().unwrap().set_dirty();
        crate::undo::project_helper::update_region(ctx);
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::ProjectEdit(
                fl!("construction_apply"),
                Box::new(previous),
                Box::new(project.clone()),
            ),
            ctx,
        );
        self.dirty = true;
    }

    fn node_allowed(&self, id: &str) -> bool {
        match self.kind {
            ConstructionPatternKind::Wall => matches!(id, "pattern_ref" | "wall_output"),
            ConstructionPatternKind::Surface => {
                matches!(
                    id,
                    "inset"
                        | "subdivide"
                        | "surface_pattern_ref"
                        | "color"
                        | "noise"
                        | "gradient"
                        | "tile"
                        | "output"
                )
            }
            ConstructionPatternKind::Pattern => {
                matches!(
                    id,
                    "masonry"
                        | "wall_finish"
                        | "pattern_output"
                        | "pattern_scale"
                        | "color"
                        | "noise"
                        | "gradient"
                        | "tile"
                )
            }
        }
    }

    fn remove_branch(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &ServerContext,
    ) {
        let Some(id) = self.selected else {
            return;
        };
        let map_linked = project
            .regions
            .iter()
            .flat_map(|region| &region.map.wall_assemblies)
            .any(|assembly| {
                assembly.pattern_id == Some(id)
                    || assembly
                        .spans
                        .iter()
                        .any(|span| span.pattern_id == Some(id))
                    || assembly
                        .area_surfaces
                        .iter()
                        .any(|surface| surface.pattern_id == Some(id))
            });
        let id_string = id.to_string();
        let graph_linked = project.construction_patterns.values().any(|asset| {
            asset.graph.nodes.iter().any(|node| matches!(node.definition.as_deref(), Some("pattern_ref" | "surface_pattern_ref"))
                && node.rows.iter().any(|row| row.key.as_deref() == Some("pattern")
                    && matches!(&row.value, GraphControlValue::Custom { data, .. } if data.as_str() == Some(id_string.as_str()))))
        });
        if map_linked || graph_linked {
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                fl!("construction_branch_in_use"),
            ));
            return;
        }
        let previous = project.clone();
        project.construction_patterns.shift_remove(&id);
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::ProjectEdit(
                fl!("construction_remove_branch"),
                Box::new(previous),
                Box::new(project.clone()),
            ),
            ctx,
        );
        self.select(
            project.construction_patterns.keys().next().copied(),
            project,
        );
        self.sync_branches(ui, ctx, project, server);
        self.dirty = true;
    }

    fn open_material_picker(&mut self, point: [f32; 2], ui: &mut TheUI) -> bool {
        let graph_point = self.editor.viewport.to_graph(point);
        let metrics = self.doc.metrics();
        let target = self
            .doc
            .nodes
            .iter()
            .rev()
            .filter(|node| !node.folded)
            .find_map(|node| {
                node.rows.iter().enumerate().find_map(|(index, row)| {
                    let GraphControlValue::Custom { kind, .. } = &row.value else {
                        return None;
                    };
                    if matches!(
                        kind.as_str(),
                        "tile" | "color" | "palette" | "pattern" | "material"
                    ) && node.row_rect(index, &metrics).contains(graph_point)
                    {
                        Some((
                            node.id,
                            row.id,
                            kind.as_str(),
                            node.row_rect(index, &metrics),
                        ))
                    } else {
                        None
                    }
                })
            });
        let Some((node_id, row_id, kind, row)) = target else {
            return false;
        };
        let Some(view) = ui.get_render_view(VIEW) else {
            return false;
        };
        let dim = *view.dim();
        let anchor = GraphRect {
            origin: self.editor.viewport.to_screen(row.origin),
            size: [
                row.size[0] * self.editor.viewport.zoom(),
                row.size[1] * self.editor.viewport.zoom(),
            ],
        };
        let mut items = match kind {
            "tile" => self.materials.tiles.clone(),
            "pattern" => self.materials.patterns.clone(),
            "material" => self
                .materials
                .palette
                .iter()
                .chain(self.materials.tiles.iter())
                .cloned()
                .collect(),
            _ => self.materials.palette.clone(),
        };
        if kind == "pattern" {
            let current = self.selected.map(|id| format!("pattern:{id}"));
            items.retain(|item| Some(item.id.as_str()) != current.as_deref());
        }
        let mut popup = if kind == "pattern" {
            GraphPicker::compact(anchor, [dim.width as f32, dim.height as f32], items)
        } else {
            GraphPicker::grid(anchor, [dim.width as f32, dim.height as f32], items)
        };
        popup.previews = self
            .materials
            .images
            .keys()
            .map(|id| (id.clone(), id.clone()))
            .collect();
        popup.search_label = fl!("node_search");
        popup.empty_label = fl!("node_no_results");
        self.choice_target = Some((node_id, row_id));
        self.popup = Some(popup);
        true
    }

    fn pick_material(&mut self, item: String) {
        let Some((node_id, row_id)) = self.choice_target.take() else {
            return;
        };
        let gradient = self
            .doc
            .nodes
            .iter()
            .any(|node| node.id == node_id && node.definition.as_deref() == Some("gradient"));
        let gradient_color = if gradient {
            self.materials
                .images
                .get(&item)
                .and_then(|image| image.get_pixel(0, 0))
        } else {
            None
        };
        let Some(row) = self
            .doc
            .nodes
            .iter_mut()
            .find(|node| node.id == node_id)
            .and_then(|node| node.rows.iter_mut().find(|row| row.id == row_id))
        else {
            return;
        };
        let Some((kind, data)) = (if let Some(id) = item.strip_prefix("pattern:") {
            Some(("pattern", id.to_string()))
        } else if let Some(index) = item.strip_prefix("palette:") {
            Some(("palette", index.to_string()))
        } else if !item.is_empty() {
            Some(("tile", item))
        } else {
            None
        }) else {
            return;
        };
        if let Some(color) = gradient_color {
            row.value = GraphControlValue::Custom {
                kind: "color".into(),
                data: format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2]).into(),
            };
            return;
        }
        if matches!(&row.value, GraphControlValue::Custom { kind, .. } if kind == "material") {
            let token = if kind == "tile" {
                format!("tile:{data}")
            } else {
                format!("palette:{data}")
            };
            row.value = GraphControlValue::Custom {
                kind: "material".into(),
                data: token.into(),
            };
        } else {
            row.value = GraphControlValue::Custom {
                kind: kind.into(),
                data: data.into(),
            };
        }
    }
}

impl Dock for ConstructionDock {
    fn new() -> Self {
        let font = fontdue::Font::from_bytes(
            theframework::Embedded::get("fonts/Roboto-Bold.ttf")
                .unwrap()
                .data
                .as_ref(),
            fontdue::FontSettings::default(),
        )
        .unwrap();
        Self {
            kind: ConstructionPatternKind::Wall,
            selected: None,
            doc: GraphDocument::default(),
            committed: GraphDocument::default(),
            editor: GraphEditor::default(),
            resources: GraphRasterResources::new(font),
            materials: MaterialControls::default(),
            popup: None,
            choice_target: None,
            dirty: false,
            initial_layouts: HashMap::new(),
        }
    }

    fn setup(&mut self, _: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();
        canvas.set_top(Self::graph_toolbar());
        canvas.set_left(branch_list_canvas());
        let mut view = TheRenderView::new(TheId::named(VIEW));
        view.set_scroll_behavior(-0.6, false);
        view.limiter_mut()
            .set_max_size(Vec2::new(i32::MAX, i32::MAX));
        canvas.set_widget(view);
        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server: &mut ServerContext,
    ) {
        self.materials.refresh(project);
        let id = Self::linked_pattern(project, server)
            .or(self.selected)
            .or_else(|| {
                project
                    .construction_patterns
                    .iter()
                    .find(|(_, asset)| asset.kind == ConstructionPatternKind::Wall)
                    .map(|(id, _)| *id)
            })
            .or_else(|| project.construction_patterns.keys().next().copied());
        self.select(id, project);
        self.fit_graph(ui);
        self.sync_branches(ui, ctx, project, server);
        self.render(ui, ctx);
    }

    fn supports_actions(&self) -> bool {
        false
    }
    fn has_changes(&self) -> bool {
        self.dirty
    }
    fn mark_saved(&mut self) {
        self.dirty = false;
    }
    fn reset_for_project_switch(&mut self) {
        self.selected = None;
        self.doc = GraphDocument::default();
        self.committed = self.doc.clone();
        self.popup = None;
        self.choice_target = None;
        self.dirty = false;
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut ServerContext,
    ) -> bool {
        match event {
            TheEvent::WidgetResized(id, _) if id.name == VIEW => self.render(ui, ctx),
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == ASSIGN_BRANCH => {
                self.editor.finish_text(&mut self.doc, true);
                self.store(project, ctx, server);
                self.apply(project, ctx, server);
                self.sync_branches(ui, ctx, project, server);
                self.render(ui, ctx);
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == REMOVE_BRANCH => {
                self.editor.finish_text(&mut self.doc, true);
                self.store(project, ctx, server);
                self.remove_branch(ui, ctx, project, server);
                self.render(ui, ctx);
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == TIDY_GRAPH => {
                self.editor.finish_text(&mut self.doc, true);
                self.tidy_graph();
                self.store(project, ctx, server);
                self.fit_graph(ui);
                self.render(ui, ctx);
            }
            TheEvent::StateChanged(id, state)
                if matches!(state, TheWidgetState::Clicked | TheWidgetState::Selected)
                    && id.name.starts_with("Construction Branch/") =>
            {
                let Some(branch_id) = id
                    .name
                    .strip_prefix("Construction Branch/")
                    .and_then(|id| Uuid::parse_str(id).ok())
                else {
                    return false;
                };
                self.editor.finish_text(&mut self.doc, true);
                self.store(project, ctx, server);
                self.select(Some(branch_id), project);
                self.fit_graph(ui);
                self.sync_branches(ui, ctx, project, server);
                self.render(ui, ctx);
            }
            TheEvent::RenderViewDrop(id, point, drop) if id.name == VIEW => {
                let Some(key) = drop.id.name.strip_prefix("Node Catalog/") else {
                    return false;
                };
                if key != drop.data || !node_available(key) {
                    return false;
                }
                self.editor.finish_text(&mut self.doc, true);
                self.store(project, ctx, server);
                if matches!(key, "wall" | "surface" | "pattern") {
                    let before = project.clone();
                    let root_label = match key {
                        "wall" => fl!("construction_node_wall"),
                        "surface" => fl!("construction_node_surface"),
                        _ => fl!("construction_node_pattern"),
                    };
                    let name = format!("{root_label} {}", project.construction_patterns.len() + 1);
                    let asset = match key {
                        "wall" => {
                            let style = project
                                .get_map(server)
                                .and_then(|map| {
                                    map.selected_wall_assembly
                                        .and_then(|id| map.wall_assembly(id))
                                        .map(|assembly| {
                                            map.selected_wall_spans
                                                .first()
                                                .and_then(|id| assembly.span(*id))
                                                .and_then(|span| span.style_override.as_ref())
                                                .unwrap_or(&assembly.style)
                                        })
                                })
                                .cloned()
                                .unwrap_or_default();
                            let pattern = ConstructionPatternAsset::new_pattern(
                                format!(
                                    "{} {}",
                                    fl!("construction_node_pattern"),
                                    project.construction_patterns.len() + 1
                                ),
                                &style,
                            );
                            let pattern_id = pattern.id;
                            project.construction_patterns.insert(pattern_id, pattern);
                            let mut wall = ConstructionPatternAsset::new_wall(name, &style);
                            wall.graph =
                                construction_graph::wall_reference_graph(&style, pattern_id);
                            construction_graph::set_root_name(&mut wall.graph, &wall.name);
                            wall
                        }
                        "surface" => {
                            let (elevation, thickness) = project
                                .get_map(server)
                                .and_then(|map| {
                                    map.selected_wall_assembly
                                        .and_then(|id| map.wall_assembly(id))
                                        .and_then(|assembly| {
                                            map.selected_wall_surface
                                                .and_then(|id| assembly.area_surface(id))
                                        })
                                })
                                .map(|surface| (surface.elevation, surface.thickness))
                                .unwrap_or((0.25, 0.08));
                            ConstructionPatternAsset::new_surface(name, elevation, thickness)
                        }
                        _ => ConstructionPatternAsset::new_pattern(
                            name,
                            &rusterix::map::wall::WallStyle::default(),
                        ),
                    };
                    let id = asset.id;
                    project.construction_patterns.insert(id, asset);
                    UNDOMANAGER.write().unwrap().add_undo(
                        ProjectUndoAtom::ProjectEdit(
                            fl!("construction_create"),
                            Box::new(before),
                            Box::new(project.clone()),
                        ),
                        ctx,
                    );
                    self.select(Some(id), project);
                    self.fit_graph(ui);
                    self.dirty = true;
                    self.sync_branches(ui, ctx, project, server);
                } else {
                    if self.selected.is_none() || !self.node_allowed(key) {
                        return false;
                    }
                    let position = self
                        .editor
                        .viewport
                        .to_graph([point.x as f32, point.y as f32]);
                    if let Ok(mut node) =
                        construction_graph::definitions().instantiate(key, position)
                    {
                        if key == "noise" {
                            if let GraphControlValue::Choice { options, .. } =
                                &mut node.rows[0].value
                            {
                                *options = vec![
                                    fl!("construction_noise_value"),
                                    fl!("construction_noise_voronoi"),
                                ];
                            }
                        }
                        self.editor.selected = Some(node.id);
                        self.doc.nodes.push(node);
                        self.store(project, ctx, server);
                    }
                }
                self.render(ui, ctx);
            }
            TheEvent::RenderViewClicked(id, point) if id.name == VIEW => {
                if self.selected.is_none() {
                    return false;
                }
                if let Some(view) = ui.get_render_view(VIEW) {
                    ctx.ui.set_focus(view.id());
                }
                let position = [point.x as f32, point.y as f32];
                if let Some(popup) = self.popup.take() {
                    if let Some(item) = popup.pick(position) {
                        self.pick_material(item);
                        self.store(project, ctx, server);
                    } else if popup.contains(position) {
                        self.popup = Some(popup);
                    } else {
                        self.choice_target = None;
                    }
                } else if self.open_material_picker(position, ui) {
                    self.editor.finish_text(&mut self.doc, false);
                } else if ui.shift {
                    self.editor.begin_cut(&mut self.doc, position);
                } else {
                    self.editor
                        .pointer_down(&mut self.doc, position, &self.materials);
                }
                self.render(ui, ctx);
            }
            TheEvent::RenderViewDragged(id, point) if id.name == VIEW && self.popup.is_none() => {
                self.editor.pointer_move(
                    &mut self.doc,
                    [point.x as f32, point.y as f32],
                    &self.materials,
                );
                self.render(ui, ctx);
            }
            TheEvent::RenderViewUp(id, point) if id.name == VIEW && self.popup.is_none() => {
                self.editor.pointer_up(
                    &mut self.doc,
                    [point.x as f32, point.y as f32],
                    &AllowGraphConnections,
                );
                self.store(project, ctx, server);
                self.render(ui, ctx);
            }
            TheEvent::RenderViewScrollBy(id, delta)
            | TheEvent::RenderViewPreciseScrollBy(id, delta)
                if id.name == VIEW =>
            {
                if let Some(popup) = &mut self.popup {
                    if matches!(event, TheEvent::RenderViewPreciseScrollBy(_, _)) {
                        popup.scroll_pixels(delta.y as f32);
                    } else {
                        popup.scroll_by(delta.y.signum());
                    }
                } else if ui.ctrl || ui.logo {
                    self.editor
                        .viewport
                        .zoom_at(self.editor.cursor, (delta.y as f32 * 0.01).exp());
                } else {
                    self.editor.viewport.pan[0] -= delta.x as f32;
                    self.editor.viewport.pan[1] -= delta.y as f32;
                }
                self.render(ui, ctx);
            }
            TheEvent::RenderViewZoomBy(id, delta) if id.name == VIEW => {
                self.editor
                    .viewport
                    .zoom_at(self.editor.cursor, (1.0 + *delta).max(0.1));
                self.render(ui, ctx);
            }
            TheEvent::RenderViewHoverChanged(id, point) if id.name == VIEW => {
                self.editor.cursor = [point.x as f32, point.y as f32];
                if let Some(popup) = &mut self.popup {
                    popup.hover(Some(self.editor.cursor));
                    self.render(ui, ctx);
                }
            }
            TheEvent::KeyDown(TheValue::Char(ch))
                if ctx.ui.focus.as_ref().is_some_and(|id| id.name == VIEW) =>
            {
                if let Some(popup) = &mut self.popup {
                    popup.type_char(*ch);
                } else {
                    self.editor
                        .text_input(&mut self.doc, GraphTextInput::Insert(ch.to_string()));
                }
                self.render(ui, ctx);
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(key))
                if ctx.ui.focus.as_ref().is_some_and(|id| id.name == VIEW) =>
            {
                if let Some(mut popup) = self.popup.take() {
                    match key {
                        TheKeyCode::Escape => {
                            self.choice_target = None;
                        }
                        TheKeyCode::Delete => {
                            popup.backspace();
                            self.popup = Some(popup);
                        }
                        TheKeyCode::Return => {
                            let filtered = popup.filtered();
                            let choice = filtered
                                .iter()
                                .find(|item| popup.hovered.as_deref() == Some(item.id.as_str()))
                                .or_else(|| filtered.get(popup.scroll))
                                .map(|item| item.id.clone());
                            if let Some(item) = choice {
                                self.pick_material(item);
                                self.store(project, ctx, server);
                            }
                        }
                        _ => self.popup = Some(popup),
                    }
                    self.render(ui, ctx);
                    return true;
                }
                match key {
                    TheKeyCode::Return => {
                        self.editor.finish_text(&mut self.doc, true);
                        self.store(project, ctx, server);
                    }
                    TheKeyCode::Delete if self.editor.text_focus().is_some() => {
                        self.editor
                            .text_input(&mut self.doc, GraphTextInput::Backspace);
                    }
                    TheKeyCode::Delete => {
                        self.editor.delete_connection(&mut self.doc);
                        if let Some(id) = self.editor.selected.take() {
                            let root = self.doc.nodes.iter().any(|node| {
                                node.id == id
                                    && matches!(
                                        node.definition.as_deref(),
                                        Some("wall" | "surface" | "pattern")
                                    )
                            });
                            if !root {
                                self.doc.nodes.retain(|node| node.id != id);
                            }
                            let ports: std::collections::HashSet<_> = self
                                .doc
                                .nodes
                                .iter()
                                .flat_map(|node| node.ports.iter().map(|port| port.id))
                                .collect();
                            self.doc.connections.retain(|link| {
                                ports.contains(&link.from) && ports.contains(&link.to)
                            });
                        }
                        self.store(project, ctx, server);
                    }
                    TheKeyCode::Escape => {
                        self.editor.finish_text(&mut self.doc, false);
                    }
                    _ => {}
                }
                self.render(ui, ctx);
            }
            TheEvent::Custom(id, _) if id.name == "Map Selection Changed" => {
                self.store(project, ctx, server);
                let pattern_id = Self::linked_pattern(project, server)
                    .or(self.selected)
                    .or_else(|| project.construction_patterns.keys().next().copied());
                if pattern_id != self.selected {
                    self.select(pattern_id, project);
                }
                self.sync_branches(ui, ctx, project, server);
                self.render(ui, ctx);
            }
            TheEvent::Custom(id, _) if id.name == "Update Action List" => {
                if self.editor.text_focus().is_none() {
                    let saved = self
                        .selected
                        .and_then(|id| project.construction_patterns.get(&id));
                    if saved.map(|asset| &asset.graph) != Some(&self.committed) {
                        self.select(saved.map(|asset| asset.id), project);
                        self.sync_branches(ui, ctx, project, server);
                        self.render(ui, ctx);
                    }
                }
            }
            TheEvent::Custom(id, _) if id.name == "Update Construction Branches" => {
                self.sync_branches(ui, ctx, project, server);
                self.render(ui, ctx);
            }
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn reopening_and_cutting_preserve_manual_layout() {
        let mut project = Project::new();
        let mut asset = ConstructionPatternAsset::new("Floor");
        for node in &mut asset.graph.nodes {
            node.position = [900., 900.];
        }
        let id = asset.id;
        project.construction_patterns.insert(id, asset);
        let mut dock = ConstructionDock::new();
        dock.select(Some(id), &project);
        let initial = dock.doc.clone();
        assert!(
            initial
                .nodes
                .iter()
                .any(|node| node.position != [900., 900.])
        );
        dock.select(None, &project);
        dock.select(Some(id), &project);
        assert_eq!(dock.doc, initial);

        dock.doc.nodes[0].position = [1234., 567.];
        dock.doc.connections.clear();
        project.construction_patterns.get_mut(&id).unwrap().graph = dock.doc.clone();
        let manual = dock.doc.clone();
        dock.select(None, &project);
        dock.select(Some(id), &project);
        assert_eq!(dock.doc, manual);
    }
}
