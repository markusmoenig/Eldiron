// use crate::editor::UNDOMANAGER;
use crate::prelude::*;
use shared::prelude::*;

use rusterix::{ShapeFX, ShapeFXGraph, ShapeFXRole};

pub struct NodeEditor {
    material_start_index: i32,

    pub graph: ShapeFXGraph,
}

#[allow(clippy::new_without_default)]
impl NodeEditor {
    pub fn new() -> Self {
        Self {
            material_start_index: 0,

            graph: ShapeFXGraph::default(),
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

        /*
        for i in 0..20 {
            let mut icon = TheIconView::new(TheId::named(&format!("Material Icon #{}", i)));
            // ground_icon.set_text(Some("FLOOR".to_string()));
            // ground_icon.set_text_size(10.0);
            // ground_icon.set_text_color([200, 200, 200, 255]);
            icon.limiter_mut().set_max_size(Vec2::new(20, 20));
            icon.set_border_color(Some(BLACK));

            toolbar_hlayout.add_widget(Box::new(icon));
        }*/

        let mut create_button = TheTraybarButton::new(TheId::named("Create Graph Button"));
        create_button.set_status_text("Apply the source to the selected geometry.");
        create_button.set_text("Create Graph".to_string());
        toolbar_hlayout.add_widget(Box::new(create_button));

        let mut nodes_button = TheTraybarButton::new(TheId::named("ShapeFX Nodes"));
        //add_button.set_icon_name("icon_role_add".to_string());
        nodes_button.set_text(str!("Nodes"));
        nodes_button.set_status_text("Available region effect nodes.");
        nodes_button.set_context_menu(Some(TheContextMenu {
            items: vec![TheContextMenuItem::new(
                "Gradient".to_string(),
                TheId::named("Gradient"),
            )],
            ..Default::default()
        }));

        // let mut nodes_drop_down = TheDropdownMenu::new(TheId::named("Nodes Selector"));
        // for role in ShapeFXRole::iterator() {
        //     if role != ShapeFXRole::Geometry {
        //         nodes_drop_down.add_option(role.to_string());
        //     }
        // }
        toolbar_hlayout.add_widget(Box::new(nodes_button));

        toolbar_hlayout.set_reverse_index(Some(2));
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
            ..Default::default()
        };

        for (index, node) in self.graph.effects.iter().enumerate() {
            let n = TheNode {
                name: node.name(),
                position: node.position,
                inputs: node.inputs(),
                outputs: node.outputs(),
                preview: TheRGBABuffer::default(), //node.preview.clone(),
                supports_preview: false,           //node.supports_preview,
                preview_is_open: false,            //node.preview_is_open,
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
            /*
            TheEvent::Custom(id, _) => {
                if id.name == "Update Material Previews" {
                    for i in 0..20 {
                        if let Some(icon_view) = ui.get_icon_view(&format!("Material Icon #{}", i))
                        {
                            let index = self.material_start_index + i;
                            if let Some((_, material)) = project.materials.get_index(index as usize)
                            {
                                icon_view.set_rgba_tile(TheRGBATile::buffer(
                                    material.get_preview().scaled(20, 20),
                                ));
                            }
                        }
                    }
                } else if id.name == "Map Selection Changed" {
                    if let Some(map) = project.get_map(server_ctx) {
                        if let Some(rc) =
                            server_ctx.get_texture_for_mode(server_ctx.curr_texture_mode, map)
                        {
                            if let Some(material_index) = rc.1 {
                                self.set_material_selection(
                                    ui,
                                    ctx,
                                    project,
                                    server_ctx,
                                    Some(material_index),
                                );
                            } else {
                                self.set_material_selection(ui, ctx, project, server_ctx, None);
                            }
                        } else {
                            self.set_material_selection(ui, ctx, project, server_ctx, None);
                        }
                    }
                }
            }*/
            TheEvent::ContextMenuSelected(id, item) => {
                #[allow(clippy::collapsible_if)]
                if id.name == "ShapeFX Nodes" && !self.graph.effects.is_empty() {
                    if let Ok(role) = item.name.parse::<ShapeFXRole>() {
                        let mut effect = ShapeFX::new(role);

                        effect.position = Vec2::new(
                            self.graph.scroll_offset.x + 220,
                            self.graph.scroll_offset.y + 10,
                        );
                        self.graph.effects.push(effect);
                        self.graph.selected_node = Some(self.graph.effects.len() - 1);

                        let canvas = self.to_canvas();
                        ui.set_node_canvas("ShapeFX NodeCanvas", canvas);

                        if let Some(map) = project.get_map_mut(server_ctx) {
                            map.effect_graphs.insert(self.graph.id, self.graph.clone());
                        }
                    }
                }
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) => {
                if id.name == "Create Graph Button" {
                    self.graph = ShapeFXGraph {
                        effects: vec![ShapeFX::new(ShapeFXRole::Geometry)],
                        ..Default::default()
                    };

                    let canvas = self.to_canvas();
                    ui.set_node_canvas("ShapeFX NodeCanvas", canvas);

                    if let Some(map) = project.get_map_mut(server_ctx) {
                        map.effect_graphs.insert(self.graph.id, self.graph.clone());
                    }
                }
            }
            /*
            if id.name.starts_with("Material Icon #") {
                let index_str = id.name.replace("Material Icon #", "");
                if let Ok(index) = index_str.parse::<i32>() {
                    let index = (index + self.material_start_index) as u8;
                    // if let Some((_, material)) = project.materials.get_index_mut(index as usize)
                    // {
                    //     let node_canvas = material.to_canvas(&project.palette);
                    //     ui.set_node_canvas("Map NodeCanvas", node_canvas);
                    // }
                    if let Some(map) = project.get_map_mut(server_ctx) {
                        let prev = map.clone();

                        match server_ctx.curr_texture_mode {
                            MapTextureMode::Floor => {
                                for sector_id in &map.selected_sectors.clone() {
                                    if let Some(sector) = map.find_sector_mut(*sector_id) {
                                        sector.floor_material = Some(index);
                                        sector.floor_texture = None;
                                    }
                                }
                            }
                            MapTextureMode::Wall => {
                                let mut linedef_ids = Vec::new();
                                for sector_id in &map.selected_sectors {
                                    if let Some(sector) = map.find_sector(*sector_id) {
                                        linedef_ids.extend(&sector.linedefs);
                                    }
                                }

                                for linedef_id in &map.selected_linedefs {
                                    if !linedef_ids.contains(linedef_id) {
                                        linedef_ids.push(*linedef_id);
                                    }
                                }

                                for linedef_id in linedef_ids {
                                    if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                                        linedef.material = Some(index);
                                        linedef.texture = None;
                                    }
                                }
                            }
                            MapTextureMode::Ceiling => {
                                for sector_id in &map.selected_sectors.clone() {
                                    if let Some(sector) = map.find_sector_mut(*sector_id) {
                                        sector.ceiling_material = Some(index);
                                        sector.ceiling_texture = None;
                                    }
                                }
                            }
                            _ => {}
                        }

                        let undo =
                            RegionUndoAtom::MapEdit(Box::new(prev), Box::new(map.clone()));

                        UNDOMANAGER.lock().unwrap().add_region_undo(
                            &server_ctx.curr_region,
                            undo,
                            ctx,
                        );

                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Minimap"),
                            TheValue::Empty,
                        ));

                        self.set_material_selection(ui, ctx, project, server_ctx, Some(index));
                    }
                }
                if let Some(region) = project.get_region_ctx(server_ctx) {
                    server.update_region(region);
                }
            }*/
            // }
            TheEvent::NodeSelectedIndexChanged(id, index) => {
                if id.name == "ShapeFX NodeCanvas" {
                    self.graph.selected_node = *index;
                    self.set_selected_material_node_ui(server_ctx, project, ui, ctx, true);
                    if let Some(map) = project.get_map_mut(server_ctx) {
                        map.effect_graphs.insert(self.graph.id, self.graph.clone());
                    }
                }
            }
            TheEvent::NodeDragged(id, index, position) => {
                if id.name == "ShapeFX NodeCanvas" {
                    self.graph.effects[*index].position = *position;
                    if let Some(map) = project.get_map_mut(server_ctx) {
                        map.effect_graphs.insert(self.graph.id, self.graph.clone());
                    }
                }
            }
            TheEvent::NodeConnectionAdded(id, connections)
            | TheEvent::NodeConnectionRemoved(id, connections) => {
                if id.name == "ShapeFX NodeCanvas" {
                    self.graph.connections.clone_from(connections);
                    if let Some(map) = project.get_map_mut(server_ctx) {
                        map.effect_graphs.insert(self.graph.id, self.graph.clone());
                    }
                }
                //     if let Some(material_id) = server_ctx.curr_material {
                //         if let Some(material) = project.materials.get_mut(&material_id) {
                //             let prev = material.to_json();
                //             material.connections.clone_from(connections);
                //             material.render_preview(&project.palette, &TEXTURES.lock().unwrap());
                //             ui.set_node_preview("Map NodeCanvas", 0, material.get_preview());
                //             let undo =
                //                 MaterialFXUndoAtom::Edit(material.id, prev, material.to_json());
                //             UNDOMANAGER.lock().unwrap().add_materialfx_undo(undo, ctx);
                //             redraw = true;
                //         }
                //         MAPRENDER.lock().unwrap().set_materials(project);
                //     }
                // }
            }
            TheEvent::NodeDeleted(id, deleted_node_index, connections) => {
                if id.name == "ShapeFX NodeCanvas" {
                    self.graph.effects.remove(*deleted_node_index);
                    self.graph.connections.clone_from(connections);
                }
                // if id.name == "Map NodeCanvas" {
                //     if let Some(material_id) = server_ctx.curr_material {
                //         if let Some(material) = project.materials.get_mut(&material_id) {
                //             let prev = material.to_json();
                //             material.nodes.remove(*deleted_node_index);
                //             //material.node_previews.remove(*deleted_node_index);
                //             material.connections.clone_from(connections);
                //             material.selected_node = None;
                //             material.render_preview(&project.palette, &TEXTURES.lock().unwrap());
                //             ui.set_node_preview(
                //                 "MaterialFX NodeCanvas",
                //                 0,
                //                 material.get_preview().clone(),
                //             );
                //             let undo =
                //                 MaterialFXUndoAtom::Edit(material.id, prev, material.to_json());
                //             UNDOMANAGER.lock().unwrap().add_materialfx_undo(undo, ctx);
                //             redraw = true;
                //         }
                //         MAPRENDER.lock().unwrap().set_materials(project);
                //     }
                // }
            }
            TheEvent::NodeViewScrolled(id, offset) => {
                if id.name == "ShapeFX NodeCanvas" {
                    self.graph.scroll_offset = *offset;
                }
            }
            TheEvent::PaletteIndexChanged(_, _index) => {
                // if let Some(material_id) = server_ctx.curr_material_object {
                //     if let Some(material) = project.materials.get_mut(&material_id) {
                //         if let Some(selected_index) = material.selected_node {
                //             let prev = material.to_json();
                //             if material.nodes[selected_index].set_palette_index(*index) {
                //                 material
                //                     .render_preview(&project.palette, &TEXTURES.lock().unwrap());
                //                 ui.set_node_preview("Map NodeCanvas", 0, material.get_preview());

                //                 let next = material.to_json();
                //                 MAPRENDER.lock().unwrap().set_materials(project);
                //                 let undo = MaterialFXUndoAtom::Edit(material_id, prev, next);
                //                 UNDOMANAGER.lock().unwrap().add_materialfx_undo(undo, ctx);

                //                 redraw = true;
                //             }
                //         }
                //     }
                // }
            }
            // TheEvent::ValueChanged(id, value) => {
            /*
            if id.name.starts_with(":MATERIALFX:") {
                if let Some(name) = id.name.strip_prefix(":MATERIALFX: ") {
                    let mut value = value.clone();

                    #[allow(clippy::collapsible_else_if)]
                    if let Some(material_id) = server_ctx.curr_material_object {
                        if let Some(material) = project.materials.get_mut(&material_id) {
                            if let Some(selected_index) = material.selected_node {
                                let prev = material.to_json();

                                // Convert TextList back
                                if let Some(TheValue::TextList(_, list)) =
                                    material.nodes[selected_index].get(name)
                                {
                                    if let Some(v) = value.to_i32() {
                                        value = TheValue::TextList(v, list.clone());
                                    }
                                }

                                // Look up the texture.
                                if material.nodes[selected_index].role
                                    == MaterialFXNodeRole::Material
                                {
                                    // if let TheValue::Text(tags) = &value {
                                    //     if let Some(TheValue::Tile(_, id)) = TILEDRAWER
                                    //         .lock()
                                    //         .unwrap()
                                    //         .get_tile_by_tags(0, &tags.to_lowercase())
                                    //     {
                                    //         material.nodes[selected_index].texture_id =
                                    //             Some(id);
                                    //     } else {
                                    //         material.nodes[selected_index].texture_id = None;
                                    //     }
                                    // }
                                }

                                material.nodes[selected_index].set(name, value);

                                if material.nodes[selected_index].supports_preview {
                                    material.nodes[selected_index]
                                        .render_preview(&project.palette);
                                    ui.set_node_preview(
                                        "Map NodeCanvas",
                                        selected_index,
                                        material.nodes[selected_index].preview.clone(),
                                    );
                                }
                                material.render_preview(
                                    &project.palette,
                                    &TEXTURES.lock().unwrap(),
                                );
                                ui.set_node_preview(
                                    "Map NodeCanvas",
                                    0,
                                    material.get_preview(),
                                );
                                let next = material.to_json();
                                MAPRENDER.lock().unwrap().set_materials(project);

                                let undo = MaterialFXUndoAtom::Edit(material_id, prev, next);
                                UNDOMANAGER.lock().unwrap().add_materialfx_undo(undo, ctx);
                            }
                        }
                    }
                }
            }*/
            //}
            // TheEvent::StateChanged(id, TheWidgetState::Selected) => {
            //     if id.name == "Material Item" {
            //         let material_id = id.uuid;
            //         server_ctx.curr_material_object = Some(material_id);
            //         if let Some(material) = project.materials.get_mut(&material_id) {
            //             let node_canvas = material.to_canvas(&project.palette);
            //             ui.set_node_canvas("MaterialFX NodeCanvas", node_canvas);
            //         }
            //     }
            // }
            _ => {}
        }

        redraw
    }

    pub fn set_material_selection(
        &self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
        _server_ctx: &mut ServerContext,
        index: Option<u8>,
    ) {
        for i in 0..20 {
            if let Some(icon_view) = ui.get_icon_view(&format!("Material Icon #{}", i)) {
                let icon_index = (self.material_start_index + i) as u8;

                if Some(icon_index) == index {
                    icon_view.set_border_color(Some(WHITE));
                } else {
                    icon_view.set_border_color(Some(BLACK));
                }
            }
        }
        // if let Some(index) = index {
        //     if let Some((id, material)) = project.materials.get_index_mut(index as usize) {
        //         let node_canvas = material.to_canvas(&project.palette);
        //         ui.set_node_canvas("Map NodeCanvas", node_canvas);
        //         server_ctx.curr_material = Some(*id);
        //     }
        // } else {
        //     let mut material = MaterialFXObject::default();
        //     let node_canvas = material.to_canvas(&project.palette);
        //     ui.set_node_canvas("Map NodeCanvas", node_canvas);
        // }
    }

    pub fn set_selected_material_node_ui(
        &mut self,
        _server_ctx: &mut ServerContext,
        _project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _switch_to_nodes: bool,
    ) {
        /*
        if let Some(material_id) = server_ctx.curr_material {
            if let Some(material) = project.materials.get_mut(&material_id) {
                if let Some(selected_index) = material.selected_node {
                    // Safeguard, not actually needed
                    if selected_index >= material.nodes.len() {
                        material.selected_node = None;
                        return;
                    }

                    let collection = material.nodes[selected_index].collection();

                    if let Some(text_layout) = ui.get_text_layout("Node Settings") {
                        text_layout.clear();

                        if switch_to_nodes {
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Show Node Settings"),
                                TheValue::Text("Material Node".to_string()),
                            ));
                        }

                        for (name, value) in &collection.keys {
                            if let TheValue::Text(text) = value {
                                let mut edit = TheTextLineEdit::new(TheId::named(
                                    (":MATERIALFX: ".to_owned() + name).as_str(),
                                ));
                                edit.set_value(TheValue::Text(text.clone()));
                                text_layout.add_pair(name.clone(), Box::new(edit));
                            } else if let TheValue::FloatRange(value, range) = value {
                                let mut slider = TheTextLineEdit::new(TheId::named(
                                    (":MATERIALFX: ".to_owned() + name).as_str(),
                                ));
                                slider.set_value(TheValue::Float(*value));
                                //slider.set_default_value(TheValue::Float(0.0));
                                slider.set_range(TheValue::RangeF32(range.clone()));
                                //slider.set_continuous(true);
                                text_layout.add_pair(name.clone(), Box::new(slider));
                            } else if let TheValue::IntRange(value, range) = value {
                                let mut slider = TheTextLineEdit::new(TheId::named(
                                    (":MATERIALFX: ".to_owned() + name).as_str(),
                                ));
                                slider.set_value(TheValue::Int(*value));
                                slider.set_range(TheValue::RangeI32(range.clone()));
                                //slider.set_continuous(true);
                                text_layout.add_pair(name.clone(), Box::new(slider));
                            } else if let TheValue::TextList(index, list) = value {
                                let mut dropdown = TheDropdownMenu::new(TheId::named(
                                    (":MATERIALFX: ".to_owned() + name).as_str(),
                                ));
                                for item in list {
                                    dropdown.add_option(item.clone());
                                }
                                dropdown.set_selected_index(*index);
                                text_layout.add_pair(name.clone(), Box::new(dropdown));
                            }
                        }
                        ctx.ui.relayout = true;
                    }
                }
                // PRERENDERTHREAD
                //     .lock()
                //     .unwrap()
                //     .material_changed(material.clone());
                // if let Some(region) = project.get_region(&server_ctx.curr_region) {
                //     let area = region.get_material_area(material_id);
                //     PRERENDERTHREAD.lock().unwrap().render_region(
                //         region.clone(),
                //         project.palette.clone(),
                //         area,
                //     );
                // }
            }
        } else if let Some(text_layout) = ui.get_text_layout("Node Settings") {
            text_layout.clear();
        }*/
    }
}
