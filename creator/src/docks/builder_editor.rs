use crate::prelude::*;
use rusterix::Surface;
use std::fs;

const BUILDER_CANVAS_VIEW: &str = "Builder Graph NodeCanvas";
const BUILDER_SETTINGS_LAYOUT: &str = "Builder Graph Settings";
const BUILDER_ADD_BUTTON: &str = "Builder Graph Add";
const BUILDER_GRAPH_BUTTON: &str = "Builder Graph Menu";
const BUILDER_RESET_BUTTON: &str = "Builder Graph Reset";

fn builder_node_name(kind: &BuilderNodeKind) -> &'static str {
    match kind {
        BuilderNodeKind::SectorSurface => "Sector Surface",
        BuilderNodeKind::LinedefSurface => "Linedef Surface",
        BuilderNodeKind::VertexPoint => "Vertex Point",
        BuilderNodeKind::Offset { .. } => "Offset",
        BuilderNodeKind::CornerLayout { .. } => "Corner Layout",
        BuilderNodeKind::Box { .. } => "Box",
        BuilderNodeKind::SectorCorners { .. } => "Sector Corners",
        BuilderNodeKind::SectorGrid { .. } => "Sector Grid",
        BuilderNodeKind::SectorEdges { .. } => "Sector Edges",
        BuilderNodeKind::LinedefRow { .. } => "Linedef Row",
        BuilderNodeKind::LinedefSpan { .. } => "Linedef Span",
        BuilderNodeKind::ItemAnchor { .. } => "Item Anchor",
        BuilderNodeKind::ItemSurface { .. } => "Item Surface",
        BuilderNodeKind::MaterialAnchor { .. } => "Material Anchor",
        BuilderNodeKind::ItemSlot { .. } => "Item Slot",
        BuilderNodeKind::MaterialSlot { .. } => "Material Slot",
        BuilderNodeKind::Join => "Join",
        BuilderNodeKind::GeometryOutput { .. } => "Geometry Output",
    }
}

fn builder_node_status_text(kind: &BuilderNodeKind) -> &'static str {
    match kind {
        BuilderNodeKind::SectorSurface => "Exposes the current sector surface and its layout.",
        BuilderNodeKind::LinedefSurface => "Exposes the current linedef span and its layout.",
        BuilderNodeKind::VertexPoint => "Exposes the current vertex point as a placement source.",
        BuilderNodeKind::Offset { .. } => {
            "Offsets upstream surface-relative geometry or layout in local X/Y/Z."
        }
        BuilderNodeKind::CornerLayout { .. } => {
            "Creates four corner placements from the upstream sector layout."
        }
        BuilderNodeKind::Box { .. } => {
            "Primitive box used for builder geometry. Width or depth 0 means relative to the incoming host surface/span."
        }
        BuilderNodeKind::SectorCorners { .. } => {
            "Places upstream geometry into the four corners of the host sector."
        }
        BuilderNodeKind::SectorGrid { .. } => {
            "Subdivides the host sector into a grid and instances upstream geometry."
        }
        BuilderNodeKind::SectorEdges { .. } => {
            "Places upstream geometry along the selected edges of the host sector."
        }
        BuilderNodeKind::LinedefRow { .. } => {
            "Distributes upstream geometry along the current linedef span."
        }
        BuilderNodeKind::LinedefSpan { .. } => {
            "Stretches upstream geometry across the current linedef span."
        }
        BuilderNodeKind::ItemAnchor { .. } => {
            "Creates an item attachment on the top of upstream geometry."
        }
        BuilderNodeKind::ItemSurface { .. } => {
            "Creates an item attachment surface on the top of upstream geometry."
        }
        BuilderNodeKind::MaterialAnchor { .. } => {
            "Creates a material attachment on the top of upstream geometry."
        }
        BuilderNodeKind::ItemSlot { .. } => {
            "Defines a named attachment point for items on the built prop."
        }
        BuilderNodeKind::MaterialSlot { .. } => {
            "Defines a named material attachment point for the built prop."
        }
        BuilderNodeKind::Join => "Combines multiple geometry and anchor branches into one output.",
        BuilderNodeKind::GeometryOutput { .. } => "Final builder assembly output and host target.",
    }
}

fn builder_node_inputs(kind: &BuilderNodeKind) -> Vec<TheNodeTerminal> {
    let assembly = |name: &str| TheNodeTerminal {
        name: name.to_string(),
        category_name: "Assembly".to_string(),
    };
    match kind {
        BuilderNodeKind::SectorSurface
        | BuilderNodeKind::LinedefSurface
        | BuilderNodeKind::VertexPoint
        | BuilderNodeKind::Box { .. } => Vec::new(),
        BuilderNodeKind::Offset { .. }
        | BuilderNodeKind::CornerLayout { .. }
        | BuilderNodeKind::SectorCorners { .. }
        | BuilderNodeKind::SectorGrid { .. }
        | BuilderNodeKind::SectorEdges { .. }
        | BuilderNodeKind::LinedefRow { .. }
        | BuilderNodeKind::LinedefSpan { .. }
        | BuilderNodeKind::ItemAnchor { .. }
        | BuilderNodeKind::ItemSurface { .. }
        | BuilderNodeKind::MaterialAnchor { .. } => vec![assembly("In")],
        BuilderNodeKind::ItemSlot { .. } | BuilderNodeKind::MaterialSlot { .. } => Vec::new(),
        BuilderNodeKind::Join => vec![
            assembly("A"),
            assembly("B"),
            assembly("C"),
            assembly("D"),
            assembly("E"),
            assembly("F"),
            assembly("G"),
            assembly("H"),
        ],
        BuilderNodeKind::GeometryOutput { .. } => vec![assembly("Assembly")],
    }
}

