use crate::prelude::*;

use zeno::{Mask, Stroke};
use itertools::Itertools;

pub struct NodeGraph  {
    pub rect                    : (usize, usize, usize, usize),
    pub dirty                   : bool,
    buffer                      : Vec<u8>,
    graph_mode                  : GraphMode,
    graph_type                  : BehaviorType,
    pub nodes                   : Vec<NodeWidget>,

    pub offset                  : (isize, isize),

    drag_indices                : Vec<usize>,
    drag_offset                 : (isize, isize),
    drag_node_pos               : Vec<(isize, isize)>,

    // For connecting nodes
    source_conn                 : Option<(BehaviorNodeConnector,usize)>,
    dest_conn                   : Option<(BehaviorNodeConnector,usize)>,

    pub clicked                 : bool,
    overview_preview_clicked    : bool,

    pub preview                 : Option<NodePreviewWidget>,
    preview_drag_start          : (isize, isize),
    preview_is_visible          : bool,

    mouse_pos                   : (usize, usize),
    mouse_hover_pos             : (usize, usize),

    behavior_tree_ids           : Vec<Uuid>,
    behavior_tree_rects         : Vec<(usize, usize, usize, usize)>,
    curr_behavior_tree_id       : Option<Uuid>,

    visible_node_ids            : Vec<Uuid>,
    behavior_id                 : Uuid,

    behavior_debug_data         : Option<BehaviorDebugData>,

    corner_index                : Option<usize>,

    pub sub_type                : NodeSubType,
    pub active_indices          : Vec<usize>,
}

