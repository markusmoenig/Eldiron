use crate::editor::{RUSTERIX, UNDOMANAGER};
use crate::prelude::*;
use shared::prelude::*;

use ShapeFXParam::*;
use rusterix::{
    Assets, ShapeFX, ShapeFXGraph, ShapeFXParam, ShapeFXRole, ShapeStack, Texture, Value,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeContext {
    Region,
    Shape,
    Material,
    GlobalRender,
    Screen,
}

use NodeContext::*;

pub struct NodeEditor {
    pub context: NodeContext,
    pub graph: ShapeFXGraph,

    pub categories: FxHashMap<String, TheColor>,
}

#[allow(clippy::new_without_default)]
impl NodeEditor {
    pub fn new() -> Self {
        let mut categories: FxHashMap<String, TheColor> = FxHashMap::default();
        categories.insert("ShapeFX".into(), TheColor::from("#c49a00")); // Warm gold — represents pixel-level artistic material control
        categories.insert("Render".into(), TheColor::from("#e53935")); // Bright red — strong signal for core rendering pipeline
        categories.insert("Modifier".into(), TheColor::from("#00bfa5")); // Teal green — evokes transformation, procedural mesh edits
        categories.insert("FX".into(), TheColor::from("#7e57c2")); // Purple — expressive and magical for particles, screen fx, etc.
        categories.insert("Shape".into(), TheColor::from("#4285F4")); // Vivid blue — represents structure, geometric form, and stability

        Self {
            context: NodeContext::Region,
            graph: ShapeFXGraph::default(),
            categories,
        }
    }

    /// Set the context of the node editor.
    pub fn set_context(
        &mut self,
        context: NodeContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        // println!("NodeContext {:?}", context);
        self.context = context;
        if context == GlobalRender {
            if project.render_graph.nodes.is_empty() {
                project.render_graph = ShapeFXGraph {
                    nodes: vec![ShapeFX::new(ShapeFXRole::Render)],
                    ..Default::default()
                };
            }
            self.graph = project.render_graph.clone();
        } else {
            self.graph = ShapeFXGraph::default();
        }
        let canvas = self.to_canvas();
        ui.set_node_canvas("ShapeFX NodeCanvas", canvas);
        self.graph_changed(project, ui, ctx, server_ctx);

        ctx.ui.send(TheEvent::Custom(
            TheId::named_with_id("Nodegraph Id Changed", self.graph.id),
            TheValue::Empty,
        ));
    }

    /// Activates the given graph in the editor
    pub fn apply_graph(
        &mut self,
        context: NodeContext,
        graph: &ShapeFXGraph,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        // println!("Apply Graph {:?}", context);
        self.context = context;
        self.graph = graph.clone();
        self.graph.selected_node = None;
        let canvas = self.to_canvas();
        ui.set_node_canvas("ShapeFX NodeCanvas", canvas);

        ctx.ui.send(TheEvent::Custom(
            TheId::named_with_id("Nodegraph Id Changed", graph.id),
            TheValue::Empty,
        ));
    }

    /// Called when the graph has changed, updating the UI and providing undo.
    fn graph_changed(
        &mut self,
        project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        // println!("Graph Changed {:?}", self.context);

        if self.context == NodeContext::GlobalRender {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.client.global = self.graph.clone();
            project.render_graph = self.graph.clone();
        } else if self.context == NodeContext::Region {
            if let Some(map) = project.get_map_mut(server_ctx) {
                map.changed += 1;
                map.shapefx_graphs.insert(self.graph.id, self.graph.clone());
                map.terrain.mark_dirty();
                RUSTERIX.write().unwrap().set_dirty();

                // Reset the background renderer
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Render SceneManager Map"),
                    TheValue::Empty,
                ));
            }
        }
        /*else if self.context == NodeContext::Material {
            if let Some(map) = project.get_map_mut(server_ctx) {
                let prev = map.clone();
                map.changed += 1;
                map.shapefx_graphs.insert(self.graph.id, self.graph.clone());
                let undo = MaterialUndoAtom::MapEdit(Box::new(prev), Box::new(map.clone()));
                UNDOMANAGER.write().unwrap().add_material_undo(undo, ctx);
                self.create_material_preview(map, &RUSTERIX.read().unwrap().assets);

                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Materialpicker"),
                    TheValue::Empty,
                ));
            }
        }*/
        else if self.context == NodeContext::Shape {
            if let Some(map) = project.get_map_mut(server_ctx) {
                let prev = map.clone();
                map.changed += 1;
                map.shapefx_graphs.insert(self.graph.id, self.graph.clone());
                let undo = RegionUndoAtom::MapEdit(Box::new(prev), Box::new(map.clone()));
                UNDOMANAGER
                    .write()
                    .unwrap()
                    .add_region_undo(&map.id, undo, ctx);
                self.create_shape_preview(map, &RUSTERIX.read().unwrap().assets);
            }
        }
    }

    pub fn build(&mut self) -> TheCanvas {
        let mut center = TheCanvas::new();

        // Toolbar
        let mut top_toolbar = TheCanvas::new();
        top_toolbar.set_widget(TheTraybar::new(TheId::empty()));

        let mut toolbar_hlayout = TheHLayout::new(TheId::named("Material Tool Params"));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(10, 4, 5, 4));

        let mut id_text = TheText::new(TheId::named("Graph Id Text"));
        id_text.set_fixed_size_text("(---)".into());
        id_text.set_status_text(&fl!("status_node_editor_graph_id"));
        id_text.set_text("(--)".to_string());
        toolbar_hlayout.add_widget(Box::new(id_text));

        let mut create_button = TheTraybarButton::new(TheId::named("Create Graph Button"));
        create_button.set_status_text(&fl!("status_node_editor_create_button"));
        create_button.set_text(fl!("node_editor_create_button"));
        toolbar_hlayout.add_widget(Box::new(create_button));

        let mut fx_nodes_button = TheTraybarButton::new(TheId::named("FX Nodes"));
        fx_nodes_button.set_custom_color(self.categories.get("FX").cloned());
        fx_nodes_button.set_text(str!("FX"));
        fx_nodes_button.set_status_text(&fl!("status_node_editor_fx_node_button"));
        fx_nodes_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Material".to_string(), TheId::named("Material")),
                TheContextMenuItem::new("Point Light".to_string(), TheId::named("Point Light")),
            ],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(fx_nodes_button));

        let mut render_nodes_button = TheTraybarButton::new(TheId::named("Render Nodes"));
        render_nodes_button.set_custom_color(self.categories.get("Render").cloned());
        render_nodes_button.set_text(str!("Render"));
        render_nodes_button.set_status_text(&fl!("status_node_editor_render_nodes_button"));
        render_nodes_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Lights".to_string(), TheId::named("Lights")),
                TheContextMenuItem::new("Fog".to_string(), TheId::named("Fog")),
                TheContextMenuItem::new("Sky".to_string(), TheId::named("Sky")),
            ],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(render_nodes_button));

        let mut mesh_nodes_button = TheTraybarButton::new(TheId::named("Modifier Nodes"));
        mesh_nodes_button.set_custom_color(self.categories.get("Modifier").cloned());
        mesh_nodes_button.set_text(str!("Modifier"));
        mesh_nodes_button.set_status_text(&fl!("status_node_editor_mesh_nodes_button"));
        mesh_nodes_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Terrain: Colorize".to_string(), TheId::named("Colorize")),
                TheContextMenuItem::new("Terrain: Flatten".to_string(), TheId::named("Flatten")),
            ],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(mesh_nodes_button));

        /*
        let mut shape_nodes_button = TheTraybarButton::new(TheId::named("Shape Nodes"));
        shape_nodes_button.set_custom_color(self.categories.get("Shape").cloned());
        shape_nodes_button.set_text(str!("Shape"));
        shape_nodes_button.set_status_text("Nodes which attach to geometry and create shapes.");
        shape_nodes_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Circle".to_string(), TheId::named("Circle")),
                TheContextMenuItem::new("Line".to_string(), TheId::named("Line")),
                TheContextMenuItem::new("Box".to_string(), TheId::named("Box")),
            ],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(shape_nodes_button));
        */

        let mut shapefx_nodes_button = TheTraybarButton::new(TheId::named("ShapeFX Nodes"));
        shapefx_nodes_button.set_custom_color(self.categories.get("ShapeFX").cloned());
        shapefx_nodes_button.set_text(str!("Shape FX"));
        shapefx_nodes_button.set_status_text(&fl!("status_node_editor_shapefx_nodes_button"));
        shapefx_nodes_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Color".to_string(), TheId::named("Color")),
                TheContextMenuItem::new("Gradient".to_string(), TheId::named("Gradient")),
                TheContextMenuItem::new("Outline".to_string(), TheId::named("Outline")),
                TheContextMenuItem::new("Glow".to_string(), TheId::named("Glow")),
                TheContextMenuItem::new("Noise Overlay".to_string(), TheId::named("Noise Overlay")),
                TheContextMenuItem::new("Wood".to_string(), TheId::named("Wood")),
                TheContextMenuItem::new("Stone".to_string(), TheId::named("Stone")),
            ],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(shapefx_nodes_button));

        toolbar_hlayout.set_reverse_index(Some(4));
        top_toolbar.set_layout(toolbar_hlayout);
        center.set_top(top_toolbar);

        let mut material_node_canvas = TheCanvas::new();
        let node_view = TheNodeCanvasView::new(TheId::named("ShapeFX NodeCanvas"));
        material_node_canvas.set_widget(node_view);

        center.set_center(material_node_canvas);

        center
    }

    pub fn to_canvas(&mut self) -> TheNodeCanvas {
        let mut canvas = TheNodeCanvas {
            node_width: 136,
            selected_node: self.graph.selected_node,
            offset: self.graph.scroll_offset,
            connections: self.graph.connections.clone(),
            categories: self.categories.clone(),
            ..Default::default()
        };

        for (index, node) in self.graph.nodes.iter().enumerate() {
            let n = TheNode {
                name: node.name(),
                position: node.position,
                inputs: node.inputs(),
                outputs: node.outputs(),
                preview: TheRGBABuffer::default(),
                supports_preview: false,
                preview_is_open: false,
                can_be_deleted: index != 0,
            };
            canvas.nodes.push(n);
        }

        canvas
    }

    #[allow(clippy::too_many_arguments)]
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
        #[allow(clippy::single_match)]
        match event {
            TheEvent::ContextMenuSelected(id, item) => {
                if (id.name == "ShapeFX Nodes"
                    || id.name == "Modifier Nodes"
                    || id.name == "Render Nodes"
                    || id.name == "FX Nodes"
                    || id.name == "Shape Nodes")
                    && !self.graph.nodes.is_empty()
                {
                    if let Ok(role) = item.name.parse::<ShapeFXRole>() {
                        let mut effect = ShapeFX::new(role);

                        effect.position = Vec2::new(
                            self.graph.scroll_offset.x + 220,
                            self.graph.scroll_offset.y + 10,
                        );
                        self.graph.nodes.push(effect);
                        self.graph.selected_node = Some(self.graph.nodes.len() - 1);

                        let canvas = self.to_canvas();
                        ui.set_node_canvas("ShapeFX NodeCanvas", canvas);

                        if let Some(map) = project.get_map_mut(server_ctx) {
                            map.shapefx_graphs.insert(self.graph.id, self.graph.clone());
                        }
                        self.set_selected_node_ui(project, ui, ctx, true);
                    }
                }
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) => {
                if id.name == "Create Graph Button" {
                    // println!("{:?}", server_ctx.curr_map_context);
                    //

                    if server_ctx.get_map_context() == MapContext::Screen {
                        {
                            self.graph = ShapeFXGraph {
                                nodes: vec![ShapeFX::new(ShapeFXRole::Widget)],
                                ..Default::default()
                            };
                            self.context = NodeContext::Screen;
                        }
                    } else if server_ctx.get_map_context() == MapContext::Shader
                        || server_ctx.profile_view.is_some()
                    {
                        {
                            self.graph = ShapeFXGraph {
                                nodes: vec![ShapeFX::new(ShapeFXRole::MaterialGeometry)],
                                ..Default::default()
                            };
                            self.context = NodeContext::Material;
                        }
                    } else if server_ctx.get_map_context() == MapContext::Character
                        || server_ctx.get_map_context() == MapContext::Item
                    {
                        self.graph = ShapeFXGraph {
                            nodes: vec![ShapeFX::new(ShapeFXRole::MaterialGeometry)],
                            ..Default::default()
                        };
                        self.context = NodeContext::Shape;
                    } else if self.context == NodeContext::Region {
                        if server_ctx.curr_map_tool_type == MapToolType::Sector {
                            self.graph = ShapeFXGraph {
                                nodes: vec![ShapeFX::new(ShapeFXRole::SectorGeometry)],
                                ..Default::default()
                            };
                        } else if server_ctx.curr_map_tool_type == MapToolType::Linedef {
                            self.graph = ShapeFXGraph {
                                nodes: vec![ShapeFX::new(ShapeFXRole::LinedefGeometry)],
                                ..Default::default()
                            };
                        }
                        self.context = NodeContext::Region;
                    }

                    let canvas = self.to_canvas();
                    ui.set_node_canvas("ShapeFX NodeCanvas", canvas);
                    self.graph_changed(project, ui, ctx, server_ctx);
                }
            }
            TheEvent::NodeSelectedIndexChanged(id, index) => {
                if id.name == "ShapeFX NodeCanvas" {
                    self.graph.selected_node = *index;
                    self.set_selected_node_ui(project, ui, ctx, true);
                    if let Some(map) = project.get_map_mut(server_ctx) {
                        map.changed += 1;
                        map.shapefx_graphs.insert(self.graph.id, self.graph.clone());
                    }
                }
            }
            TheEvent::NodeDragged(id, index, position) => {
                if id.name == "ShapeFX NodeCanvas" {
                    self.graph.nodes[*index].position = *position;
                    if let Some(map) = project.get_map_mut(server_ctx) {
                        // let prev = map.clone();
                        map.changed += 1;
                        map.shapefx_graphs.insert(self.graph.id, self.graph.clone());
                        // let undo = MaterialUndoAtom::MapEdit(Box::new(prev), Box::new(map.clone()));
                        // UNDOMANAGER.write().unwrap().add_material_undo(undo, ctx);
                    }
                }
            }
            TheEvent::NodeConnectionAdded(id, connections)
            | TheEvent::NodeConnectionRemoved(id, connections) => {
                if id.name == "ShapeFX NodeCanvas" {
                    self.graph.connections.clone_from(connections);
                    self.graph_changed(project, ui, ctx, server_ctx);
                }
            }
            TheEvent::NodeDeleted(id, deleted_node_index, connections) => {
                if id.name == "ShapeFX NodeCanvas" {
                    self.graph.nodes.remove(*deleted_node_index);
                    self.graph.connections.clone_from(connections);

                    self.graph_changed(project, ui, ctx, server_ctx);
                }
            }
            TheEvent::NodeViewScrolled(id, offset) => {
                if id.name == "ShapeFX NodeCanvas" {
                    self.graph.scroll_offset = *offset;
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name.starts_with("shapefx") {
                    let snake_case = self.transform_to_snake_case(&id.name, "shapefx");
                    if let Some(index) = self.graph.selected_node {
                        if let Some(node) = self.graph.nodes.get_mut(index) {
                            match value {
                                TheValue::FloatRange(v, _) => {
                                    node.values.set(&snake_case, rusterix::Value::Float(*v))
                                }
                                TheValue::IntRange(v, _) => {
                                    node.values.set(&snake_case, rusterix::Value::Int(*v))
                                }
                                TheValue::Int(v) => {
                                    node.values.set(&snake_case, rusterix::Value::Int(*v))
                                }
                                TheValue::ColorObject(v) => node
                                    .values
                                    .set(&snake_case, rusterix::Value::Color(v.clone())),
                                _ => {}
                            }
                        }
                        self.graph_changed(project, ui, ctx, server_ctx);
                    }
                }
            }
            _ => {}
        }

        redraw
    }

    /// Create the UI for the selected node.
    pub fn set_selected_node_ui(
        &mut self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        switch_to_nodes: bool,
    ) {
        let mut nodeui = TheNodeUI::default();
        let mut node_name = "Node".to_string();

        if let Some(index) = self.graph.selected_node {
            if let Some(node) = self.graph.nodes.get(index) {
                node_name = node.name();
                for param in node.params() {
                    match param {
                        Float(id, name, status, value, range) => {
                            let item = TheNodeUIItem::FloatEditSlider(
                                format!(
                                    "shapefx{}",
                                    id.get(0..1).unwrap_or("").to_uppercase()
                                        + id.get(1..).unwrap_or("")
                                ),
                                name.clone(),
                                status.clone(),
                                value,
                                range,
                                false,
                            );
                            nodeui.add_item(item);
                        }
                        Int(id, name, status, value, range) => {
                            let item = TheNodeUIItem::IntEditSlider(
                                format!(
                                    "shapefx{}",
                                    id.get(0..1).unwrap_or("").to_uppercase()
                                        + id.get(1..).unwrap_or("")
                                ),
                                name.clone(),
                                status.clone(),
                                value,
                                range,
                                false,
                            );
                            nodeui.add_item(item);
                        }
                        Color(id, name, status, value) => {
                            let item = TheNodeUIItem::ColorPicker(
                                format!(
                                    "shapefx{}",
                                    id.get(0..1).unwrap_or("").to_uppercase()
                                        + id.get(1..).unwrap_or("")
                                ),
                                name.clone(),
                                status.clone(),
                                value,
                                false,
                            );
                            nodeui.add_item(item);
                        }
                        PaletteIndex(id, name, status, value) => {
                            let item = TheNodeUIItem::PaletteSlider(
                                format!(
                                    "shapefx{}",
                                    id.get(0..1).unwrap_or("").to_uppercase()
                                        + id.get(1..).unwrap_or("")
                                ),
                                name.clone(),
                                status.clone(),
                                value,
                                project.palette.clone(),
                                false,
                            );
                            nodeui.add_item(item);
                        }
                        Selector(id, name, status, options, value) => {
                            let item = TheNodeUIItem::Selector(
                                format!(
                                    "shapefx{}",
                                    id.get(0..1).unwrap_or("").to_uppercase()
                                        + id.get(1..).unwrap_or("")
                                ),
                                name.clone(),
                                status.clone(),
                                options.clone(),
                                value,
                            );
                            nodeui.add_item(item);
                        }
                    }
                }
            }
        }

        if let Some(layout) = ui.get_text_layout("Node Settings") {
            nodeui.apply_to_text_layout(layout);
            // layout.relayout(ctx);
            ctx.ui.relayout = true;

            if switch_to_nodes {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Show Node Settings"),
                    TheValue::Text(format!("{node_name} Settings")),
                ));
            }
        }
    }

    /// Create a preview for the shape and stores it in the map
    pub fn create_shape_preview(&self, map: &mut Map, assets: &Assets) {
        let size = 128;
        let mut texture = Texture::alloc(size as usize, size as usize);

        let mut stack = ShapeStack::new(Vec2::new(-5.0, -5.0), Vec2::new(5.0, 5.0));
        stack.render_geometry(&mut texture, map, assets, false, &FxHashMap::default());

        map.properties.set("shape", Value::Texture(texture));
    }

    /// Create a preview for the material and stores it in the map
    pub fn create_material_preview(&self, map: &mut Map, assets: &Assets) {
        let size = 128;
        let mut texture = Texture::alloc(size as usize, size as usize);

        let mut stack = ShapeStack::new(Vec2::new(-5.0, -5.0), Vec2::new(5.0, 5.0));
        stack.render_geometry(&mut texture, map, assets, true, &FxHashMap::default());

        map.properties.set("material", Value::Texture(texture));
    }

    pub fn force_update(&self, ctx: &mut TheContext, map: &mut Map) {
        if self.context == NodeContext::Shape {
            self.create_shape_preview(map, &RUSTERIX.read().unwrap().assets);
        } else if self.context == NodeContext::Material {
            self.create_material_preview(map, &RUSTERIX.read().unwrap().assets);

            ctx.ui.send(TheEvent::Custom(
                TheId::named("Update Materialpicker"),
                TheValue::Empty,
            ));
        }
    }

    fn transform_to_snake_case(&self, input: &str, strip_prefix: &str) -> String {
        // Strip the prefix if it exists
        let stripped = if let Some(remainder) = input.strip_prefix(strip_prefix) {
            remainder
        } else {
            input
        };

        // Convert to snake_case
        let mut snake_case = String::new();
        for (i, c) in stripped.chars().enumerate() {
            if c.is_uppercase() {
                // Add an underscore before uppercase letters (except for the first character)
                if i > 0 {
                    snake_case.push('_');
                }
                snake_case.push(c.to_ascii_lowercase());
            } else {
                snake_case.push(c);
            }
        }

        snake_case
    }
}