fn builder_node_outputs(kind: &BuilderNodeKind) -> Vec<TheNodeTerminal> {
    let assembly = |name: &str| TheNodeTerminal {
        name: name.to_string(),
        category_name: "Assembly".to_string(),
    };
    match kind {
        BuilderNodeKind::SectorSurface
        | BuilderNodeKind::LinedefSurface
        | BuilderNodeKind::VertexPoint
        | BuilderNodeKind::Offset { .. }
        | BuilderNodeKind::CornerLayout { .. }
        | BuilderNodeKind::Box { .. }
        | BuilderNodeKind::SectorCorners { .. }
        | BuilderNodeKind::SectorGrid { .. }
        | BuilderNodeKind::SectorEdges { .. }
        | BuilderNodeKind::LinedefRow { .. }
        | BuilderNodeKind::LinedefSpan { .. }
        | BuilderNodeKind::ItemAnchor { .. }
        | BuilderNodeKind::ItemSurface { .. }
        | BuilderNodeKind::MaterialAnchor { .. }
        | BuilderNodeKind::Join
        | BuilderNodeKind::ItemSlot { .. }
        | BuilderNodeKind::MaterialSlot { .. } => vec![assembly("Out")],
        BuilderNodeKind::GeometryOutput { .. } => Vec::new(),
    }
}

pub struct BuilderEditorDock {
    graph: BuilderGraph,
    active_builder_id: Option<Uuid>,
    categories: FxHashMap<String, TheColor>,
}

impl BuilderEditorDock {
    fn rename_field_text(&self) -> String {
        if let Some(index) = self.graph.selected_node
            && let Some(node) = self.graph.nodes.get(index)
        {
            if matches!(node.kind, BuilderNodeKind::GeometryOutput { .. }) {
                return self.graph.name.clone();
            }
            return node.name.clone();
        }
        self.graph.name.clone()
    }

    fn sync_rename_field(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        ui.set_widget_value(
            "Builder Editor Name",
            ctx,
            TheValue::Text(self.rename_field_text()),
        );
    }

    fn default_categories() -> FxHashMap<String, TheColor> {
        let mut categories = FxHashMap::default();
        categories.insert("Assembly".into(), TheColor::from("#4c8bf5"));
        categories.insert("Placement".into(), TheColor::from("#0aa37f"));
        categories.insert("Attachment".into(), TheColor::from("#64a6a8"));
        categories.insert("Output".into(), TheColor::from("#d47f4a"));
        categories
    }

    fn graph_to_canvas(&self) -> TheNodeCanvas {
        let mut canvas = TheNodeCanvas {
            node_width: 156,
            selected_node: self.graph.selected_node,
            offset: self.graph.scroll_offset,
            connections: self.graph_connections_to_canvas(),
            categories: self.categories.clone(),
            ..Default::default()
        };

        for (index, node) in self.graph.nodes.iter().enumerate() {
            let type_name = builder_node_name(&node.kind);
            let display_name = if node.name.is_empty() {
                type_name.to_string()
            } else if node.name == type_name {
                node.name.clone()
            } else {
                format!("{} · {}", node.name, type_name)
            };
            canvas.nodes.push(TheNode {
                name: display_name,
                status_text: Some(builder_node_status_text(&node.kind).to_string()),
                position: node.pos,
                inputs: builder_node_inputs(&node.kind),
                outputs: builder_node_outputs(&node.kind),
                preview: TheRGBABuffer::default(),
                supports_preview: false,
                preview_is_open: !node.preview_collapsed,
                can_be_deleted: index != 0,
            });
        }

        canvas
    }

    fn graph_connections_to_canvas(&self) -> Vec<(u16, u8, u16, u8)> {
        self.graph
            .connections
            .iter()
            .filter_map(|(from_id, from_terminal, to_id, to_terminal)| {
                let from_index = self
                    .graph
                    .nodes
                    .iter()
                    .position(|node| node.id == *from_id)?;
                let to_index = self.graph.nodes.iter().position(|node| node.id == *to_id)?;
                Some((
                    from_index as u16,
                    *from_terminal,
                    to_index as u16,
                    *to_terminal,
                ))
            })
            .collect()
    }

    fn canvas_connections_to_graph(
        &self,
        connections: &[(u16, u8, u16, u8)],
    ) -> Vec<(u16, u8, u16, u8)> {
        connections
            .iter()
            .filter_map(|(from_index, from_terminal, to_index, to_terminal)| {
                let from_node = self.graph.nodes.get(*from_index as usize)?;
                let to_node = self.graph.nodes.get(*to_index as usize)?;
                Some((from_node.id, *from_terminal, to_node.id, *to_terminal))
            })
            .collect()
    }

    fn sync_canvas(&self, ui: &mut TheUI) {
        ui.set_node_canvas(BUILDER_CANVAS_VIEW, self.graph_to_canvas());
        ui.set_node_overlay(BUILDER_CANVAS_VIEW, Some(self.render_preview_overlay()));
    }

    fn render_preview_overlay(&self) -> TheRGBABuffer {
        let preview = self.graph.render_preview(156);
        let mut buffer =
            TheRGBABuffer::new(TheDim::sized(preview.width as i32, preview.height as i32));
        buffer.pixels_mut().copy_from_slice(&preview.pixels);
        buffer
    }

    fn clear_selected_node_ui(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(layout) = ui.get_text_layout(BUILDER_SETTINGS_LAYOUT) {
            layout.clear();
            ctx.ui.relayout = true;
        }
    }