impl EditorContent for NodeGraph  {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), behavior_type: BehaviorType, _asset: &Asset, _context: &ScreenContext) -> Self where Self: Sized  {
        Self {
            rect,
            dirty                       : true,
            buffer                      : vec![0;rect.2 * rect.3 * 4],
            graph_mode                  : GraphMode::Overview,
            graph_type                  : behavior_type,
            nodes                       : vec![],
            offset                      : (0, 0),
            drag_indices                : vec![],
            drag_offset                 : (0, 0),
            drag_node_pos               : vec![],

            clicked                     : false,
            overview_preview_clicked    : false,

            source_conn                 : None,
            dest_conn                   : None,

            preview                     : None,
            preview_drag_start          : (0,0),
            preview_is_visible          : false,

            mouse_pos                   : (0,0),
            mouse_hover_pos             : (0, 0),

            behavior_tree_ids           : vec![],
            behavior_tree_rects         : vec![],
            curr_behavior_tree_id       : None,

            visible_node_ids            : vec![],

            behavior_id                 : Uuid::new_v4(),

            behavior_debug_data         : None,

            sub_type                    : NodeSubType::None,
            active_indices              : vec![],

            corner_index                : None,
        }
    }

    fn set_mode(&mut self, mode: GraphMode, context: &ScreenContext) {

        // Create previews
        if mode == GraphMode::Detail && (self.graph_type == BehaviorType::Behaviors || self.graph_type == BehaviorType::Systems/*  || self.graph_type == BehaviorType::GameLogic*/) && self.preview.is_none() {
            self.preview = Some(NodePreviewWidget::new(context, self.graph_type));
        }

        self.graph_mode = mode;
    }

    fn set_mode_and_rect(&mut self, mode: GraphMode, rect: (usize, usize, usize, usize), context: &mut ScreenContext) {
        if mode == GraphMode::Detail && (self.graph_type == BehaviorType::Behaviors || self.graph_type == BehaviorType::Systems /*|| self.graph_type == BehaviorType::GameLogic*/) && self.preview.is_none() {
            self.preview = Some(NodePreviewWidget::new(context, self.graph_type));
        }
        self.graph_mode = mode;
        self.rect = rect;
        self.resize(rect.2, rect.3, context)
    }

    fn set_mode_and_nodes(&mut self, mode: GraphMode, nodes: Vec<NodeWidget>, _context: &ScreenContext) {
        self.graph_mode = mode;
        self.nodes = nodes;
        self.mark_all_dirty();
    }

    fn resize(&mut self, width: usize, height: usize, context: &mut ScreenContext) {
        self.buffer.resize(width * height * 4, 0);
        self.dirty = true;
        self.rect.2 = width;
        self.rect.3 = height;

        self.sort(context);
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, _options: &mut Option<Box<dyn EditorOptions>>) {
        let safe_rect = (0_usize, 0_usize, self.rect.2, self.rect.3);

        if self.dirty {
            context.draw2d.draw_square_pattern(&mut self.buffer[..], &safe_rect, safe_rect.2, &[44, 44, 46, 255], &[56, 56, 56, 255], 40);

            if self.graph_mode == GraphMode::Overview {
                for active_index in 0..self.active_indices.len() {
                    let index = self.active_indices[active_index];

                    let mut selected = false;

                    if self.graph_type == BehaviorType::Tiles {
                        if index == context.curr_tileset_index {
                            selected = true;
                        }
                    } else
                    if self.graph_type == BehaviorType::Regions {
                        if index == context.curr_region_index {
                            selected = true;
                        }
                    } else
                    if self.graph_type == BehaviorType::Behaviors {
                        if index == context.curr_behavior_index {
                            selected = true;
                        }
                    } else
                    if self.graph_type == BehaviorType::Systems {
                        if index == context.curr_systems_index {
                            selected = true;
                        }
                    }
                    if self.graph_type == BehaviorType::Items {
                        if index == context.curr_items_index {
                            selected = true;
                        }
                    }

                    if self.nodes[index].dirty {
                        let mut preview_buffer = vec![0; 100 * 100 * 4];
                        if self.graph_type == BehaviorType::Tiles && self.sub_type == NodeSubType::Tilemap {
                            // For tile maps draw the default_tile
                            if let Some(map)= asset.tileset.maps.get_mut(&asset.tileset.maps_ids[active_index]) {
                                if let Some(default_tile) = map.settings.default_tile {
                                    context.draw2d.draw_animated_tile(&mut preview_buffer[..], &(0, 0), map, 100, &default_tile, 0, 100);
                                }
                            }
                        } else
                        if self.graph_type == BehaviorType::Tiles && self.sub_type == NodeSubType::Audio {
                            // For audio draw an audio icon
                            if let Some(icon) = context.icons.get(&"audio".to_string()) {
                                context.draw2d.scale_chunk(&mut preview_buffer[..], &(10, 10, 80, 80), 100, &icon.0[..], &(icon.1 as usize, icon.2 as usize), 0.5);
                            }
                        } else
                        if self.graph_type == BehaviorType::Systems {
                            // For audio draw an audio icon
                            if let Some(icon) = context.icons.get(&"skills".to_string()) {
                                context.draw2d.scale_chunk(&mut preview_buffer[..], &(10, 10, 80, 80), 100, &icon.0[..], &(icon.1 as usize, icon.2 as usize), 0.5);
                            }
                        } else
                        if self.graph_type == BehaviorType::Regions {
                            // For regions draw the center of the map
                            if let Some(region)= context.data.regions.get_mut(&context.data.regions_ids[index]) {

                                let mut mask_buffer = vec![0.0; 100 * 100];
                                context.draw2d.create_rounded_rect_mask(&mut mask_buffer[..], &(0, 0, 100, 100), 100, &(10.0, 10.0, 10.0, 10.0));

                                context.draw2d.mask = Some(mask_buffer.clone());
                                context.draw2d.mask_size = (100, 100);

                                let offset = region.get_center_offset_for_visible_size((10, 10));
                                context.draw2d.draw_region(&mut preview_buffer[..], region, &(0, 0, 100, 100), &offset, 100, 10, anim_counter, asset, false);

                                context.draw2d.mask = None;
                            }
                        } else
                        if self.graph_type == BehaviorType::Behaviors {
                            // Draw the main behavior tile
                            if let Some(tile_id) = context.data.get_behavior_default_tile(context.data.behaviors_ids[index]) {
                                if let Some(map)= asset.tileset.maps.get_mut(&tile_id.tilemap) {
                                    context.draw2d.draw_animated_tile(&mut preview_buffer[..], &(0, 0), map, 100, &(tile_id.x_off as usize, tile_id.y_off as usize), 0, 100);
                                }
                            }
                        } else
                        if self.graph_type == BehaviorType::Items {
                            if self.sub_type == NodeSubType::Item {
                                // Draw the item tile
                                if let Some(tile_id) = context.data.get_item_default_tile(context.data.items_ids[index]) {
                                    if let Some(map)= asset.tileset.maps.get_mut(&tile_id.tilemap) {
                                        context.draw2d.draw_animated_tile(&mut preview_buffer[..], &(0, 0), map, 100, &(tile_id.x_off as usize, tile_id.y_off as usize), 0, 100);
                                    }
                                }
                            } else
                            if self.sub_type == NodeSubType::Spell {
                                // Draw the spell tile
                                if let Some(tile_id) = context.data.get_spell_default_tile(context.data.spells_ids[active_index]) {
                                    if let Some(map)= asset.tileset.maps.get_mut(&tile_id.tilemap) {
                                        context.draw2d.draw_animated_tile(&mut preview_buffer[..], &(0, 0), map, 100, &(tile_id.x_off as usize, tile_id.y_off as usize), 0, 100);
                                    }
                                }
                            }
                        }

                        self.nodes[index].draw_overview(frame, anim_counter, asset, context, selected, &preview_buffer, selected && self.overview_preview_clicked);
                    }

                    let rect= self.get_node_rect(index, true);
                    self.nodes[index].graph_offset = (rect.0, rect.1);
                    self.nodes[index].graph_offset = (rect.0, rect.1);
                    context.draw2d.blend_slice_safe(&mut self.buffer[..], &self.nodes[index].buffer[..], &rect, safe_rect.2, &safe_rect);
                }
            } else
            if self.nodes.len() > 0 {
                // Detail View

                let mut corner_index : Option<usize> = None;

                // Draw nodes
                for index in 0..self.nodes.len() {

                    // Check for corner node
                    if self.nodes[index].is_corner_node {
                        corner_index = Some(index);
                        self.corner_index = Some(index);
                        continue;
                    }

                    // We only draw nodes which are marked visible, i.e. connected to the current behavior tree or unconnected nodes
                    if self.visible_node_ids.contains(&self.widget_index_to_node_id(index)) {

                        if self.nodes[index].dirty {

                            let selected = if Some(self.nodes[index].id) == self.get_curr_node_id(context) { true } else { false };
                            self.nodes[index].draw(frame, anim_counter, asset, context, selected);
                        }

                        let rect= self.get_node_rect(index, true);
                        self.nodes[index].graph_offset = (rect.0, rect.1);
                        context.draw2d.blend_slice_safe(&mut self.buffer[..], &self.nodes[index].buffer[..], &rect, safe_rect.2, &safe_rect);
                    }
                }

                // Corner node if available
                if let Some(corner_index) = corner_index {
                    if self.nodes[corner_index].dirty {
                        self.nodes[corner_index].draw(frame, anim_counter, asset, context, false);
                    }

                    let rect= self.get_node_rect(corner_index, true);
                    self.nodes[corner_index].graph_offset = (rect.0, rect.1);
                    self.nodes[corner_index].graph_offset = (rect.0, rect.1);
                    context.draw2d.blend_slice_safe(&mut self.buffer[..], &self.nodes[corner_index].buffer[..], &rect, safe_rect.2, &safe_rect);
                }

                // --
                let mut mask : Vec<u8> = vec![0; safe_rect.2 * safe_rect.3];
                let mut path : String = "".to_string();

                let mut orange_mask : Vec<u8> = vec![0; safe_rect.2 * safe_rect.3];
                let mut orange_path : String = "".to_string();

                // Draw connections
                if let Some(behavior) = context.data.get_behavior(self.get_curr_behavior_id(context), self.graph_type) {

                    for (source_node_id , source_connector, dest_node_id, dest_connector) in &behavior.data.connections {

                        if self.visible_node_ids.contains(source_node_id) {
                            let source_index = self.node_id_to_widget_index(source_node_id.clone());
                            let source_rect = &self.get_node_rect(source_index, true);
                            let source_node = &self. nodes[source_index];

                            let dest_index = self.node_id_to_widget_index(dest_node_id.clone());
                            let dest_rect = &self.get_node_rect(dest_index, true);
                            let dest_node = &self. nodes[dest_index];

                            let s_connector = &source_node.node_connector[source_connector];
                            let d_connector = &dest_node.node_connector[dest_connector];

                            let start_x;
                            let start_y;

                            let control_start_x;
                            let control_start_y;

                            let control_end_x;
                            let control_end_y;

                            let end_x;
                            let end_y;

                            let sdx : f64;
                            let sdy : f64;
                            let edx : f64;
                            let edy : f64;

                            if *source_connector == BehaviorNodeConnector::Right {
                                start_x = source_rect.0 + s_connector.rect.0 as isize + s_connector.rect.2 as isize;
                                start_y = source_rect.1 + s_connector.rect.1 as isize + s_connector.rect.3 as isize / 2;
                                sdx = 1.0; sdy = 0.0;
                            } else {
                                start_x = source_rect.0 + s_connector.rect.0 as isize + s_connector.rect.2 as isize / 2;
                                start_y = source_rect.1 + s_connector.rect.1 as isize + s_connector.rect.3 as isize;
                                sdx = 0.0; sdy = 1.0;
                            }

                            if *dest_connector == BehaviorNodeConnector::Left {
                                end_x = dest_rect.0 + d_connector.rect.0 as isize + 1;
                                end_y = dest_rect.1 + d_connector.rect.1 as isize + d_connector.rect.3 as isize / 2;
                                edx = -1.0; edy = 0.0;
                            } else {
                                end_x = dest_rect.0 + d_connector.rect.0 as isize + d_connector.rect.2 as isize / 2;
                                end_y = dest_rect.1 + d_connector.rect.1 as isize + 1;
                                edx = 0.0; edy = -1.0;
                            }

                            let dx = start_x - end_x;
                            let dy = start_y - end_y;

                            let d = ((dx * dx + dy * dy) as f64).sqrt().clamp(0.0, 50.0);

                            control_start_x = start_x + (sdx * d) as isize;
                            control_start_y = start_y + (sdy * d) as isize;

                            control_end_x = end_x + (edx * d) as isize;
                            control_end_y = end_y + (edy * d) as isize;

                            let mut connection_drawn = false;
                            if let Some(debug_data) = &self.behavior_debug_data {
                                if debug_data.executed_connections.contains(&(self.graph_type, *source_node_id, *source_connector)) {
                                    orange_path += format!("M {},{} C {},{} {},{} {},{}", start_x, start_y, control_start_x, control_start_y, control_end_x, control_end_y, end_x, end_y).as_str();
                                    connection_drawn = true;
                                }
                            }

                            if connection_drawn == false {
                                path += format!("M {},{} C {},{} {},{} {},{}", start_x, start_y, control_start_x, control_start_y, control_end_x, control_end_y, end_x, end_y).as_str();
                            }
                        }
                    }
                }

                // Draw ongoing connection effort
                if let Some(conn) = &self.source_conn {

                    let node = &self.nodes[conn.1];
                    let connector = &node.node_connector[&conn.0];

                    let node_rect = self.get_node_rect(conn.1, true);

                    let start_x = node_rect.0 + connector.rect.0 as isize + connector.rect.2 as isize / 2;
                    let start_y = node_rect.1 + connector.rect.1 as isize + connector.rect.3 as isize / 2;

                    let mut end_x = self.mouse_pos.0 as isize - self.rect.0 as isize;
                    let mut end_y = self.mouse_pos.1 as isize - self.rect.1 as isize;

                    if let Some(conn) = &self.dest_conn {
                        let node = &self.nodes[conn.1];
                        let connector = &node.node_connector[&conn.0];
                        let node_rect = self.get_node_rect(conn.1, true);

                        end_x = node_rect.0 + connector.rect.0 as isize + connector.rect.2 as isize / 2;
                        end_y = node_rect.1 + connector.rect.1 as isize + connector.rect.3 as isize / 2;
                    }

                    path += format!("M {},{} L {},{}", start_x, start_y, end_x, end_y).as_str();
                }

                // Draw the path if not empty
                if !path.is_empty() || !orange_path.is_empty() {
                    Mask::new(path.as_str())
                    .size(safe_rect.2 as u32, safe_rect.3 as u32)
                    .style(
                       Stroke::new(2.0)
                    )
                    .render_into(&mut mask, None);

                    context.draw2d.blend_mask(&mut self.buffer[..], &safe_rect, safe_rect.2, &mask[..], &(safe_rect.2, safe_rect.3), &context.node_connector_color);
                }

                // Draw the orange path if not empty
                if !orange_path.is_empty() {
                    Mask::new(orange_path.as_str())
                    .size(safe_rect.2 as u32, safe_rect.3 as u32)
                    .style(
                        Stroke::new(2.0)
                    )
                    .render_into(&mut orange_mask, None);

                    context.draw2d.blend_mask(&mut self.buffer[..], &safe_rect, safe_rect.2, &orange_mask[..], &(safe_rect.2, safe_rect.3), &context.color_orange);
                }

                let behavior_id = self.get_curr_behavior_id(context);
                self.preview_is_visible = false;

                // Render the preview widget
                if let Some(preview) = &mut self.preview {
                    if context.data.get_behavior_default_position(behavior_id).is_some() {
                        preview.draw(frame, anim_counter, asset, context);
                        preview.rect = (self.rect.0 + self.rect.2 - preview.size.0, self.rect.1, preview.size.0, preview.size.1);
                        preview.graph_offset = (preview.rect.0 as isize, preview.rect.1 as isize);
                        context.draw2d.blend_slice(&mut self.buffer[..], &mut preview.buffer[..], &(self.rect.2 - preview.size.0, 0, preview.size.0, preview.size.1), safe_rect.2);
                        self.preview_is_visible = true;
                    }
                }

                // Render the behavior tree buttons

                self.behavior_tree_rects = vec![];

                let left_start = if self.graph_type == BehaviorType::Behaviors || self.graph_type == BehaviorType::Items || self.graph_type == BehaviorType::GameLogic { 180 } else { 5 };
                let mut total_width = safe_rect.2 - left_start - 5;
                if let Some(preview) = &mut self.preview {
                    if self.preview_is_visible {
                        total_width -= preview.size.0;
                    }
                }
                let mut bt_rect = (left_start, 3, 170, 25);
                for bt_id in &self.behavior_tree_ids {

                    let mut selected = false;
                    if let Some(curr_index) = self.curr_behavior_tree_id {
                        if curr_index == *bt_id {
                            selected = true;
                        }
                    }

                    let color = if selected { context.color_gray } else {context.color_black };
                    context.draw2d.draw_rounded_rect(&mut self.buffer[..], &bt_rect, safe_rect.2, &((bt_rect.2) as f64, (bt_rect.3) as f64), &color, &(0.0, 0.0, 0.0, 0.0));

                    self.behavior_tree_rects.push(bt_rect.clone());

                    let idx = self.node_id_to_widget_index(*bt_id);
                    context.draw2d.draw_text_rect(&mut self.buffer[..], &bt_rect, safe_rect.2, &asset.get_editor_font("OpenSans"), 16.0, &self.nodes[idx].name, &context.color_white, &color, crate::draw2d::TextAlignment::Center);

                    bt_rect.0 += 171;
                    if (bt_rect.0 + bt_rect.2) - left_start > total_width {
                        bt_rect.0 = left_start;
                        bt_rect.1 += 26;
                    }
                }
            }
        }
        self.dirty = false;
        context.draw2d.copy_slice(frame, &mut self.buffer[..], &self.rect, context.width);

        // Draw nodes overlay
        for index in 0..self.nodes.len() {
            let mut node_offset = self.nodes[index].graph_offset.clone();
            node_offset.0 += self.rect.0 as isize;
            node_offset.1 += self.rect.1 as isize;

            if let Some(menu) = &mut self.nodes[index].menu {
                menu.emb_offset = node_offset.clone();
                menu.draw_overlay(frame, &self.rect, anim_counter, asset, context);
            }

            for atom in &mut self.nodes[index].widgets {
                atom.emb_offset = node_offset.clone();
                atom.draw_overlay(frame, &self.rect, anim_counter, asset, context);
            }
        }

        // Preview overlay ?
        if let Some(preview) = &mut self.preview {
            if self.preview_is_visible {
                let node_offset = preview.graph_offset.clone();
                for atom in &mut preview.widgets {
                    atom.emb_offset = node_offset.clone();
                    atom.draw_overlay(frame, &self.rect, anim_counter, asset, context);
                }
            }
        }
    }

    /// Returns the rectangle for the given node either in relative or absolute coordinates
    fn get_node_rect(&self, node_index: usize, relative: bool) -> (isize, isize, usize, usize) {
        let mut x = self.nodes[node_index].user_data.position.0 + self.offset.0;
        let mut y = self.nodes[node_index].user_data.position.1 + self.offset.1;

        if self.nodes[node_index].is_corner_node {
            x = -7;
            y = -14;
        }

        if relative == false {
            x += self.rect.0 as isize;
            y += self.rect.1 as isize;
        }

        (x, y, self.nodes[node_index].size.0, self.nodes[node_index].size.1)
    }

    /// Updates a node value from the dialog
    fn update_from_dialog(&mut self, id: (Uuid, Uuid, String), value: Value, _asset: &mut Asset, context: &mut ScreenContext, _options: &mut Option<Box<dyn EditorOptions>>) {
        // println!("graph {:?} {:?}", id, value);

        if context.dialog_entry == DialogEntry::NodeName {
            // Node based
            for node_index in 0..self.nodes.len() {
                if self.nodes[node_index].id == context.dialog_node_behavior_id.0 {
                    self.nodes[node_index].name = context.dialog_node_behavior_value.4.clone();
                    self.nodes[node_index].dirty = true;
                    self.dirty = true;

                    context.data.set_behavior_node_name((self.get_curr_behavior_id(context), context.dialog_node_behavior_id.0),context.dialog_node_behavior_value.4.clone(), self.graph_type);
                    break;
                }
            }
            return
        }

        // Atom based
        for node_index in 0..self.nodes.len() {
            for atom_index in 0..self.nodes[node_index].widgets.len() {
                if let Some(node_id) = &self.nodes[node_index].widgets[atom_index].behavior_id {
                    if node_id.0 == id.0 && node_id.1 == id.1 && node_id.2 == id.2 {
                        self.nodes[node_index].widgets[atom_index].atom_data.value = value.clone();
                        self.nodes[node_index].widgets[atom_index].dirty = true;
                        self.nodes[node_index].dirty = true;
                        self.dirty = true;

                        context.data.set_behavior_id_value(id.clone(), value.clone(), context.curr_graph_type);

                        break;
                    }
                }
            }
        }
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _options: &mut Option<Box<dyn EditorOptions>>, toolbar: &mut Option<&mut ToolBar>) -> bool {

        self.overview_preview_clicked = false;
        let mut rc = false;

        // Clicked outside ?
        if pos.0 < self.rect.0 || pos.1 < self.rect.1 {
            return false;
        }

        if self.graph_mode == GraphMode::Overview {
            for active_index in 0..self.active_indices.len() {
                let index = self.active_indices[active_index];

                let rect= self.get_node_rect(index, false);

                if context.contains_pos_for_isize(pos, rect) {

                    self.drag_offset = (pos.0 as isize, pos.1 as isize);
                    self.drag_node_pos = vec!((self.nodes[index].user_data.position.0 as isize, self.nodes[index].user_data.position.1 as isize));
                    self.drag_indices = vec![index];

                    if self.graph_type == BehaviorType::Tiles {
                        if context.curr_tileset_index != index {
                            self.nodes[context.curr_tileset_index].dirty = true;
                            context.curr_tileset_index = index;
                            self.nodes[index].dirty = true;
                            self.dirty = true;
                            self.clicked = true;
                        }
                    }
                    if self.graph_type == BehaviorType::Regions {
                        if context.curr_region_index != index {

                            self.nodes[context.curr_region_index].dirty = true;
                            context.curr_region_index = index;
                            self.nodes[index].dirty = true;
                            self.dirty = true;
                            self.clicked = true;
                        }
                    }
                    if self.graph_type == BehaviorType::Behaviors {
                        if context.curr_behavior_index != index {

                            self.nodes[context.curr_behavior_index].dirty = true;
                            context.curr_behavior_index = index;
                            self.nodes[index].dirty = true;
                            self.dirty = true;
                            self.clicked = true;
                        }
                    }
                    if self.graph_type == BehaviorType::Systems {
                        if context.curr_systems_index != index {

                            self.nodes[context.curr_systems_index].dirty = true;
                            context.curr_systems_index = index;
                            self.nodes[index].dirty = true;
                            self.dirty = true;
                            self.clicked = true;
                        }
                    }
                    if self.graph_type == BehaviorType::Items {
                        if context.curr_items_index != index {

                            self.nodes[context.curr_items_index].dirty = true;
                            context.curr_items_index = index;
                            self.nodes[index].dirty = true;
                            self.dirty = true;
                            self.clicked = true;
                        }
                    }

                    if let Some(toolbar) = toolbar {
                        if let Some(atom) = toolbar.get_atom_at_index(0) {
                            if atom.curr_index != active_index {
                                atom.curr_index = active_index;
                                atom.dirty = true;
                            }
                        }
                    }

                    // Test for click in preview area
                    let dx = pos.0 as isize - rect.0;
                    let dy = pos.1 as isize - rect.1;
                    if dx >= 10 && dx <= 110 && dy >= 10 && dy <= 110 {
                        self.overview_preview_clicked = true;
                        self.drag_indices = vec![];
                        self.nodes[index].dirty = true;
                        self.dirty = true;

                        if self.sub_type == NodeSubType::Audio {
                            for (audio_index, n) in asset.audio_names.iter().enumerate() {
                                if *n == self.nodes[index].name {

                                    if let Some(file) = std::fs::File::open(asset.audio_paths[audio_index].clone()).ok() {
                                        let buffered = std::io::BufReader::new(file);
                                        context.play_audio((*n).to_string(), buffered);
                                    }
                                }
                            }
                        }
                    }

                    rc = true;
                }
            }

            // Check the nodes
            for active_index in 0..self.active_indices.len() {
                let index = self.active_indices[active_index];

                let rect= self.get_node_rect(index, false);
                if context.contains_pos_for_isize(pos, rect) {

                    let local = ((pos.0 as isize - rect.0) as usize, (pos.1 as isize  - rect.1) as usize);

                    if self.nodes[index].mouse_down(local, asset, context) {
                        self.dirty = true;
                        return true;
                    }
                    break;
                }
            }
        } else
        if self.graph_mode == GraphMode::Detail {

            // Check the behavior tree selector at the top
            for index in 0..self.behavior_tree_rects.len() {
                if context.contains_pos_for((pos.0 - self.rect.0, pos.1 - self.rect.1), self.behavior_tree_rects[index]) {
                    self.curr_behavior_tree_id = Some(self.behavior_tree_ids[index]);
                    self.check_node_visibility(context);
                    self.dirty = true;
                    return true;
                }
            }

            for index in 0..self.nodes.len() {

                if self.nodes[index].is_corner_node {
                    continue;
                }

                let rect= self.get_node_rect(index, false);

                if self.visible_node_ids.contains(&self.widget_index_to_node_id(index)) {

                    if context.contains_pos_for_isize(pos, rect) {

                        // Check for node terminals
                        for (conn, connector) in &self.nodes[index].node_connector {
                            let c_rect = (pos.0  as isize - rect.0, pos.1 as isize - rect.1);
                            if c_rect.0 > 0 && c_rect.1 > 0 {
                                if context.contains_pos_for((c_rect.0 as usize, c_rect.1 as usize), connector.rect) {
                                    self.source_conn = Some((*conn, index));

                                    return true;
                                }
                            }
                        }
                        self.drag_offset = (pos.0 as isize, pos.1 as isize);
                        self.drag_node_pos = vec!((self.nodes[index].user_data.position.0 as isize, self.nodes[index].user_data.position.1 as isize));
                        self.drag_indices = vec![index];
                        self.collect_drag_children_indices(self.widget_index_to_node_id(index), &context);

                        if self.get_curr_node_id(context) != Some(self.nodes[index].id) {
                            // Update the old selection
                            if let Some(selected_id) = self.get_curr_node_id(context) {
                                let sel_index = self.node_id_to_widget_index(selected_id);

                                self.nodes[sel_index].dirty = true;
                            }

                            if let Some(behavior) = context.data.get_mut_behavior(self.behavior_id, self.graph_type) {
                                behavior.data.curr_node_id = Some(self.nodes[index].id);
                            }

                            self.nodes[index].dirty = true;
                            self.dirty = true;
                            self.clicked = true;
                            rc = true;
                        }
                    }
                }
            }

            // Check Preview

            // Render the preview widget
            if let Some(preview) = &mut self.preview {
                if self.preview_is_visible {
                    if context.contains_pos_for(pos, preview.rect) {
                        if preview.mouse_down((pos.0 - preview.rect.0, pos.1 - preview.rect.1), asset, context) {

                            if preview.clicked {
                                self.dirty = true;
                                return true;
                            } else {
                                self.preview_drag_start = (pos.0 as isize, pos.1 as isize);
                            }
                        }
                    }
                }
            }


            // Check the nodes
            for index in 0..self.nodes.len() {

                // Only if the node is visible
                if self.visible_node_ids.contains(&self.widget_index_to_node_id(index)) {

                    let rect= self.get_node_rect(index, false);
                    if context.contains_pos_for_isize(pos, rect) {

                        let local = ((pos.0 as isize - rect.0) as usize, (pos.1 as isize  - rect.1) as usize);

                        if self.nodes[index].mouse_down(local, asset, context) {
                            self.dirty = true;
                            return true;
                        }
                        break;
                    }
                }
            }
        }

        rc
    }

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _options: &mut Option<Box<dyn EditorOptions>>, toolbar: &mut Option<&mut ToolBar>) -> bool {


        // Switch the editor state if an overview was clicked
        if self.overview_preview_clicked {

            let mut change_index = true;

            let mut widget_index = 0;
            if self.graph_type == BehaviorType::Tiles {
                if self.nodes[context.curr_tileset_index].sub_type == NodeSubType::Tilemap || self.nodes[context.curr_tileset_index].sub_type == NodeSubType::Image {
                    context.switch_editor_state = Some(super::EditorState::TilesDetail);
                    widget_index = 1;
                } else {
                    change_index = false;
                    self.dirty = true;
                    self.nodes[context.curr_tileset_index].dirty = true;
                }
            } else
            if self.graph_type == BehaviorType::Regions {
                context.switch_editor_state = Some(super::EditorState::RegionDetail);
                widget_index = 2;
            } else
            if self.graph_type == BehaviorType::Behaviors {
                context.switch_editor_state = Some(super::EditorState::BehaviorDetail);
                widget_index = 3;
            } else
            if self.graph_type == BehaviorType::Systems {
                context.switch_editor_state = Some(super::EditorState::SystemsDetail);
                widget_index = 4;
            } else
            if self.graph_type == BehaviorType::Items {
                context.switch_editor_state = Some(super::EditorState::ItemsDetail);
                widget_index = 5;
            }

            if change_index {
                if let Some(toolbar) = toolbar {
                    toolbar.widgets[widget_index].selected = false;
                    toolbar.widgets[widget_index].right_selected = true;
                    toolbar.widgets[widget_index].dirty = true;
                }
            }
        }

        self.overview_preview_clicked = false;

        if self.drag_indices.is_empty() == false {
            context.target_fps = context.default_fps;

            // Save the new node position
            if self.graph_mode == GraphMode::Detail {
                if let Some(behavior) = context.data.get_mut_behavior(self.get_curr_behavior_id(context), self.graph_type) {
                    for node_widget in &self.nodes {
                        if self.drag_indices.contains(&self.node_id_to_widget_index(node_widget.id)) {
                            let position = node_widget.user_data.position.clone();
                            if let Some(behavior_node) = behavior.data.nodes.get_mut(&node_widget.id) {
                                behavior_node.position = position;
                            }
                        }
                    }
                    behavior.save_data();
                }
            }
            self.drag_indices = vec![];
            self.drag_node_pos = vec![];
        }

        // Node connection
        if let Some(source_conn) = &self.source_conn {
            if let Some(dest_conn) = &self.dest_conn {

                if let Some(behavior) = context.data.get_mut_behavior(self.get_curr_behavior_id(context), self.graph_type) {

                    // Add the connection in the order of source connector -> dest connector
                    if self.connector_is_source(dest_conn.0) {
                        behavior.data.connections.push((self.widget_index_to_node_id(dest_conn.1), dest_conn.0, self.widget_index_to_node_id(source_conn.1), source_conn.0));
                        self.nodes[source_conn.1].dirty = true;
                        behavior.save_data();
                    } else {
                        behavior.data.connections.push((self.widget_index_to_node_id(source_conn.1), source_conn.0, self.widget_index_to_node_id(dest_conn.1), dest_conn.0));
                        self.nodes[dest_conn.1].dirty = true;
                        behavior.save_data();
                    }
                }
                self.dest_conn = None;
            }
            self.source_conn = None;
            self.dirty = true;
            return true;
        }

        // Check the atom widgets
        for index in 0..self.nodes.len() {
            let rect= self.get_node_rect(index, false);
            let local = ((pos.0 as isize - rect.0) as usize, (pos.1 as isize  - rect.1) as usize);

            let mut menu_activated : Option<String> = None;
            if let Some(menu) = &mut self.nodes[index].menu {
                if menu.mouse_up(local, asset, context) {
                    if menu.new_selection.is_some() {
                        menu_activated = Some(menu.text[menu.curr_index].clone());
                        menu.dirty = true;
                        self.dirty = true;
                    }
                }
            }

            // If a menu was activated, mark the node as dirty
            if let Some(menu_activated) = menu_activated {
                self.nodes[index].dirty = true;

                if self.graph_mode == GraphMode::Overview {
                    if  "Rename".to_string() == menu_activated {
                        // Rename node
                        context.dialog_state = DialogState::Opening;
                        context.dialog_height = 0;
                        context.target_fps = 60;
                        context.dialog_entry = DialogEntry::NodeName;
                        context.dialog_node_behavior_id = (self.nodes[index].id, Uuid::new_v4(), "".to_string());
                        context.dialog_node_behavior_value = (0.0, 0.0, 0.0, 0.0, self.nodes[index].name.clone());
                        context.dialog_value = Value::String(self.nodes[index].name.clone());
                    } else
                    if "Delete".to_string() == menu_activated {
                        if self.graph_type == BehaviorType::Regions {
                            self.nodes.remove(context.curr_region_index);
                            context.data.delete_region(&context.curr_region_index);
                            if let Some(toolbar) = toolbar {
                                toolbar.widgets[0].text = context.data.regions_names.clone();
                                toolbar.widgets[0].curr_index = 0;
                                toolbar.widgets[0].dirty = true;
                            }
                            context.curr_region_index = 0;
                        } else
                        if self.graph_type == BehaviorType::Behaviors {
                            self.nodes.remove(context.curr_behavior_index);
                            context.data.delete_behavior(&context.curr_behavior_index);
                            if let Some(toolbar) = toolbar {
                                toolbar.widgets[0].text = context.data.behaviors_names.clone();
                                toolbar.widgets[0].curr_index = 0;
                                toolbar.widgets[0].dirty = true;
                            }
                            context.curr_behavior_index = 0;
                        } else
                        if self.graph_type == BehaviorType::Systems {
                            self.nodes.remove(context.curr_systems_index);
                            context.data.delete_system(&context.curr_systems_index);
                            if let Some(toolbar) = toolbar {
                                toolbar.widgets[0].text = context.data.systems_names.clone();
                                toolbar.widgets[0].curr_index = 0;
                                toolbar.widgets[0].dirty = true;
                            }
                            context.curr_systems_index = 0;
                        } else
                        if self.graph_type == BehaviorType::Items {

                            self.nodes.remove(context.curr_items_index);

                            if self.sub_type == NodeSubType::Item {
                                if let Some(index) = self.active_indices.iter().position(|&r| r == context.curr_items_index) {
                                    context.data.delete_item(&index);
                                    if let Some(toolbar) = toolbar {
                                        toolbar.widgets[0].text = context.data.items_names.clone();
                                        toolbar.widgets[0].curr_index = 0;
                                        toolbar.widgets[0].dirty = true;
                                    }
                                }
                            } else
                            if self.sub_type == NodeSubType::Spell {
                                if let Some(index) = self.active_indices.iter().position(|&r| r == context.curr_items_index) {
                                    context.data.delete_spell(&index);
                                    if let Some(toolbar) = toolbar {
                                        toolbar.widgets[0].text = context.data.spells_names.clone();
                                        toolbar.widgets[0].curr_index = 0;
                                        toolbar.widgets[0].dirty = true;
                                    }
                                }
                            }

                            context.curr_items_index = 0;
                        }
                        self.sort(context);
                        if self.nodes.len() > 0 {
                            self.nodes[0].dirty = true;
                        }
                    }
                }

                if self.graph_mode == GraphMode::Detail {
                    if  "Rename".to_string() == menu_activated {
                        // Rename node
                        context.dialog_state = DialogState::Opening;
                        context.dialog_height = 0;
                        context.target_fps = 60;
                        context.dialog_entry = DialogEntry::NodeName;
                        context.dialog_node_behavior_id = (self.nodes[index].id, Uuid::new_v4(), "".to_string());
                        context.dialog_value = Value::String(self.nodes[index].name.clone());
                    } else
                    if "Disconnect".to_string() == menu_activated {
                        // Disconnect node
                        self.disconnect_node(self.nodes[index].id, context);
                    } else
                    if "Delete".to_string() == menu_activated {
                        // Delete node
                        self.delete_node(self.nodes[index].id, context);
                    } else
                    if "Help".to_string() == menu_activated {
                        if let Some(help_link) = &self.nodes[index].help_link {
                            _  = open::that(help_link);
                        }
                    }
                }

                return true;
            }

            if self.nodes[index].mouse_up(local, asset, context) {
                self.dirty = true;

                if self.graph_type != BehaviorType::Regions {
                    if let Some(behavior) = context.data.get_mut_behavior(self.get_curr_behavior_id(context), self.graph_type) {
                        behavior.save_data();
                    }
                }
                return true;
            }
        }

        // Preview
        if let Some(preview) = &mut self.preview {
            if self.preview_is_visible && preview.mouse_up(pos, asset, context) {
                self.dirty = true;
                return  true;
            }
        }
        false
    }

    fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {

        self.mouse_pos = pos.clone();

        for index in 0..self.nodes.len() {

            // Node menu
            if let Some(menu) = &mut self.nodes[index].menu {
                if menu.mouse_dragged(pos, asset, context) {
                    return true;
                }
            }

            let rect= self.get_node_rect(index, false);
            for atom in &mut self.nodes[index].widgets {
                if atom.atom_widget_type == AtomWidgetType::SmallMenuButton || atom.atom_widget_type == AtomWidgetType::NodeMenuButton {
                    if atom.mouse_dragged(pos, asset, context) {
                        return true;
                    }
                } else {
                    let x = if pos.0 as isize > rect.0 { pos.0 as isize - rect.0 } else { 0 };
                    let y = if pos.1 as isize > rect.1 { pos.1 as isize - rect.1 } else { 0 };
                    let local = (x as usize, y as usize);
                    if atom.mouse_dragged(local, asset, context) {
                        self.dirty = atom.dirty;
                        self.nodes[index].dirty = atom.dirty;
                        return true;
                    }
                }
            }
        }

        // Draw preview overlay
        if let Some(preview) = &mut self.preview {
            if self.preview_is_visible {
                for atom in &mut preview.widgets {
                    if atom.mouse_dragged(pos, asset, context) {
                        return true;
                    }
                }
            }
        }

        // Dragging a node
        if self.drag_indices.is_empty() == false {
            let dx = pos.0 as isize - self.drag_offset.0;
            let dy = pos.1 as isize - self.drag_offset.1;

            for offset in 0..self.drag_indices.len() {
                let index = self.drag_indices[offset];
                self.nodes[index].user_data.position.0 = self.drag_node_pos[offset].0 + dx;
                self.nodes[index].user_data.position.1 = self.drag_node_pos[offset].1 + dy;
                self.dirty = true;
            }

            context.target_fps = 60;

            return true;
        }

        self.dest_conn = None;
        // Dragging a connection, check for dest connection
        if let Some(source) = self.source_conn {
            for index in 0..self.nodes.len() {
                let rect= self.get_node_rect(index, false);

                if self.visible_node_ids.contains(&self.widget_index_to_node_id(index)) {
                    if context.contains_pos_for_isize(pos, rect) {

                        // Check for node terminals
                        for (conn, connector) in &self.nodes[index].node_connector {
                            let c_rect = (pos.0  as isize - rect.0, pos.1 as isize - rect.1);
                            if c_rect.0 > 0 && c_rect.1 > 0 {

                                if context.contains_pos_for((c_rect.0 as usize, c_rect.1 as usize), connector.rect) {

                                    // Check if nodes are different
                                    if index != source.1 {

                                        let source1 = self.connector_is_source(source.0);
                                        let source2 = self.connector_is_source(*conn);

                                        // We can connect if the two connectors are sourcee and dest
                                        if source1 != source2 {
                                            self.dest_conn = Some((*conn, index));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            self.dirty = true;
            return true;
        }

        // Preview
        if let Some(preview) = &mut self.preview {
            if self.preview_is_visible {
                let x = pos.0 as isize - preview.rect.0 as isize;
                let y = pos.1 as isize - preview.rect.1 as isize;

                let r_x = self.preview_drag_start.0 - pos.0 as isize;
                let r_y = pos.1 as isize - self.preview_drag_start.1;

                if preview.mouse_dragged((x as usize, y as usize), (r_x, r_y), asset, context) {
                    self.dirty = true;
                    //preview.rect = (self.rect.0 + self.rect.2 - preview.size.0, self.rect.1, preview.size.0, preview.size.1);

                    return  true;
                }
            }
        }

        false
    }

    fn mouse_wheel(&mut self, delta: (isize, isize), asset: &mut Asset, context: &mut ScreenContext, _options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {

        if let Some(preview) = &mut self.preview {
            if self.preview_is_visible {
                if context.contains_pos_for(self.mouse_hover_pos, preview.rect) {
                    preview.mouse_wheel(delta, asset, context);
                    self.dirty = true;
                    return true;
                }
            }
        }
        self.offset.0 += delta.0 / 20;
        self.offset.1 += delta.1 / 20;
        self.dirty = true;
        true
    }

    fn mouse_hover(&mut self, pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext, _options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {
        self.mouse_hover_pos = pos;
        false
    }

    /// Marks the two nodes as dirty
    fn changed_selection(&mut self, old_selection: usize, new_selection: usize) {
        if self.graph_mode == GraphMode::Overview {
            for index in 0..self.nodes.len() {
                if index == old_selection || index == new_selection {
                    self.nodes[index].dirty = true;
                    self.dirty = true;
                }
            }
        }
    }

    /// Mark all nodes as dirty
    fn mark_all_dirty(&mut self) {
        if self.graph_mode == GraphMode::Overview {
            for index in 0..self.nodes.len() {
                self.nodes[index].dirty = true;
            }
        }
        self.dirty = true;
    }

    /// Set the behavior id, this will take the bevhavior node data and create node widgets
    fn set_behavior_id(&mut self, id: Uuid, context: &mut ScreenContext) {

        self.nodes = vec![];
        self.behavior_tree_ids = vec![];
        self.curr_behavior_tree_id = None;

        self.behavior_id = id;
        let mut nodes = vec![];

        if let Some(behavior) = context.data.get_behavior(id, self.graph_type) {
            let mut ids = vec![];

            for (id, node) in &behavior.data.nodes {
                ids.push((id, node.name.clone()));
            }

            ids.sort_by(|x, y| x.1.cmp(&y.1));

            for (id, _name) in ids {
                let node_widget = NodeWidget::new_from_behavior_data(&behavior.data,  &behavior.data.nodes[id]);
                nodes.push(node_widget);
            }
        }

        // Init the nodes and add them
        for mut node_widget in nodes {
            self.init_node_widget(&mut node_widget, context);
            self.nodes.push(node_widget);
        }

        self.dirty = true;
        self.check_node_visibility(context);
        context.jump_to_position = context.data.get_behavior_default_position(id);
    }

    /// Adds the given node
    fn add_overview_node(&mut self, node: NodeWidget, context: &mut ScreenContext) {
        self.nodes.push(node);
        self.sort(context);
        self.dirty = true;
    }

    /// Adds a node of the type identified by its name
    fn add_node_of_name(&mut self, name: String, position: (isize, isize), context: &mut ScreenContext) {

        let mut node_widget : Option<NodeWidget> =  None;

        // Create the node
        if let Some(behavior) = context.data.get_mut_behavior(self.get_curr_behavior_id(context), self.graph_type) {

            let node_type = match name.as_str() {
                "Expression" => BehaviorNodeType::Expression,
                "Script" => BehaviorNodeType::Script,
                "Sequence" => BehaviorNodeType::Sequence,
                "Linear" => BehaviorNodeType::Linear,
                "Move" => BehaviorNodeType::Move,
                "Screen" => BehaviorNodeType::Screen,
                "Widget" => BehaviorNodeType::Widget,
                "Message" if self.graph_type != BehaviorType::Regions => BehaviorNodeType::Message,
                "Action" if self.graph_type != BehaviorType::Regions => BehaviorNodeType::Action,
                "Take" => BehaviorNodeType::Take,
                "Drop" => BehaviorNodeType::Drop,
                "Light" if self.graph_type == BehaviorType::Items => BehaviorNodeType::LightItem,
                "Set Tile" if self.graph_type == BehaviorType::Items => BehaviorNodeType::SetItemTile,
                "Random Walk" => BehaviorNodeType::RandomWalk,
                "Pathfinder" => BehaviorNodeType::Pathfinder,
                "Lookout" => BehaviorNodeType::Lookout,
                "Close In" => BehaviorNodeType::CloseIn,
                "Multi Choice" => BehaviorNodeType::MultiChoice,
                "Sell" => BehaviorNodeType::Sell,
                "Lock Tree" => BehaviorNodeType::LockTree,
                "Unlock" => BehaviorNodeType::UnlockTree,
                "Set State" => BehaviorNodeType::SetState,
                "Call System" => BehaviorNodeType::CallSystem,
                "Call Behavior" => BehaviorNodeType::CallBehavior,
                "Has State ?" => BehaviorNodeType::HasState,
                "Has Target ?" => BehaviorNodeType::HasTarget,
                "Untarget" => BehaviorNodeType::Untarget,
                "Deal Damage" => BehaviorNodeType::DealDamage,
                "Take Damage" => BehaviorNodeType::TakeDamage,
                "Magic Damage" => BehaviorNodeType::MagicDamage,
                "Drop Inv." => BehaviorNodeType::DropInventory,
                "Target" => BehaviorNodeType::Target,
                "Magic Target" => BehaviorNodeType::MagicTarget,
                "Teleport" if self.graph_type == BehaviorType::Regions => BehaviorNodeType::TeleportArea,
                "Teleport" => BehaviorNodeType::Teleport,
                "Audio" if self.graph_type == BehaviorType::Regions => BehaviorNodeType::AudioArea,
                "Audio" => BehaviorNodeType::Audio,
                "Effect" => BehaviorNodeType::Effect,
                "Heal" => BehaviorNodeType::Heal,
                "Take Heal" => BehaviorNodeType::TakeHeal,
                "Respawn" => BehaviorNodeType::Respawn,
                "Equip" => BehaviorNodeType::Equip,
                "Set Level Tree" => BehaviorNodeType::SetLevelTree,
                "Schedule" => BehaviorNodeType::Schedule,

                "Skill Level" if self.graph_type == BehaviorType::Items => BehaviorNodeType::SkillLevelItem,
                "Skill Tree" => BehaviorNodeType::SkillTree,
                "Skill Level" => BehaviorNodeType::SkillLevel,

                "Level Tree" => BehaviorNodeType::LevelTree,
                "Level" => BehaviorNodeType::Level,

                "Always" => BehaviorNodeType::Always,
                "Enter Area" => BehaviorNodeType::EnterArea,
                "Leave Area" => BehaviorNodeType::LeaveArea,
                "Inside Area" => BehaviorNodeType::InsideArea,
                "Message" if self.graph_type == BehaviorType::Regions => BehaviorNodeType::MessageArea,
                "Light" if self.graph_type == BehaviorType::Regions => BehaviorNodeType::LightArea,
                "Action" if self.graph_type == BehaviorType::Regions => BehaviorNodeType::ActionArea,

                "Overlay Tiles" => BehaviorNodeType::OverlayTiles,

                /*
                "Widget" => BehaviorNodeType::Widget,
                */

                _ => BehaviorNodeType::BehaviorTree
            };

            let id = behavior.add_node(node_type, name.clone());
            if let Some(node) = behavior.data.nodes.get_mut(&id) {
                node.position = position;
            }

            let node = NodeWidget::new_from_behavior_data(&behavior.data, &behavior.data.nodes.get(&id).unwrap());
            node_widget = Some(node);

            behavior.save_data();
        }

        if let Some(mut node) = node_widget {
            self.init_node_widget( &mut node, context);
            self.nodes.push(node);
        }

        self.check_node_visibility(context);
        self.dirty = true;
    }

    /// Inits the node widget (atom widgets, id)
    fn init_node_widget(&mut self, node_widget: &mut NodeWidget, context: &mut ScreenContext) {

        let behavior_data_id;
        let node_id;
        let node_behavior_type;

        if let Some(behavior) = context.data.get_mut_behavior(self.get_curr_behavior_id(context), self.graph_type) {

            behavior_data_id = behavior.data.id;

            if let Some(node) = behavior.data.nodes.get(&node_widget.id) {

                node_id = node.id;
                node_behavior_type = node.behavior_type;
            } else {
                return;
            }
        } else {
            return;
        }

        // menu button factory
        let mut create_menu_atom = |id: String, text: Vec<String>, def: Value| -> AtomWidget {
            let mut atom = AtomWidget::new(text, AtomWidgetType::NodeMenuButton, AtomData::new(id.to_lowercase().as_str(), Value::Empty()));
            atom.atom_data.text = id.clone();
            let id = (behavior_data_id, node_id, id.to_lowercase());
            atom.behavior_id = Some(id.clone());
            atom.atom_data.value = context.data.get_behavior_id_value(id, def, self.graph_type);
            if let Some(index) = atom.atom_data.value.to_integer() {
                atom.curr_index = index as usize;
            }
            atom
        };

        if node_behavior_type == BehaviorNodeType::BehaviorType {
            if self.graph_type == BehaviorType::Behaviors {
                node_widget.is_corner_node = true;

                let aligh_menu = create_menu_atom("Alignment".to_string(), vec!["Hero".to_string(), "Neutral".to_string(), "Monster".to_string()], Value::Integer(0));

                node_widget.widgets.push(aligh_menu);
                node_widget.color = context.color_black.clone();

                // Position
                let mut position_atom = AtomWidget::new(vec![], AtomWidgetType::NodePositionButton,
                AtomData::new("position", Value::Empty()));
                position_atom.atom_data.text = "position".to_string();
                let id = (behavior_data_id, node_id, "position".to_string());
                position_atom.behavior_id = Some(id.clone());
                //position_atom.atom_data.data = context.data.get_behavior_id_value(id, (-1.0,0.0,0.0,0.0, "".to_string()), self.graph_type);
                position_atom.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
                node_widget.widgets.push(position_atom);

                let mut character_settings = AtomWidget::new(vec!["Settings".to_string()], AtomWidgetType::NodeCharacterSettingsButton,
                AtomData::new("settings", Value::Empty()));
                character_settings.atom_data.text = "Settings".to_string();
                let id = (behavior_data_id, node_id, "settings".to_string());
                character_settings.behavior_id = Some(id.clone());
                let mut sink = PropertySink::new();
                update_item_sink(&mut sink);
                character_settings.atom_data.value = context.data.get_behavior_id_value(id, Value::PropertySink(sink), self.graph_type);
                node_widget.widgets.push(character_settings);

                // Default Character Tile
                let mut tile_atom = AtomWidget::new(vec![], AtomWidgetType::NodeCharTileButton,
                    AtomData::new("tile", Value::Empty()));
                tile_atom.atom_data.text = "tile".to_string();
                let id = (behavior_data_id, node_id, "tile".to_string());
                tile_atom.behavior_id = Some(id.clone());
                tile_atom.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
                node_widget.widgets.push(tile_atom);

                // Chunks
                let mut chunk_atom = AtomWidget::new(vec![], AtomWidgetType::NodePropertyLog,
                    AtomData::new("chunks", Value::Empty()));
                chunk_atom.atom_data.text = "chunks".to_string();
                let id = (behavior_data_id, node_id, "chunks".to_string());
                chunk_atom.behavior_id = Some(id.clone());
                chunk_atom.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
                node_widget.widgets.push(chunk_atom);
            } else
            if self.graph_type == BehaviorType::Items {
                node_widget.is_corner_node = true;

                // Default Character Tile
                let mut tile_atom = AtomWidget::new(vec![], AtomWidgetType::NodeIconTileButton,
                    AtomData::new("tile", Value::Empty()));
                tile_atom.atom_data.text = "tile".to_string();
                let id = (behavior_data_id, node_id, "tile".to_string());
                tile_atom.behavior_id = Some(id.clone());
                tile_atom.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
                node_widget.widgets.push(tile_atom);

                let mut item_settings;
                if self.sub_type == NodeSubType::Spell {
                    item_settings = AtomWidget::new(vec!["Settings".to_string()], AtomWidgetType::NodeSpellSettingsButton,AtomData::new("settings", Value::Empty()));
                } else {
                    item_settings = AtomWidget::new(vec!["Settings".to_string()], AtomWidgetType::NodeItemSettingsButton, AtomData::new("settings", Value::Empty()));
                }

                item_settings.atom_data.text = "Settings".to_string();
                let id = (behavior_data_id, node_id, "settings".to_string());
                item_settings.behavior_id = Some(id.clone());
                let mut sink = PropertySink::new();
                update_item_sink(&mut sink);
                item_settings.atom_data.value = context.data.get_behavior_id_value(id, Value::PropertySink(sink), self.graph_type);
                node_widget.widgets.push(item_settings);
            } else
            if self.graph_type == BehaviorType::GameLogic {
                node_widget.is_corner_node = true;

                // Name of the startup tree
                let mut startup_atom = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
                AtomData::new("startup", Value::Empty()));
                startup_atom.atom_data.text = "startup".to_string();
                let id = (behavior_data_id, node_id, "startup".to_string());
                startup_atom.behavior_id = Some(id.clone());
                //startup_atom.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "Game".to_string()), self.graph_type);
                startup_atom.atom_data.value = context.data.get_behavior_id_value(id, Value::String("Game".to_string()), self.graph_type);
                node_widget.widgets.push(startup_atom);
            }
            return;
        }

        // Node menu

        let mut menu_text : Vec<String> = vec!["Rename".to_string()];
        menu_text.push( "Disconnect".to_string());
        menu_text.push( "Delete".to_string());
        menu_text.push( "Help".to_string());

        let mut node_menu_atom = AtomWidget::new(menu_text, AtomWidgetType::NodeMenu,
        AtomData::new("menu", Value::Empty()));
        node_menu_atom.atom_data.text = "menu".to_string();
        let id = (behavior_data_id, node_id, "menu".to_string());
        node_menu_atom.behavior_id = Some(id.clone());
        node_widget.menu = Some(node_menu_atom);

        if node_behavior_type == BehaviorNodeType::BehaviorTree {
            let tree_menu = create_menu_atom("Execute".to_string(), vec!["Always".to_string(), "On Startup".to_string(), "On Demand".to_string()], Value::Integer(0));

            node_widget.widgets.push(tree_menu);
            node_widget.color = context.color_green.clone();

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#behavior-tree".to_string());

            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
            if self.graph_type != BehaviorType::GameLogic {
                node_widget.node_connector.insert(BehaviorNodeConnector::Bottom1, NodeConnector { rect: (0,0,0,0) } );
                node_widget.node_connector.insert(BehaviorNodeConnector::Bottom2, NodeConnector { rect: (0,0,0,0) } );
                node_widget.node_connector.insert(BehaviorNodeConnector::Bottom3, NodeConnector { rect: (0,0,0,0) } );
                node_widget.node_connector.insert(BehaviorNodeConnector::Bottom4, NodeConnector { rect: (0,0,0,0) } );
            }

            // Add the node to the behavior tree ids
            self.behavior_tree_ids.push(node_widget.id);
            if self.curr_behavior_tree_id == None {
                self.curr_behavior_tree_id = Some(node_widget.id);
            }
        } else
        if node_behavior_type == BehaviorNodeType::Linear {
            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#linear".to_string());
            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom1, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom2, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom3, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom4, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Sequence {
            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#sequence".to_string());
            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom1, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom2, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom3, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom4, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Expression {
            let mut atom1 = AtomWidget::new(vec!["Expression".to_string()], AtomWidgetType::NodeExpressionButton,
            AtomData::new("expression", Value::Empty()));
            atom1.atom_data.text = "Expression".to_string();
            let id = (behavior_data_id, node_id, "expression".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::String("".to_owned()), self.graph_type);
            node_widget.widgets.push(atom1);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#expression".to_string());

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Script {
            let mut atom1 = AtomWidget::new(vec!["Script".to_string()], AtomWidgetType::NodeScriptButton,
            AtomData::new("script", Value::Empty()));
            atom1.atom_data.text = "Script".to_string();
            let id = (behavior_data_id, node_id, "script".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::String("".to_owned()), self.graph_type);
            node_widget.widgets.push(atom1);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#script".to_string());

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Screen {

            let mut script_name = AtomWidget::new(vec!["Script Name".to_string()], AtomWidgetType::NodeTextButton,
            AtomData::new("script_name", Value::String("".to_string())));
            script_name.atom_data.text = "Script Name".to_string();
            let id = (behavior_data_id, node_id, "script_name".to_string());
            script_name.behavior_id = Some(id.clone());
            script_name.atom_data.value = context.data.get_behavior_id_value(id, Value::String("main.rhai".to_string()), self.graph_type);
            node_widget.widgets.push(script_name);

            let mut reveal_atom = AtomWidget::new(vec!["Reveal Scripts Folder".to_string()], AtomWidgetType::NodeRevealScriptsButton,
            AtomData::new("reveal_scripts", Value::String("".to_string())));
            reveal_atom.atom_data.text = "Reveal Scripts Folder".to_string();
            let id = (behavior_data_id, node_id, "reveal_scripts".to_string());
            reveal_atom.behavior_id = Some(id.clone());
            reveal_atom.atom_data.value = context.data.get_behavior_id_value(id, Value::String("Reveal".to_string()), self.graph_type);
            node_widget.widgets.push(reveal_atom);

            let mut edit_atom = AtomWidget::new(vec!["Edit Script".to_string()], AtomWidgetType::NodeScreenButton,
            AtomData::new("script", Value::Empty()));
            edit_atom.atom_data.text = "Edit Script".to_string();
            let id = (behavior_data_id, node_id, "script".to_string());
            edit_atom.behavior_id = Some(id.clone());
            // let mut def_text = "".to_string();
            // if let Some(txt) = context.scripts.get("screen") {
            //     def_text = txt.clone();
            // }
            edit_atom.atom_data.value = Value::String("Edit".to_string());//context.data.get_behavior_id_value(id, Value::String("Edit".to_string()), self.graph_type);
            node_widget.widgets.push(edit_atom);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#screen".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Widget {
            let mut atom1 = AtomWidget::new(vec!["Script".to_string()], AtomWidgetType::NodeScreenButton,
            AtomData::new("script", Value::Empty()));
            atom1.atom_data.text = "Script".to_string();
            let id = (behavior_data_id, node_id, "script".to_string());
            atom1.behavior_id = Some(id.clone());
            let mut def_text = "".to_string();
            if let Some(txt) = context.scripts.get("widget") {
                def_text = txt.clone();
            }
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::String(def_text), self.graph_type);
            node_widget.widgets.push(atom1);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#widget".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Move {
            let mut atom1 = AtomWidget::new(vec!["Speed Delay".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("speed", Value::Integer(0)));
            atom1.atom_data.text = "Speed Delay".to_string();
            let id = (behavior_data_id, node_id, "speed".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::Integer(8), self.graph_type);
            node_widget.widgets.push(atom1);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#move".to_string());

            node_widget.color = context.color_gray.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Target {
            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#target".to_string());
            node_widget.color = context.color_gray.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::MagicTarget {
            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#magic-target".to_string());
            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Message || node_behavior_type == BehaviorNodeType::MessageArea {

            let type_menu = create_menu_atom("Type".to_string(), vec!["Status".to_string(), "Say".to_string(), "Yell".to_string(), "Private".to_string(), "Debug".to_string()], Value::Integer(0));
            node_widget.widgets.push(type_menu);

            let mut atom2 = AtomWidget::new(vec!["Text".to_string()], AtomWidgetType::NodeTextButton,
            AtomData::new("text", Value::String("".to_string())));
            atom2.atom_data.text = "Text".to_string();
            let id = (behavior_data_id, node_id, "text".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::String("Message".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#message".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Action || node_behavior_type == BehaviorNodeType::ActionArea {

            let mut atom2 = AtomWidget::new(vec!["Action".to_string()], AtomWidgetType::NodeTextButton,
            AtomData::new("action", Value::String("".to_string())));
            atom2.atom_data.text = "Action".to_string();
            let id = (behavior_data_id, node_id, "action".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::String("Action".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#action".to_string());

            node_widget.color = context.color_blue.clone();
            if  node_behavior_type == BehaviorNodeType::Action {
                node_widget.color = context.color_gray.clone();
                node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
                node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
                node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
            } else {
                node_widget.color = context.color_green.clone();
                node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
            }
        } else
        if node_behavior_type == BehaviorNodeType::Take {
            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#take".to_string());
            node_widget.color = context.color_gray.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Drop {
            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#drop".to_string());
            node_widget.color = context.color_gray.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Equip {
            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#equip".to_string());
            node_widget.color = context.color_gray.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
       if node_behavior_type == BehaviorNodeType::RandomWalk {
            // Position
            let mut position_atom = AtomWidget::new(vec![], AtomWidgetType::NodePositionButton,
            AtomData::new("position", Value::Empty()));
            position_atom.atom_data.text = "Position".to_string();
            let id = (behavior_data_id, node_id, "position".to_string());
            position_atom.behavior_id = Some(id.clone());
            position_atom.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(position_atom);

            let mut max_distance = AtomWidget::new(vec!["Max Distance".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("max_distance", Value::Integer(0)));
            max_distance.atom_data.text = "Max Distance".to_string();
            let id = (behavior_data_id, node_id, "max_distance".to_string());
            max_distance.behavior_id = Some(id.clone());
            max_distance.atom_data.value = context.data.get_behavior_id_value(id, Value::String("4".to_string()), self.graph_type);
            node_widget.widgets.push(max_distance);

            let mut speed = AtomWidget::new(vec!["Speed Delay".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("speed", Value::Empty()));
            speed.atom_data.text = "Speed Delay".to_string();
            let id = (behavior_data_id, node_id, "speed".to_string());
            speed.behavior_id = Some(id.clone());
            speed.atom_data.value = context.data.get_behavior_id_value(id, Value::String("8".to_string()), self.graph_type);
            node_widget.widgets.push(speed);

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );

            let mut speed = AtomWidget::new(vec!["Delay".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("delay", Value::Empty()));
            speed.atom_data.text = "Delay".to_string();
            let id = (behavior_data_id, node_id, "delay".to_string());
            speed.behavior_id = Some(id.clone());
            speed.atom_data.value = context.data.get_behavior_id_value(id, Value::String("10".to_string()), self.graph_type);
            node_widget.widgets.push(speed);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#random-walk".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Pathfinder {
            let mut atom1 = AtomWidget::new(vec!["Destination".to_string()], AtomWidgetType::NodePositionButton,
            AtomData::new("destination", Value::Empty()));
            atom1.atom_data.text = "Destination".to_string();
            let id = (behavior_data_id, node_id, "destination".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec!["Speed Delay".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("speed", Value::Empty()));
            atom2.atom_data.text = "Speed Delay".to_string();
            let id = (behavior_data_id, node_id, "speed".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::String("8".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#pathfinder".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Lookout {
            let type_menu = create_menu_atom("State".to_string(), vec!["Normal".to_string(), "Killed".to_string(), "Sleeping".to_string(), "Intoxicated".to_string()], Value::Integer(0));
            node_widget.widgets.push(type_menu);

            let mut atom1 = AtomWidget::new(vec!["Expression".to_string()], AtomWidgetType::NodeExpressionButton,
            AtomData::new("expression", Value::Empty()));
            atom1.atom_data.text = "Expression".to_string();
            let id = (behavior_data_id, node_id, "expression".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec!["Max Distance".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("max_distance", Value::Empty()));
            atom2.atom_data.text = "Max Distance".to_string();
            let id = (behavior_data_id, node_id, "max_distance".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::String("7".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#lookout".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::CloseIn {
            let mut atom1 = AtomWidget::new(vec!["To Distance".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("to_distance", Value::Empty()));
            atom1.atom_data.text = "To Distance".to_string();
            let id = (behavior_data_id, node_id, "to_distance".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::String("1".to_string()), self.graph_type);
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec!["Speed Delay".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("speed", Value::Empty()));
            atom2.atom_data.text = "Speed Delay".to_string();
            let id = (behavior_data_id, node_id, "speed".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#close-in".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::MultiChoice {
            let mut header = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new("header", Value::Empty()));
            header.atom_data.text = "Header".to_string();
            let id = (behavior_data_id, node_id, "header".to_string());
            header.behavior_id = Some(id.clone());
            header.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(header);

            let mut atom1 = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new("text", Value::Empty()));
            atom1.atom_data.text = "Text".to_string();
            let id = (behavior_data_id, node_id, "text".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new("answer", Value::Empty()));
            atom2.atom_data.text = "Answer".to_string();
            let id = (behavior_data_id, node_id, "answer".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#multi-choice".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Sell {
            let mut header = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new("header", Value::Empty()));
            header.atom_data.text = "Header".to_string();
            let id = (behavior_data_id, node_id, "header".to_string());
            header.behavior_id = Some(id.clone());
            header.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(header);

            let mut exit = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new("exit", Value::Empty()));
            exit.atom_data.text = "Exit Text".to_string();
            let id = (behavior_data_id, node_id, "exit".to_string());
            exit.behavior_id = Some(id.clone());
            exit.atom_data.value = context.data.get_behavior_id_value(id, Value::String("Exit".to_string()), self.graph_type);
            node_widget.widgets.push(exit);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#sell".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::LockTree {
            // let target_menu = create_menu_atom("For".to_string(), vec!["Self".to_string(), "Target".to_string()], Value::Integer(0));
            // node_widget.widgets.push(target_menu);

            let mut atom2 = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new("tree", Value::Empty()));
            atom2.atom_data.text = "Tree Name".to_string();
            let id = (behavior_data_id, node_id, "tree".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#lock-tree".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::UnlockTree {
            // let target_menu = create_menu_atom("For".to_string(), vec!["Self".to_string(), "Target".to_string()], Value::Integer(0));
            // node_widget.widgets.push(target_menu);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#unlock-tree".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::SetState {
            let target_menu = create_menu_atom("For".to_string(), vec!["Self".to_string(), "Target".to_string()], Value::Integer(0));
            node_widget.widgets.push(target_menu);

            let state_menu = create_menu_atom("State".to_string(), vec!["Normal".to_string(), "Killed".to_string(), "Purged".to_string(), "Sleeping".to_string(), "Intoxicated".to_string()], Value::Integer(0));
            node_widget.widgets.push(state_menu);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#set-state".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::CallSystem {

            let mut atom1 = AtomWidget::new(context.data.systems_names.clone(), AtomWidgetType::NodeTextButton,
            AtomData::new("system", Value::Empty()));
            atom1.atom_data.text = "System Name".to_string();
            let id = (behavior_data_id, node_id, "system".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new("tree", Value::Empty()));
            atom2.atom_data.text = "Tree Name".to_string();
            let id = (behavior_data_id, node_id, "tree".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#call-system".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::CallBehavior {
            let mut atom1 = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new("tree", Value::Empty()));
            atom1.atom_data.text = "Tree Name".to_string();
            let id = (behavior_data_id, node_id, "tree".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom1);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#call-behavior".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::HasState {
            let type_menu = create_menu_atom("State".to_string(), vec!["Normal".to_string(), "Killed".to_string(), "Sleeping".to_string(), "Intoxicated".to_string()], Value::Integer(0));
            node_widget.widgets.push(type_menu);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#has-state".to_string());
            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::HasTarget {
            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#has-target".to_string());
            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Untarget {
            let mut atom2 = AtomWidget::new(vec!["If Distance Is Greater".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("distance", Value::Empty()));
            atom2.atom_data.text = "If Distance Is Greater".to_string();
            let id = (behavior_data_id, node_id, "distance".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::String("3".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#untarget".to_string());

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::DealDamage {
            let mut atom2 = AtomWidget::new(vec!["Attack Rating".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("attack_rating", Value::Empty()));
            atom2.atom_data.text = "Attack Rating".to_string();
            let id = (behavior_data_id, node_id, "attack_rating".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::String("0".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#deal-damage".to_string());

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::TakeDamage {
            let mut atom2 = AtomWidget::new(vec!["Reduce By".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("reduce by", Value::Empty()));
            atom2.atom_data.text = "Reduce By".to_string();
            let id = (behavior_data_id, node_id, "reduce by".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::String("0".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#take-damage".to_string());

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::MagicDamage {
            let mut atom2 = AtomWidget::new(vec!["Damage".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("damage", Value::Empty()));
            atom2.atom_data.text = "Damage".to_string();
            let id = (behavior_data_id, node_id, "damage".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::String("0".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#magic-damage".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::DropInventory {
            let type_menu = create_menu_atom("Drop".to_string(), vec!["Everything".to_string(), "Random Item".to_string()], Value::Integer(0));

            node_widget.widgets.push(type_menu);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#drop-inventory".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
       if node_behavior_type == BehaviorNodeType::Teleport {
            // Position
            let mut position_atom = AtomWidget::new(vec![], AtomWidgetType::NodePositionButton,
            AtomData::new("position", Value::Empty()));
            position_atom.atom_data.text = "Position".to_string();
            let id = (behavior_data_id, node_id, "position".to_string());
            position_atom.behavior_id = Some(id.clone());
            position_atom.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(position_atom);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#teleport".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
       if node_behavior_type == BehaviorNodeType::Effect {
            let mut effect_atom = AtomWidget::new(vec![], AtomWidgetType::NodeEffectTileButton,
            AtomData::new("effect", Value::Empty()));
            effect_atom.atom_data.text = "Effect".to_string();
            let id = (behavior_data_id, node_id, "effect".to_string());
            effect_atom.behavior_id = Some(id.clone());
            effect_atom.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(effect_atom);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#effect".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Audio {
            let mut atom1 = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new("audio", Value::Empty()));
            atom1.atom_data.text = "Audio".to_string();
            let id = (behavior_data_id, node_id, "audio".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom1);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#audio".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Heal {
            let target_menu = create_menu_atom("For".to_string(), vec!["Self".to_string(), "Target".to_string()], Value::Integer(0));
            node_widget.widgets.push(target_menu);

            let mut atom2 = AtomWidget::new(vec!["Amount".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("amount", Value::Empty()));
            atom2.atom_data.text = "Amount".to_string();
            let id = (behavior_data_id, node_id, "amount".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::String("0".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            let mut atom1 = AtomWidget::new(vec!["Speed Delay".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("speed", Value::Empty()));
            atom1.atom_data.text = "Speed Delay".to_string();
            let id = (behavior_data_id, node_id, "speed".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::String(7.to_string()), self.graph_type);
            node_widget.widgets.push(atom1);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#heal".to_string());

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::TakeHeal {
            let mut atom2 = AtomWidget::new(vec!["Increase By".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("increase by", Value::Empty()));
            atom2.atom_data.text = "Increase By".to_string();
            let id = (behavior_data_id, node_id, "increase by".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::String("0".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#take-heal".to_string());

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Respawn {
            let mut atom2 = AtomWidget::new(vec!["Minutes to Wait".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("minutes", Value::Empty()));
            atom2.atom_data.text = "Minutes to Wait".to_string();
            let id = (behavior_data_id, node_id, "minutes".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::String("30".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#respawn".to_string());

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Schedule {
            let mut atom1 = AtomWidget::new(vec!["From".to_string()], AtomWidgetType::NodeTimeButton,
            AtomData::new("from", Value::Empty()));
            atom1.atom_data.text = "From".to_string();
            let id = (behavior_data_id, node_id, "from".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::Date(Date::new_time(0, 0)), self.graph_type);
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec!["To".to_string()], AtomWidgetType::NodeTimeButton,
            AtomData::new("to", Value::Empty()));
            atom2.atom_data.text = "To".to_string();
            let id = (behavior_data_id, node_id, "to".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::Date(Date::new_time(23, 59)), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#schedule".to_string());

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else

        // Area
        if node_behavior_type == BehaviorNodeType::InsideArea {
            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#inside-area".to_string());
            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::Always {
            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#always".to_string());
            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::EnterArea {
            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#enter-area".to_string());
            let menu = create_menu_atom("Character".to_string(), vec!["Everyone".to_string(), "First Only".to_string()], Value::Integer(0));
            node_widget.widgets.push(menu);

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::LeaveArea {
            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#leave-area".to_string());
            let menu = create_menu_atom("Character".to_string(), vec!["Everyone".to_string(), "Last Only".to_string()], Value::Integer(0));
            node_widget.widgets.push(menu);

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
        } else
       if node_behavior_type == BehaviorNodeType::TeleportArea {
            // Position
            let mut position_atom = AtomWidget::new(vec![], AtomWidgetType::NodePositionButton,
            AtomData::new("position", Value::Empty()));
            position_atom.atom_data.text = "Position".to_string();
            let id = (behavior_data_id, node_id, "position".to_string());
            position_atom.behavior_id = Some(id.clone());
            position_atom.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(position_atom);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#teleport".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::AudioArea {
            let mut atom1 = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new("audio", Value::Empty()));
            atom1.atom_data.text = "Audio".to_string();
            let id = (behavior_data_id, node_id, "audio".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom1);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#audio".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::LightArea {
            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#light".to_string());
            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::OverlayTiles {
            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#overlay-tiles".to_string());
            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
        } else

        // Items
        if node_behavior_type == BehaviorNodeType::LightItem {
            let menu = create_menu_atom("State".to_string(), vec!["Off".to_string(), "On".to_string()], Value::Integer(0));
            node_widget.widgets.push(menu);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#light".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::SetItemTile {

            // Default Character Tile
            let mut tile_atom = AtomWidget::new(vec![], AtomWidgetType::NodeIconTileButton,
                AtomData::new("tile", Value::Empty()));
            tile_atom.atom_data.text = "Tile".to_string();
            let id = (behavior_data_id, node_id, "tile".to_string());
            tile_atom.behavior_id = Some(id.clone());
            tile_atom.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(tile_atom);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#set-tile".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_behavior_type == BehaviorNodeType::SetLevelTree {

            let mut atom1 = AtomWidget::new(context.data.systems_names.clone(), AtomWidgetType::NodeTextButton,
            AtomData::new("system", Value::Empty()));
            atom1.atom_data.text = "System Name".to_string();
            let id = (behavior_data_id, node_id, "system".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new("tree", Value::Empty()));
            atom2.atom_data.text = "Tree Name".to_string();
            let id = (behavior_data_id, node_id, "tree".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#level-tree".to_string());

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else

        // System

        if node_behavior_type == BehaviorNodeType::SkillTree {
            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#skill-tree".to_string());

            node_widget.color = context.color_orange.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );

            // Add the node to the behavior tree ids
            self.behavior_tree_ids.push(node_widget.id);
            if self.curr_behavior_tree_id == None {
                self.curr_behavior_tree_id = Some(node_widget.id);
            }
        } else
        if node_behavior_type == BehaviorNodeType::SkillLevel {
            let mut atom1 = AtomWidget::new(vec![], AtomWidgetType::NodeNumberButton,
            AtomData::new("start", Value::Empty()));
            atom1.atom_data.text = "Starts at".to_string();
            let id = (behavior_data_id, node_id, "start".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new("message", Value::Empty()));
            atom2.atom_data.text = "Message".to_string();
            let id = (behavior_data_id, node_id, "message".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#skill-level".to_string());

            node_widget.color = context.color_orange.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else

        if node_behavior_type == BehaviorNodeType::LevelTree {

            let mut atom1 = AtomWidget::new(vec!["Experience for Kill".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("experience_kill", Value::Empty()));
            atom1.atom_data.text = "Experience for Kill".to_string();
            let id = (behavior_data_id, node_id, "experience_kill".to_string());
            atom1.behavior_id = Some(id.clone());
            let mut def_text = "".to_string();
            if let Some(txt) = context.scripts.get("level_tree_kill") {
                def_text = txt.clone();
            }
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::String(def_text), self.graph_type);
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new("message", Value::Empty()));
            atom2.atom_data.text = "Experience Message".to_string();
            let id = (behavior_data_id, node_id, "message".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::String("You gained {} experience.".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#level-tree".to_string());

            node_widget.color = context.color_orange.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );

            // Add the node to the behavior tree ids
            self.behavior_tree_ids.push(node_widget.id);
            if self.curr_behavior_tree_id == None {
                self.curr_behavior_tree_id = Some(node_widget.id);
            }
        } else
        if node_behavior_type == BehaviorNodeType::Level {
            let mut atom1 = AtomWidget::new(vec![], AtomWidgetType::NodeNumberButton,
            AtomData::new("start", Value::Empty()));
            atom1.atom_data.text = "Starts at".to_string();
            let id = (behavior_data_id, node_id, "start".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new("message", Value::Empty()));
            atom2.atom_data.text = "Message".to_string();
            let id = (behavior_data_id, node_id, "message".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom2);

            let mut atom3 = AtomWidget::new(vec![], AtomWidgetType::NodeScriptButton,
            AtomData::new("script", Value::Empty()));
            atom3.atom_data.text = "Script".to_string();
            let id = (behavior_data_id, node_id, "script".to_string());
            atom3.behavior_id = Some(id.clone());
            atom3.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom3);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#level-tree".to_string());

            node_widget.color = context.color_orange.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else

        // Item

        if node_behavior_type == BehaviorNodeType::SkillLevelItem {
            let mut atom2 = AtomWidget::new(vec![], AtomWidgetType::NodeScriptButton,
            AtomData::new("script", Value::Empty()));
            atom2.atom_data.text = "Script".to_string();
            let id = (behavior_data_id, node_id, "script".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.value = context.data.get_behavior_id_value(id, Value::Empty(), self.graph_type);
            node_widget.widgets.push(atom2);

            let mut atom1 = AtomWidget::new(vec!["Speed Delay".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new("speed", Value::Empty()));
            atom1.atom_data.text = "Speed Delay".to_string();
            let id = (behavior_data_id, node_id, "speed".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.value = context.data.get_behavior_id_value(id, Value::String(4.to_string()), self.graph_type);
            node_widget.widgets.push(atom1);

            node_widget.help_link = Some("https://eldiron.com/reference/nodes/index.html#skill-level".to_string());

            node_widget.color = context.color_orange.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        }

    }

    /// Converts the index of a node widget to a node id
    fn widget_index_to_node_id(&self, index: usize) -> Uuid {
        self.nodes[index].id
    }

    /// Converts the id of a node to a widget index
    fn node_id_to_widget_index(&self, id: Uuid) -> usize {
        for index in 0..self.nodes.len() {
            if self.nodes[index].id == id {
                return index;
            }
        }
        0
    }

    /// Returns true if the node connector is a source connector (Right or Bottom)
    fn connector_is_source(&self, connector: BehaviorNodeConnector) -> bool {
        if connector == BehaviorNodeConnector::Right || connector == BehaviorNodeConnector::Bottom || connector == BehaviorNodeConnector::Success || connector == BehaviorNodeConnector::Fail || connector == BehaviorNodeConnector::Bottom1 || connector == BehaviorNodeConnector::Bottom2 || connector == BehaviorNodeConnector::Bottom3 || connector == BehaviorNodeConnector::Bottom4 {
            return true;
        }
        false
    }

    /// Disconnect the node from all connections
    fn disconnect_node(&mut self, id: Uuid, context: &mut ScreenContext) {
        if let Some(behavior) = context.data.get_mut_behavior(self.get_curr_behavior_id(context), self.graph_type) {
            let mut nothing_to_remove = false;
            while nothing_to_remove == false {
                nothing_to_remove = true;

                for (index, connection) in behavior.data.connections.iter().enumerate() {
                    if connection.0 == id || connection.2 == id {
                        //to_remove.push(index);
                        behavior.data.connections.remove(index);
                        nothing_to_remove = false;
                        break;
                    }
                }
            }
            behavior.save_data();
        }
        self.check_node_visibility(context);
    }

    /// Disconnect the node from all connections
    fn delete_node(&mut self, id: Uuid, context: &mut ScreenContext) {
        self.disconnect_node(id, context);

        // Remove node widget
        for index in 0..self.nodes.len() {
            if self.nodes[index].id == id {
                self.nodes.remove(index);
                if let Some(rem_index) = self.behavior_tree_ids.iter().position(|&x| x == id) {
                    self.behavior_tree_ids.remove(rem_index);
                }
                break
            }
        }

        // Remove node data
        if let Some(behavior) = context.data.get_mut_behavior(self.get_curr_behavior_id(context), self.graph_type) {
            behavior.data.nodes.remove(&id);
            behavior.save_data();
        }
    }

    /// Sets the widget and behavior data for the given atom id
    fn set_node_atom_data(&mut self, node_atom_id: (Uuid, Uuid, String), value: Value, context: &mut ScreenContext) {
        for index in 0..self.nodes.len() {
            if self.nodes[index].id == node_atom_id.1 {
                for atom_index in 0..self.nodes[index].widgets.len() {
                    if self.nodes[index].widgets[atom_index].atom_data.id == node_atom_id.2 {
                        self.nodes[index].widgets[atom_index].atom_data.value = value.clone();
                        self.nodes[index].widgets[atom_index].dirty = true;
                        self.nodes[index].dirty = true;
                        self.dirty = true;

                        context.data.set_behavior_id_value(node_atom_id.clone(), value.clone(), self.graph_type);

                        break;
                    }
                }
            }
        }
    }

    /// Checks the visibility of a node
    fn check_node_visibility(&mut self, context: &ScreenContext) {

        self.visible_node_ids = vec![];

        if self.graph_type == BehaviorType::Regions {
            // Regions all nodes are visible
            for index in 0..self.nodes.len() {
                self.visible_node_ids.push(self.widget_index_to_node_id(index));
            }
            return;
        }

        if let Some(tree_id) = self.curr_behavior_tree_id {

            self.visible_node_ids.push(tree_id);
            self.mark_connections_visible(tree_id, context);

            // Find unconnected nodes and mark them visible
            for index in 0..self.nodes.len() {

                let mut connected = false;
                if let Some(behavior) = context.data.get_behavior(self.get_curr_behavior_id(context), self.graph_type) {

                    // Skip behavior tree nodes
                    if let Some(node_data) = behavior.data.nodes.get(&self.widget_index_to_node_id(index)) {
                        if node_data.behavior_type == BehaviorNodeType::BehaviorTree || node_data.behavior_type == BehaviorNodeType::SkillTree || node_data.behavior_type == BehaviorNodeType::LevelTree {
                            continue;
                        }
                    }

                    // Check if node is connected
                    for (source_node_id , _source_connector, dest_node_id, _dest_connector) in &behavior.data.connections {
                        if self.nodes[index].id == *source_node_id || self.nodes[index].id == *dest_node_id {
                            connected = true;
                            break;
                        }
                    }
                }

                if connected == false {
                    self.visible_node_ids.push(self.nodes[index].id);
                } else {
                    // If node is connected go up the tree to see if they are standalone nodes (i.e. not connectected to any behavior tree)
                    if self.belongs_to_standalone_branch(self.nodes[index].id, context) {
                        self.visible_node_ids.push(self.nodes[index].id);
                    }
                }
            }
        } else {
            // If there is no behavior tree mark all nodes as visible
            for index in 0..self.nodes.len() {
                self.visible_node_ids.push(self.widget_index_to_node_id(index));
            }
        }
    }

    /// Marks all connected nodes as visible
    fn mark_connections_visible(&mut self, id: Uuid, context: &ScreenContext) {
        if let Some(behavior) = context.data.get_behavior(self.get_curr_behavior_id(context), self.graph_type) {
            for (source_node_id , _source_connector, dest_node_id, _dest_connector) in &behavior.data.connections {
                if *source_node_id == id {
                    self.visible_node_ids.push(*dest_node_id);
                    self.mark_connections_visible(*dest_node_id, context);
                }
            }
        }
    }

    /// Checks if the given node id is part of an unconnected branch.
    fn belongs_to_standalone_branch(&mut self, id: Uuid, context: &ScreenContext) -> bool {
        if let Some(behavior) = context.data.get_behavior(self.get_curr_behavior_id(context), self.graph_type) {

            for (source_node_id , _source_connector, dest_node_id, _dest_connector) in &behavior.data.connections {
                if *dest_node_id == id {
                    return self.belongs_to_standalone_branch(*source_node_id, context);
                }
            }

            if let Some(node_data) = behavior.data.nodes.get(&id) {
                if node_data.behavior_type != BehaviorNodeType::BehaviorTree && node_data.behavior_type != BehaviorNodeType::SkillTree && node_data.behavior_type != BehaviorNodeType::LevelTree {
                    return true;
                }
            }
        }
        false
    }

    /// Collects the children indices of the given node id so that they can all be dragged at once
    fn collect_drag_children_indices(&mut self, id: Uuid, context: &ScreenContext) {
        if let Some(behavior) = context.data.get_behavior(self.get_curr_behavior_id(context), self.graph_type) {
            for (source_node_id , _source_connector, dest_node_id, _dest_connector) in &behavior.data.connections {
                if *source_node_id == id {
                    let index = self.node_id_to_widget_index(*dest_node_id);
                    if self.drag_indices.contains(&index) == false {
                        self.drag_indices.push(index);
                        self.drag_node_pos.push((self.nodes[index].user_data.position.0 as isize, self.nodes[index].user_data.position.1 as isize));
                    }
                    self.collect_drag_children_indices(*dest_node_id, context);
                }
            }
        }
    }

    /// Get the node vec
    fn get_nodes(&mut self) -> Option<&mut Vec<NodeWidget>> {
        Some(&mut self.nodes)
    }

    /// Get the rect
    fn get_rect(&self) -> (usize, usize, usize, usize) { self.rect.clone() }

    /// Get the offset
    fn get_offset(&self) -> (isize, isize) { self.offset.clone() }

    /// Needs redraw
    fn set_dirty(&mut self) {
        self.dirty = true;
    }

    /// Get the preview widget
    fn get_preview_widget(&mut self) -> Option<&mut NodePreviewWidget> {
        if self.preview.is_some() {
            return self.preview.as_mut();
        }
        None
    }

    /// Returns the behavior id for the current behavior and graph type
    fn get_curr_behavior_id(&self, context: &ScreenContext) -> Uuid {
        if self.graph_type == BehaviorType::Regions {
            if let Some(region) = context.data.regions.get(&context.data.regions_ids[context.curr_region_index]) {
                return region.behaviors[context.curr_region_area_index].data.id;
            }
        } else
        if self.graph_type == BehaviorType::Behaviors {
            return context.data.behaviors_ids[context.curr_behavior_index];
        } else
        if self.graph_type == BehaviorType::Systems {
            return context.data.systems_ids[context.curr_systems_index];
        } else
        if self.graph_type == BehaviorType::Items {

            let active_indices = self.get_active_indices();
            if let Some(index) = active_indices.iter().position(|&r| r == context.curr_items_index) {
                if self.sub_type == NodeSubType::Item {
                    return context.data.items_ids[index];
                } else
                if self.sub_type == NodeSubType::Spell {
                   return context.data.spells_ids[index];
                }
            }
        }
        Uuid::new_v4()
    }

    /// Returns the current node id for the given graph type
    fn get_curr_node_id(&self, context: &ScreenContext) -> Option<Uuid> {
        if let Some(behavior) = context.data.get_behavior(self.behavior_id, self.graph_type) {
            return behavior.data.curr_node_id;
        }
        None
    }

    /// Executed connections and script errors are passed here
    fn debug_data(&mut self, _context: &mut ScreenContext, data: BehaviorDebugData) {
        for (id, _error) in &data.script_errors {
            if id.0 == self.behavior_id {
                for n in &mut self.nodes {
                    if n.id == id.1 {
                        for w in &mut n.widgets {
                            if w.atom_data.id == id.2 {
                                w.debug_value = Some(1.0);
                                w.dirty = true;
                                n.dirty = true;
                                self.dirty = true;
                            }
                        }
                    }
                }
            }
        }
        self.dirty = true;
        self.behavior_debug_data = Some(data);
        if let Some(preview) = &mut self.preview {
            preview.dirty = true;
        }
    }

    /// A game debug update
    fn debug_update(&mut self, update: GameUpdate, context: &mut ScreenContext) {

        context.debug_log_messages.append(&mut update.messages.clone());
        context.debug_log_inventory = update.inventory.clone();
        context.debug_log_variables.clear();

        for key in update.scope_buffer.values.keys().sorted() {
            let v = &update.scope_buffer.values[key];
           context.debug_log_variables.push((key.to_string(), v.clone()));
        }

        // Update property log
        if self.graph_type == BehaviorType::Behaviors {
            if let Some(corner_index) = self.corner_index {
                if corner_index < self.nodes.len() && self.nodes[corner_index].is_corner_node {
                    self.nodes[corner_index].widgets[3].dirty = true;
                    self.nodes[corner_index].dirty = true;
                    self.dirty = true;
                }
            }
        }

        // Update the preview
        if let Some(preview) = &mut self.preview {
            preview.debug_update(update, context);
        }
    }

    /// Debugging stopped
    fn debugging_stopped(&mut self) {
        // Clear errors from all nodes
        for n in &mut self.nodes {
            for w in &mut n.widgets {
                w.debug_value = None;
                w.dirty = true;
                n.dirty = true;
                self.dirty = true;
            }
        }
        self.behavior_debug_data = None;
        self.dirty = true;
    }

    /// Get the sub type
    fn get_sub_node_type(&mut self) -> NodeSubType {
        self.sub_type
    }

    /// Set the sub type
    fn set_sub_node_type(&mut self, sub_type: NodeSubType, context: &mut ScreenContext) {
        self.sub_type = sub_type;
        self.sort(context);
        self.dirty = true;
    }

    /// Sort the items
    fn sort(&mut self, context: &mut ScreenContext) {

        if self.graph_mode == GraphMode::Detail { return; }

        let item_width = (280 + 20) as isize;
        let item_height = (120 + 20) as isize;
        let per_row = self.rect.2 as isize / item_width;

        let mut indices = vec![];

        if self.graph_type == BehaviorType::Tiles {
            let mut c = 0;
            for index in 0..self.nodes.len() {
                if self.nodes[index].sub_type == self.sub_type {
                    c += 1;
                    indices.push(index);
                    self.nodes[index].visible = true;
                } else {
                    self.nodes[index].visible = false;
                }
            }

            for i in 0..c {
                let index = indices[i];
                self.nodes[index].user_data.position = (20 + (i as isize % per_row) * item_width, 20 + (i as isize / per_row) * item_height);
                self.nodes[index].dirty = true;
            }

            if indices.len() > 0 && !indices.contains(&context.curr_tileset_index) {
                context.curr_tileset_index = indices[0];
            }
        } else
        if self.graph_type == BehaviorType::Items {
            let mut c = 0;
            for index in 0..self.nodes.len() {
                if self.nodes[index].sub_type == self.sub_type {
                    c += 1;
                    indices.push(index);
                    self.nodes[index].visible = true;
                } else {
                    self.nodes[index].visible = false;
                }
            }

            for i in 0..c {
                let index = indices[i];
                self.nodes[index].user_data.position = (20 + (i as isize % per_row) * item_width, 20 + (i as isize / per_row) * item_height);
                self.nodes[index].dirty = true;
            }

            if indices.len() > 0 && !indices.contains(&context.curr_items_index) {
                context.curr_items_index = indices[0];
            }
        } else {
            for index in 0..self.nodes.len() {
                self.nodes[index].user_data.position = (20 + (index as isize % per_row) * item_width, 20 + (index as isize / per_row) * item_height);
                self.nodes[index].visible = true;
                self.nodes[index].dirty = true;
                indices.push(index);
            }
        }

        self.active_indices = indices.clone();
    }

    /// Get the currently active indices in the node graph
    fn get_active_indices(&self) -> Vec<usize> {
        self.active_indices.clone()
    }

    fn set_active_indices(&mut self, indices: Vec<usize> ) {
        self.active_indices = indices;
    }

    fn key_down(&mut self, _char: Option<char>, key: Option<WidgetKey>, _asset: &mut Asset, context: &mut ScreenContext, _options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {
        if context.is_running { return false; }
        if let Some(key) = key {
            if key == WidgetKey::Left {
                self.offset.0 -= 20;
                self.dirty = true;
                return true;
            } else
            if key == WidgetKey::Right {
                self.offset.0 += 20;
                self.dirty = true;
                return true;
            } else
            if key == WidgetKey::Up {
                self.offset.1 -= 20;
                self.dirty = true;
                return true;
            } else
            if key == WidgetKey::Down {
                self.offset.1 += 20;
                self.dirty = true;
                return true;
            }
        }
        return false;
    }

}