    fn set_selected_node_ui(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        let mut nodeui = TheNodeUI::default();

        if let Some(index) = self.graph.selected_node
            && let Some(node) = self.graph.nodes.get(index)
        {
            nodeui.add_item(TheNodeUIItem::OpenTree("node".into()));
            match &node.kind {
                BuilderNodeKind::SectorSurface
                | BuilderNodeKind::LinedefSurface
                | BuilderNodeKind::VertexPoint => {}
                BuilderNodeKind::Offset { translate } => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeOffsetX".into(),
                        "Offset X".into(),
                        "Local X offset.".into(),
                        translate.x,
                        -1.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeOffsetY".into(),
                        "Offset Y".into(),
                        "Local Y offset.".into(),
                        translate.y,
                        -2.0..=4.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeOffsetZ".into(),
                        "Offset Z".into(),
                        "Local Z offset.".into(),
                        translate.z,
                        -1.0..=1.0,
                        false,
                    ));
                }
                BuilderNodeKind::CornerLayout { inset_x, inset_z } => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeCornerLayoutInsetX".into(),
                        "Inset X".into(),
                        "Inset from the left and right sector sides.".into(),
                        *inset_x,
                        0.0..=0.45,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeCornerLayoutInsetZ".into(),
                        "Inset Z".into(),
                        "Inset from the top and bottom sector sides.".into(),
                        *inset_z,
                        0.0..=0.45,
                        false,
                    ));
                }
                BuilderNodeKind::Box {
                    width,
                    depth,
                    height,
                } => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeBoxWidth".into(),
                        "Width".into(),
                        "Width of the box. Set to 0 to use the incoming host width.".into(),
                        *width,
                        0.0..=8.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeBoxDepth".into(),
                        "Depth".into(),
                        "Depth of the box. Set to 0 to use the incoming host depth.".into(),
                        *depth,
                        0.0..=8.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeBoxHeight".into(),
                        "Height".into(),
                        "Height of the box.".into(),
                        *height,
                        0.05..=8.0,
                        false,
                    ));
                }
                BuilderNodeKind::SectorCorners {
                    inset_x,
                    inset_z,
                    elevation,
                } => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeCornersInsetX".into(),
                        "Inset X".into(),
                        "Inset from the left and right sector sides.".into(),
                        *inset_x,
                        0.0..=0.45,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeCornersInsetZ".into(),
                        "Inset Z".into(),
                        "Inset from the top and bottom sector sides.".into(),
                        *inset_z,
                        0.0..=0.45,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeCornersElevation".into(),
                        "Elevation".into(),
                        "Vertical placement above the host.".into(),
                        *elevation,
                        -2.0..=4.0,
                        false,
                    ));
                }
                BuilderNodeKind::SectorGrid {
                    columns,
                    rows,
                    inset_x,
                    inset_z,
                    elevation,
                } => {
                    nodeui.add_item(TheNodeUIItem::IntEditSlider(
                        "builderNodeGridColumns".into(),
                        "Columns".into(),
                        "How many placements across the host width.".into(),
                        *columns as i32,
                        1..=8,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::IntEditSlider(
                        "builderNodeGridRows".into(),
                        "Rows".into(),
                        "How many placements across the host depth.".into(),
                        *rows as i32,
                        1..=8,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeGridInsetX".into(),
                        "Inset X".into(),
                        "Inset from the left and right sector sides.".into(),
                        *inset_x,
                        0.0..=0.45,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeGridInsetZ".into(),
                        "Inset Z".into(),
                        "Inset from the top and bottom sector sides.".into(),
                        *inset_z,
                        0.0..=0.45,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeGridElevation".into(),
                        "Elevation".into(),
                        "Vertical placement above the host.".into(),
                        *elevation,
                        -2.0..=4.0,
                        false,
                    ));
                }
                BuilderNodeKind::SectorEdges {
                    north,
                    south,
                    east,
                    west,
                    inset,
                    elevation,
                } => {
                    nodeui.add_item(TheNodeUIItem::Checkbox(
                        "builderNodeEdgesNorth".into(),
                        "North".into(),
                        "Place along the north edge.".into(),
                        *north,
                    ));
                    nodeui.add_item(TheNodeUIItem::Checkbox(
                        "builderNodeEdgesSouth".into(),
                        "South".into(),
                        "Place along the south edge.".into(),
                        *south,
                    ));
                    nodeui.add_item(TheNodeUIItem::Checkbox(
                        "builderNodeEdgesEast".into(),
                        "East".into(),
                        "Place along the east edge.".into(),
                        *east,
                    ));
                    nodeui.add_item(TheNodeUIItem::Checkbox(
                        "builderNodeEdgesWest".into(),
                        "West".into(),
                        "Place along the west edge.".into(),
                        *west,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeEdgesInset".into(),
                        "Inset".into(),
                        "Inset from the host edge.".into(),
                        *inset,
                        0.0..=0.45,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeEdgesElevation".into(),
                        "Elevation".into(),
                        "Vertical placement above the host.".into(),
                        *elevation,
                        -2.0..=4.0,
                        false,
                    ));
                }
                BuilderNodeKind::LinedefRow {
                    count,
                    inset,
                    elevation,
                } => {
                    nodeui.add_item(TheNodeUIItem::IntEditSlider(
                        "builderNodeLinedefCount".into(),
                        "Count".into(),
                        "How many placements along the linedef span.".into(),
                        *count as i32,
                        1..=16,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeLinedefInset".into(),
                        "Inset".into(),
                        "Inset from the start and end of the linedef span.".into(),
                        *inset,
                        0.0..=0.45,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeLinedefElevation".into(),
                        "Elevation".into(),
                        "Vertical placement above the host.".into(),
                        *elevation,
                        -2.0..=4.0,
                        false,
                    ));
                }
                BuilderNodeKind::LinedefSpan { inset, elevation } => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeLinedefSpanInset".into(),
                        "Inset".into(),
                        "Inset from the start and end of the linedef span.".into(),
                        *inset,
                        0.0..=0.45,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeLinedefSpanElevation".into(),
                        "Elevation".into(),
                        "Vertical placement above the host.".into(),
                        *elevation,
                        -2.0..=4.0,
                        false,
                    ));
                }
                BuilderNodeKind::ItemAnchor { name }
                | BuilderNodeKind::ItemSurface { name }
                | BuilderNodeKind::MaterialAnchor { name } => {
                    nodeui.add_item(TheNodeUIItem::Text(
                        "builderNodeDerivedAnchorName".into(),
                        "Name".into(),
                        "Derived attachment name.".into(),
                        name.clone(),
                        None,
                        false,
                    ));
                }
                BuilderNodeKind::ItemSlot { name, position }
                | BuilderNodeKind::MaterialSlot { name, position } => {
                    nodeui.add_item(TheNodeUIItem::Text(
                        "builderNodeSlotName".into(),
                        "Name".into(),
                        "Attachment slot name.".into(),
                        name.clone(),
                        None,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeSlotX".into(),
                        "Pos X".into(),
                        "Local X position.".into(),
                        position.x,
                        -0.5..=0.5,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeSlotY".into(),
                        "Pos Y".into(),
                        "Local Y position.".into(),
                        position.y,
                        -2.0..=4.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "builderNodeSlotZ".into(),
                        "Pos Z".into(),
                        "Local Z position.".into(),
                        position.z,
                        -0.5..=0.5,
                        false,
                    ));
                }
                BuilderNodeKind::Join => {}
                BuilderNodeKind::GeometryOutput { target, host_refs } => {
                    let selected = match target {
                        BuilderOutputTarget::Sector => 0,
                        BuilderOutputTarget::VertexPair => 1,
                        BuilderOutputTarget::Linedef => 2,
                    };
                    nodeui.add_item(TheNodeUIItem::Selector(
                        "builderNodeOutputTarget".into(),
                        "Target".into(),
                        "Host target for this builder graph.".into(),
                        vec!["Sector".into(), "Vertex Pair".into(), "Linedef".into()],
                        selected,
                    ));
                    nodeui.add_item(TheNodeUIItem::IntEditSlider(
                        "builderNodeOutputHostRefs".into(),
                        "Hosts".into(),
                        "How many host references this graph expects. Keep at 1 for now, use 2 for bridge/fence style assets, and higher later if needed."
                            .into(),
                        *host_refs as i32,
                        1..=8,
                        false,
                    ));
                }
            }
            nodeui.add_item(TheNodeUIItem::CloseTree);
        }

        if let Some(layout) = ui.get_text_layout(BUILDER_SETTINGS_LAYOUT) {
            layout.clear();
            nodeui.apply_to_text_layout(layout);
            ctx.ui.relayout = true;
        }
    }

    fn load_state_from_project(&mut self, project: &Project, server_ctx: &ServerContext) {
        self.active_builder_id = server_ctx.curr_builder_graph_id;
        if let Some(builder_id) = server_ctx.curr_builder_graph_id
            && let Some(asset) = project.builder_graphs.get(&builder_id)
            && let Ok(graph) = BuilderGraph::from_text(&asset.graph_data)
        {
            self.graph = graph;
            return;
        }
        self.graph = BuilderGraph::preset_table();
    }

    fn save_state_to_project(
        &self,
        project: &mut Project,
        server_ctx: &ServerContext,
        ctx: &mut TheContext,
    ) {
        let Some(builder_id) = self.active_builder_id.or(server_ctx.curr_builder_graph_id) else {
            return;
        };
        let Ok(graph_data) = self.graph.to_toml_string() else {
            return;
        };
        let graph_name = self.graph.name.clone();
        if let Some(asset) = project.builder_graphs.get_mut(&builder_id) {
            asset.graph_id = self.graph.id;
            asset.graph_name = graph_name.clone();
            asset.graph_data = graph_data.clone();
        }
        if let Some(map) = project.get_map_mut(server_ctx) {
            let spec = self.graph.output_spec();
            for sector_id in map.selected_sectors.clone() {
                if let Some(sector) = map.find_sector_mut(sector_id) {
                    let matches_builder = match sector.properties.get("builder_graph_id") {
                        Some(Value::Id(id)) => *id == builder_id,
                        _ => false,
                    };
                    if matches_builder {
                        sector
                            .properties
                            .set("builder_graph_name", Value::Str(graph_name.clone()));
                        sector
                            .properties
                            .set("builder_graph_data", Value::Str(graph_data.clone()));
                        sector
                            .properties
                            .set("builder_graph_target", Value::Str("sector".to_string()));
                        sector
                            .properties
                            .set("builder_surface_mode", Value::Str("overlay".to_string()));
                        sector
                            .properties
                            .set("builder_hide_host", Value::Bool(true));
                        sector
                            .properties
                            .set("builder_graph_host_refs", Value::Int(spec.host_refs as i32));
                    }
                }
                if map.get_surface_for_sector_id(sector_id).is_none() {
                    let mut surface = Surface::new(sector_id);
                    surface.calculate_geometry(map);
                    map.surfaces.insert(surface.id, surface);
                }
            }
            for vertex_id in map.selected_vertices.clone() {
                if let Some(vertex) = map.find_vertex_mut(vertex_id) {
                    let matches_builder = match vertex.properties.get("builder_graph_id") {
                        Some(Value::Id(id)) => *id == builder_id,
                        _ => false,
                    };
                    if matches_builder {
                        vertex
                            .properties
                            .set("builder_graph_name", Value::Str(graph_name.clone()));
                        vertex
                            .properties
                            .set("builder_graph_data", Value::Str(graph_data.clone()));
                        vertex.properties.set(
                            "builder_graph_target",
                            Value::Str("vertex_pair".to_string()),
                        );
                        vertex
                            .properties
                            .set("builder_graph_host_refs", Value::Int(spec.host_refs as i32));
                    }
                }
            }
            for linedef_id in map.selected_linedefs.clone() {
                if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                    let matches_builder = match linedef.properties.get("builder_graph_id") {
                        Some(Value::Id(id)) => *id == builder_id,
                        _ => false,
                    };
                    if matches_builder {
                        linedef
                            .properties
                            .set("builder_graph_name", Value::Str(graph_name.clone()));
                        linedef
                            .properties
                            .set("builder_graph_data", Value::Str(graph_data.clone()));
                        linedef
                            .properties
                            .set("builder_graph_target", Value::Str("linedef".to_string()));
                        linedef
                            .properties
                            .set("builder_graph_host_refs", Value::Int(spec.host_refs as i32));
                    }
                }
            }
        }
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Builder Graph Updated"),
            TheValue::Id(builder_id),
        ));
    }

    fn add_node(&mut self, kind: BuilderNodeKind, ui: &mut TheUI, ctx: &mut TheContext) {
        let next_id = self
            .graph
            .nodes
            .iter()
            .map(|node| node.id)
            .max()
            .unwrap_or(0)
            + 1;
        let pos = Vec2::new(
            self.graph.scroll_offset.x + 220,
            self.graph.scroll_offset.y + 80,
        );
        self.graph.nodes.push(BuilderNode {
            id: next_id,
            name: builder_node_name(&kind).to_string(),
            kind,
            pos,
            preview_collapsed: true,
        });
        self.graph.selected_node = Some(self.graph.nodes.len() - 1);
        self.sync_canvas(ui);
        self.set_selected_node_ui(ui, ctx);
        self.sync_rename_field(ui, ctx);
    }

    fn import_graph_file(&mut self, path: &std::path::Path) -> Result<(), String> {
        let source = fs::read_to_string(path)
            .map_err(|err| format!("failed to read {}: {err}", path.to_string_lossy()))?;
        let graph = BuilderGraph::from_text(&source)
            .map_err(|err| format!("failed to parse {}: {err}", path.to_string_lossy()))?;
        self.graph = graph;
        Ok(())
    }

    fn export_graph_file(&self, path: &std::path::Path) -> Result<std::path::PathBuf, String> {
        let graph_text = self.graph.to_toml_string()?;
        let mut output_path = path.to_path_buf();
        if output_path.extension().is_none() {
            output_path.set_extension("buildergraph");
        }
        fs::write(&output_path, graph_text)
            .map_err(|err| format!("failed to write {}: {err}", output_path.to_string_lossy()))?;
        Ok(output_path)
    }
}

impl Dock for BuilderEditorDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            graph: BuilderGraph::preset_table(),
            active_builder_id: None,
            categories: Self::default_categories(),
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);
        toolbar_hlayout.set_reverse_index(Some(2));

        let mut add_button = TheTraybarButton::new(TheId::named(BUILDER_ADD_BUTTON));
        add_button.set_text("Add".to_string());
        add_button.set_status_text("Add a builder node.");
        add_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Box".into(), TheId::named("Builder Add Box")),
                TheContextMenuItem::new(
                    "Sector Surface".into(),
                    TheId::named("Builder Add Sector Surface"),
                ),
                TheContextMenuItem::new(
                    "Linedef Surface".into(),
                    TheId::named("Builder Add Linedef Surface"),
                ),
                TheContextMenuItem::new(
                    "Vertex Point".into(),
                    TheId::named("Builder Add Vertex Point"),
                ),
                TheContextMenuItem::new("Offset".into(), TheId::named("Builder Add Offset")),
                TheContextMenuItem::new(
                    "Corner Layout".into(),
                    TheId::named("Builder Add Corner Layout"),
                ),
                TheContextMenuItem::new(
                    "Sector Corners".into(),
                    TheId::named("Builder Add Sector Corners"),
                ),
                TheContextMenuItem::new(
                    "Sector Grid".into(),
                    TheId::named("Builder Add Sector Grid"),
                ),
                TheContextMenuItem::new(
                    "Sector Edges".into(),
                    TheId::named("Builder Add Sector Edges"),
                ),
                TheContextMenuItem::new(
                    "Linedef Row".into(),
                    TheId::named("Builder Add Linedef Row"),
                ),
                TheContextMenuItem::new(
                    "Linedef Span".into(),
                    TheId::named("Builder Add Linedef Span"),
                ),
                TheContextMenuItem::new(
                    "Item Anchor".into(),
                    TheId::named("Builder Add Item Anchor"),
                ),
                TheContextMenuItem::new(
                    "Item Surface".into(),
                    TheId::named("Builder Add Item Surface"),
                ),
                TheContextMenuItem::new(
                    "Material Anchor".into(),
                    TheId::named("Builder Add Material Anchor"),
                ),
                TheContextMenuItem::new("Item Slot".into(), TheId::named("Builder Add Item Slot")),
                TheContextMenuItem::new(
                    "Material Slot".into(),
                    TheId::named("Builder Add Material Slot"),
                ),
                TheContextMenuItem::new("Join".into(), TheId::named("Builder Add Join")),
                TheContextMenuItem::new(
                    "Geometry Output".into(),
                    TheId::named("Builder Add Geometry Output"),
                ),
            ],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(add_button));

        let mut graph_button = TheTraybarButton::new(TheId::named(BUILDER_GRAPH_BUTTON));
        graph_button.set_text("Graph".to_string());
        graph_button.set_status_text("Import or export the current builder graph.");
        let mut graph_menu = TheContextMenu::default();
        graph_menu.add(TheContextMenuItem::new(
            "Import Graph...".to_string(),
            TheId::named("Builder Import Graph"),
        ));
        graph_menu.add(TheContextMenuItem::new(
            "Export Graph...".to_string(),
            TheId::named("Builder Export Graph"),
        ));
        graph_button.set_context_menu(Some(graph_menu));
        toolbar_hlayout.add_widget(Box::new(graph_button));

        let mut reset_button = TheTraybarButton::new(TheId::named(BUILDER_RESET_BUTTON));
        reset_button.set_text("Reset".to_string());
        reset_button.set_status_text("Reset to the default table builder graph.");
        toolbar_hlayout.add_widget(Box::new(reset_button));

        let mut name_edit = TheTextLineEdit::new(TheId::named("Builder Editor Name"));
        name_edit.set_text("Builder Graph".to_string());
        name_edit.set_status_text("Edit the current builder graph name.");
        name_edit.limiter_mut().set_max_width(160);
        toolbar_hlayout.add_widget(Box::new(name_edit));

        toolbar_canvas.set_layout(toolbar_hlayout);
        canvas.set_top(toolbar_canvas);

        let mut center = TheCanvas::new();

        let mut node_canvas = TheCanvas::new();
        node_canvas.set_widget(TheNodeCanvasView::new(TheId::named(BUILDER_CANVAS_VIEW)));
        center.set_center(node_canvas);

        let mut settings_canvas = TheCanvas::default();
        let mut text_layout = TheTextLayout::new(TheId::named(BUILDER_SETTINGS_LAYOUT));
        text_layout.limiter_mut().set_max_width(300);
        text_layout.set_text_margin(20);
        text_layout.set_text_align(TheHorizontalAlign::Right);
        settings_canvas.set_layout(text_layout);
        center.set_right(settings_canvas);

        canvas.set_center(center);

        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        self.load_state_from_project(project, server_ctx);
        self.sync_canvas(ui);
        self.set_selected_node_ui(ui, ctx);
        self.sync_rename_field(ui, ctx);
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match event {
            TheEvent::ContextMenuSelected(id, item) if id.name == BUILDER_ADD_BUTTON => {
                let kind = match item.name.as_str() {
                    "Builder Add Sector Surface" => BuilderNodeKind::SectorSurface,
                    "Builder Add Linedef Surface" => BuilderNodeKind::LinedefSurface,
                    "Builder Add Vertex Point" => BuilderNodeKind::VertexPoint,
                    "Builder Add Offset" => BuilderNodeKind::Offset {
                        translate: Vec3::zero(),
                    },
                    "Builder Add Corner Layout" => BuilderNodeKind::CornerLayout {
                        inset_x: 0.10,
                        inset_z: 0.10,
                    },
                    "Builder Add Box" => BuilderNodeKind::Box {
                        width: 1.0,
                        depth: 1.0,
                        height: 1.0,
                    },
                    "Builder Add Sector Corners" => BuilderNodeKind::SectorCorners {
                        inset_x: 0.10,
                        inset_z: 0.10,
                        elevation: 0.0,
                    },
                    "Builder Add Sector Grid" => BuilderNodeKind::SectorGrid {
                        columns: 2,
                        rows: 2,
                        inset_x: 0.0,
                        inset_z: 0.0,
                        elevation: 0.0,
                    },
                    "Builder Add Sector Edges" => BuilderNodeKind::SectorEdges {
                        north: true,
                        south: false,
                        east: false,
                        west: false,
                        inset: 0.0,
                        elevation: 0.0,
                    },
                    "Builder Add Linedef Row" => BuilderNodeKind::LinedefRow {
                        count: 2,
                        inset: 0.0,
                        elevation: 0.0,
                    },
                    "Builder Add Linedef Span" => BuilderNodeKind::LinedefSpan {
                        inset: 0.0,
                        elevation: 0.0,
                    },
                    "Builder Add Item Anchor" => BuilderNodeKind::ItemAnchor {
                        name: "item".to_string(),
                    },
                    "Builder Add Item Surface" => BuilderNodeKind::ItemSurface {
                        name: "surface".to_string(),
                    },
                    "Builder Add Material Anchor" => BuilderNodeKind::MaterialAnchor {
                        name: "TOP".to_string(),
                    },
                    "Builder Add Item Slot" => BuilderNodeKind::ItemSlot {
                        name: "item_slot".to_string(),
                        position: Vec3::zero(),
                    },
                    "Builder Add Material Slot" => BuilderNodeKind::MaterialSlot {
                        name: "material_slot".to_string(),
                        position: Vec3::zero(),
                    },
                    "Builder Add Join" => BuilderNodeKind::Join,
                    "Builder Add Geometry Output" => BuilderNodeKind::GeometryOutput {
                        target: BuilderOutputTarget::Sector,
                        host_refs: 1,
                    },
                    _ => return false,
                };
                self.add_node(kind, ui, ctx);
                self.save_state_to_project(project, server_ctx, ctx);
                true
            }
            TheEvent::ContextMenuSelected(id, item) if id.name == BUILDER_GRAPH_BUTTON => {
                if item.name == "Builder Import Graph" {
                    ctx.ui.open_file_requester(
                        TheId::named("Builder Import Graph File"),
                        "Import Builder Graph".into(),
                        TheFileExtension::new(
                            "Eldiron Builder Graph".into(),
                            vec!["buildergraph".to_string(), "json".to_string()],
                        ),
                    );
                    true
                } else if item.name == "Builder Export Graph" {
                    ctx.ui.save_file_requester(
                        TheId::named("Builder Export Graph File"),
                        "Export Builder Graph".into(),
                        TheFileExtension::new(
                            "Eldiron Builder Graph".into(),
                            vec!["buildergraph".to_string()],
                        ),
                    );
                    true
                } else {
                    false
                }
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == BUILDER_RESET_BUTTON =>
            {
                self.graph = BuilderGraph::preset_table();
                self.sync_canvas(ui);
                self.set_selected_node_ui(ui, ctx);
                self.sync_rename_field(ui, ctx);
                self.save_state_to_project(project, server_ctx, ctx);
                true
            }
            TheEvent::NodeSelectedIndexChanged(id, index) if id.name == BUILDER_CANVAS_VIEW => {
                self.graph.selected_node = *index;
                self.set_selected_node_ui(ui, ctx);
                self.sync_rename_field(ui, ctx);
                self.save_state_to_project(project, server_ctx, ctx);
                true
            }
            TheEvent::NodeDragged(id, index, position) if id.name == BUILDER_CANVAS_VIEW => {
                if let Some(node) = self.graph.nodes.get_mut(*index) {
                    node.pos = *position;
                    self.save_state_to_project(project, server_ctx, ctx);
                    return true;
                }
                false
            }
            TheEvent::NodeConnectionAdded(id, connections)
            | TheEvent::NodeConnectionRemoved(id, connections)
                if id.name == BUILDER_CANVAS_VIEW =>
            {
                self.graph.connections = self.canvas_connections_to_graph(connections);
                self.save_state_to_project(project, server_ctx, ctx);
                true
            }
            TheEvent::NodeDeleted(id, deleted_node_index, connections)
                if id.name == BUILDER_CANVAS_VIEW =>
            {
                if *deleted_node_index < self.graph.nodes.len() {
                    self.graph.nodes.remove(*deleted_node_index);
                    self.graph.connections = self.canvas_connections_to_graph(connections);
                    self.graph.selected_node = None;
                    self.sync_canvas(ui);
                    self.clear_selected_node_ui(ui, ctx);
                    self.sync_rename_field(ui, ctx);
                    self.save_state_to_project(project, server_ctx, ctx);
                    return true;
                }
                false
            }
            TheEvent::NodeViewScrolled(id, offset) if id.name == BUILDER_CANVAS_VIEW => {
                self.graph.scroll_offset = *offset;
                self.save_state_to_project(project, server_ctx, ctx);
                true
            }
            TheEvent::FileRequesterResult(id, paths)
                if id.name == "Builder Import Graph File" && !paths.is_empty() =>
            {
                if let Some(path) = paths.first() {
                    match self.import_graph_file(path) {
                        Ok(()) => {
                            self.sync_canvas(ui);
                            self.set_selected_node_ui(ui, ctx);
                            self.save_state_to_project(project, server_ctx, ctx);
                            self.sync_rename_field(ui, ctx);
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
                }
                true
            }
            TheEvent::FileRequesterResult(id, paths)
                if id.name == "Builder Export Graph File" && !paths.is_empty() =>
            {
                if let Some(path) = paths.first() {
                    match self.export_graph_file(path) {
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
                }
                true
            }
            TheEvent::Custom(id, _) if id.name == "Builder Selection Changed" => {
                self.load_state_from_project(project, server_ctx);
                self.sync_canvas(ui);
                self.set_selected_node_ui(ui, ctx);
                self.sync_rename_field(ui, ctx);
                true
            }
            TheEvent::ValueChanged(id, TheValue::Text(text))
                if id.name == "Builder Editor Name" =>
            {
                if let Some(index) = self.graph.selected_node
                    && let Some(node) = self.graph.nodes.get_mut(index)
                {
                    if matches!(node.kind, BuilderNodeKind::GeometryOutput { .. }) {
                        self.graph.name = text.clone();
                    } else {
                        node.name = text.clone();
                    }
                } else {
                    self.graph.name = text.clone();
                }
                self.sync_canvas(ui);
                self.save_state_to_project(project, server_ctx, ctx);
                true
            }
            TheEvent::ValueChanged(id, value) => {
                let mut changed = false;
                if let Some(index) = self.graph.selected_node
                    && let Some(node) = self.graph.nodes.get_mut(index)
                {
                    match (&mut node.kind, id.name.as_str(), value) {
                        (
                            BuilderNodeKind::Offset { translate, .. },
                            "builderNodeOffsetX",
                            TheValue::FloatRange(v, _),
                        ) => {
                            translate.x = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::Offset { translate, .. },
                            "builderNodeOffsetY",
                            TheValue::FloatRange(v, _),
                        ) => {
                            translate.y = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::Offset { translate, .. },
                            "builderNodeOffsetZ",
                            TheValue::FloatRange(v, _),
                        ) => {
                            translate.z = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::CornerLayout { inset_x, .. },
                            "builderNodeCornerLayoutInsetX",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *inset_x = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::CornerLayout { inset_z, .. },
                            "builderNodeCornerLayoutInsetZ",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *inset_z = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::Box { width, .. },
                            "builderNodeBoxWidth",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *width = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::Box { depth, .. },
                            "builderNodeBoxDepth",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *depth = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::Box { height, .. },
                            "builderNodeBoxHeight",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *height = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::GeometryOutput { target, .. },
                            "builderNodeOutputTarget",
                            TheValue::Int(v),
                        ) => {
                            *target = match *v {
                                1 => BuilderOutputTarget::VertexPair,
                                2 => BuilderOutputTarget::Linedef,
                                _ => BuilderOutputTarget::Sector,
                            };
                            changed = true;
                        }
                        (
                            BuilderNodeKind::GeometryOutput { host_refs, .. },
                            "builderNodeOutputHostRefs",
                            TheValue::IntRange(v, _),
                        ) => {
                            *host_refs = (*v).clamp(1, 8) as u8;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::SectorCorners { inset_x, .. },
                            "builderNodeCornersInsetX",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *inset_x = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::SectorCorners { inset_z, .. },
                            "builderNodeCornersInsetZ",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *inset_z = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::SectorCorners { elevation, .. },
                            "builderNodeCornersElevation",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *elevation = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::SectorGrid { columns, .. },
                            "builderNodeGridColumns",
                            TheValue::IntRange(v, _),
                        ) => {
                            *columns = (*v).clamp(1, 8) as u16;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::SectorGrid { rows, .. },
                            "builderNodeGridRows",
                            TheValue::IntRange(v, _),
                        ) => {
                            *rows = (*v).clamp(1, 8) as u16;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::SectorGrid { inset_x, .. },
                            "builderNodeGridInsetX",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *inset_x = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::SectorGrid { inset_z, .. },
                            "builderNodeGridInsetZ",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *inset_z = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::SectorGrid { elevation, .. },
                            "builderNodeGridElevation",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *elevation = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::SectorEdges { north, .. },
                            "builderNodeEdgesNorth",
                            TheValue::Bool(v),
                        ) => {
                            *north = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::SectorEdges { south, .. },
                            "builderNodeEdgesSouth",
                            TheValue::Bool(v),
                        ) => {
                            *south = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::SectorEdges { east, .. },
                            "builderNodeEdgesEast",
                            TheValue::Bool(v),
                        ) => {
                            *east = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::SectorEdges { west, .. },
                            "builderNodeEdgesWest",
                            TheValue::Bool(v),
                        ) => {
                            *west = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::SectorEdges { inset, .. },
                            "builderNodeEdgesInset",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *inset = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::SectorEdges { elevation, .. },
                            "builderNodeEdgesElevation",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *elevation = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::LinedefRow { count, .. },
                            "builderNodeLinedefCount",
                            TheValue::IntRange(v, _),
                        ) => {
                            *count = (*v).clamp(1, 16) as u16;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::LinedefRow { inset, .. },
                            "builderNodeLinedefInset",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *inset = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::LinedefRow { elevation, .. },
                            "builderNodeLinedefElevation",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *elevation = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::LinedefSpan { inset, .. },
                            "builderNodeLinedefSpanInset",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *inset = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::LinedefSpan { elevation, .. },
                            "builderNodeLinedefSpanElevation",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *elevation = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::ItemAnchor { name },
                            "builderNodeDerivedAnchorName",
                            TheValue::Text(v),
                        ) => {
                            *name = v.clone();
                            changed = true;
                        }
                        (
                            BuilderNodeKind::ItemSurface { name },
                            "builderNodeDerivedAnchorName",
                            TheValue::Text(v),
                        ) => {
                            *name = v.clone();
                            changed = true;
                        }
                        (
                            BuilderNodeKind::MaterialAnchor { name },
                            "builderNodeDerivedAnchorName",
                            TheValue::Text(v),
                        ) => {
                            *name = v.clone();
                            changed = true;
                        }
                        (
                            BuilderNodeKind::ItemSlot { name, .. },
                            "builderNodeSlotName",
                            TheValue::Text(v),
                        ) => {
                            *name = v.clone();
                            changed = true;
                        }
                        (
                            BuilderNodeKind::MaterialSlot { name, .. },
                            "builderNodeSlotName",
                            TheValue::Text(v),
                        ) => {
                            *name = v.clone();
                            changed = true;
                        }
                        (
                            BuilderNodeKind::ItemSlot { position, .. },
                            "builderNodeSlotX",
                            TheValue::FloatRange(v, _),
                        ) => {
                            position.x = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::MaterialSlot { position, .. },
                            "builderNodeSlotX",
                            TheValue::FloatRange(v, _),
                        ) => {
                            position.x = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::ItemSlot { position, .. },
                            "builderNodeSlotY",
                            TheValue::FloatRange(v, _),
                        ) => {
                            position.y = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::MaterialSlot { position, .. },
                            "builderNodeSlotY",
                            TheValue::FloatRange(v, _),
                        ) => {
                            position.y = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::ItemSlot { position, .. },
                            "builderNodeSlotZ",
                            TheValue::FloatRange(v, _),
                        ) => {
                            position.z = *v;
                            changed = true;
                        }
                        (
                            BuilderNodeKind::MaterialSlot { position, .. },
                            "builderNodeSlotZ",
                            TheValue::FloatRange(v, _),
                        ) => {
                            position.z = *v;
                            changed = true;
                        }
                        _ => {}
                    }
                }
                if changed {
                    self.sync_canvas(ui);
                    self.set_selected_node_ui(ui, ctx);
                    self.save_state_to_project(project, server_ctx, ctx);
                    return true;
                }
                false
            }
            _ => false,
        }
    }
}
