use crate::editor::node::{NodeConnector, NodeWidget};
use crate::atom:: { AtomData, AtomWidget, AtomWidgetType };
use crate::editor::node_preview::NodePreviewWidget;
use crate::editor::dialog::{ DialogState, DialogEntry };
use crate::widget::Widget;

use zeno::{Mask, Stroke};
use crate::editor::ToolBar;

use core_server::gamedata::behavior::{GameBehaviorData, BehaviorNodeType, BehaviorNode, BehaviorNodeConnector, BehaviorType };

use core_shared::asset::Asset;
use crate::editor::{ ScreenContext };

use itertools::Itertools;

use crate::editor::traits::{ EditorOptions, EditorContent, GraphMode };

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

    mouse_pos                   : (usize, usize),
    mouse_hover_pos             : (usize, usize),

    behavior_tree_indices       : Vec<usize>,
    behavior_tree_rects         : Vec<(usize, usize, usize, usize)>,
    curr_behavior_tree_index    : Option<usize>,

    visible_node_ids            : Vec<usize>,
    behavior_id                 : usize,
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

            mouse_pos                   : (0,0),
            mouse_hover_pos             : (0, 0),

            behavior_tree_indices       : vec![],
            behavior_tree_rects         : vec![],
            curr_behavior_tree_index    : None,

            visible_node_ids            : vec![],

            behavior_id                 : 0,
        }
    }

    /// Update the editor and the preview, this is called during debugging
    fn update(&mut self, _context: &mut ScreenContext) {
        self.dirty = true;
        if let Some(preview) = &mut self.preview {
            preview.dirty = true;
        }
    }

    fn set_mode(&mut self, mode: GraphMode, context: &ScreenContext) {
        if mode == GraphMode::Detail && (self.graph_type == BehaviorType::Behaviors || self.graph_type == BehaviorType::Systems || self.graph_type == BehaviorType::GameLogic) && self.preview.is_none() {
            self.preview = Some(NodePreviewWidget::new(context, self.graph_type));
        }
        self.graph_mode = mode;
    }

    fn set_mode_and_rect(&mut self, mode: GraphMode, rect: (usize, usize, usize, usize), context: &ScreenContext) {
        if mode == GraphMode::Detail && (self.graph_type == BehaviorType::Behaviors || self.graph_type == BehaviorType::Systems || self.graph_type == BehaviorType::GameLogic) && self.preview.is_none() {
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

    fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.buffer.resize(width * height * 4, 0);
        self.dirty = true;
        self.rect.2 = width;
        self.rect.3 = height;
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, _options: &mut Option<Box<dyn EditorOptions>>) {
        let safe_rect = (0_usize, 0_usize, self.rect.2, self.rect.3);

        // Stopped running ?
        if context.just_stopped_running {
            // If preview just stopped running, mark all variables as dirty (as they may have changed)
            for index in 0..self.nodes.len() {
                if self.nodes[index].widgets.len() == 1 && self.nodes[index].widgets[0].atom_widget_type == AtomWidgetType::NodeNumberButton {
                    self.nodes[index].dirty = true;
                    self.nodes[index].widgets[0].dirty = true;
                    self.dirty = true;
                }
            }
        }

        if self.dirty {
            context.draw2d.draw_square_pattern(&mut self.buffer[..], &safe_rect, safe_rect.2, &[44, 44, 46, 255], &[56, 56, 56, 255], 40);

            if self.graph_mode == GraphMode::Overview {
                for index in 0..self.nodes.len() {

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
                        if self.graph_type == BehaviorType::Tiles {
                            // For tile maps draw the default_tile
                            if let Some(map)= asset.tileset.maps.get_mut(&asset.tileset.maps_ids[index]) {
                                if let Some(default_tile) = map.settings.default_tile {
                                    context.draw2d.draw_animated_tile(&mut preview_buffer[..], &(0, 0), map, 100, &default_tile, 0, 100);
                                }
                            }
                        } else
                        if self.graph_type == BehaviorType::Regions {
                            // For regions draw the center of the map
                            if let Some(region)= context.data.regions.get_mut(&context.data.regions_ids[index]) {
                                let offset = region.get_center_offset_for_visible_size((10, 10));
                                context.draw2d.draw_region(&mut preview_buffer[..], region, &(0, 0, 100, 100), &offset, 100, 10, anim_counter, asset);
                            }
                        } else
                        if self.graph_type == BehaviorType::Behaviors {
                            // Draw the main behavior tile
                            if let Some(tile_id) = context.data.get_behavior_default_tile(context.data.behaviors_ids[index]) {
                                if let Some(map)= asset.tileset.maps.get_mut(&tile_id.0) {
                                    context.draw2d.draw_animated_tile(&mut preview_buffer[..], &(0, 0), map, 100, &(tile_id.1, tile_id.2), 0, 100);
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
                let mut variables : Vec<usize> = vec![];

                // Draw nodes
                for index in 0..self.nodes.len() {

                    // Check for corner node
                    if self.nodes[index].is_corner_node {
                        corner_index = Some(index);
                        continue;
                    }

                    // Check for variable node
                    if self.nodes[index].is_variable_node {
                        variables.push(index);
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

                for index in 0..variables.len() {

                    let vindex = variables[index];

                    // If the preview is running, check if this node represents a variable which has been changed
                    // And if yes, marks it for redraw
                    if context.is_running {
                        for i in 0..context.data.changed_variables.len() {
                            if context.data.changed_variables[i].2 == self.nodes[vindex].id {
                                self.nodes[vindex].dirty = true;
                                for w in 0..self.nodes[vindex].widgets.len() {
                                    self.nodes[vindex].widgets[w].dirty = true;
                                }
                            }
                        }
                    }

                    if self.nodes[vindex].dirty {
                        let selected = if Some(self.nodes[index].id) == self.get_curr_node_id(context) { true } else { false };
                        self.nodes[vindex].draw(frame, anim_counter, asset, context, selected);
                    }

                    let rect= self.get_node_rect(vindex, true);
                    self.nodes[vindex].graph_offset = (rect.0, rect.1);
                    self.nodes[vindex].graph_offset = (rect.0, rect.1);
                    context.draw2d.blend_slice_safe(&mut self.buffer[..], &self.nodes[vindex].buffer[..], &rect, safe_rect.2, &safe_rect);
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

                            if context.data.executed_connections.contains(&(self.graph_type, *source_node_id, *source_connector)) {
                                orange_path += format!("M {},{} C {},{} {},{} {},{}", start_x, start_y, control_start_x, control_start_y, control_end_x, control_end_y, end_x, end_y).as_str();
                            } else {
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

                // Render the preview widget
                if let Some(preview) = &mut self.preview {
                    preview.draw(frame, anim_counter, asset, context);
                    preview.rect = (self.rect.0 + self.rect.2 - preview.size.0, self.rect.1, preview.size.0, preview.size.1);
                    preview.graph_offset = (preview.rect.0 as isize, preview.rect.1 as isize);
                    context.draw2d.blend_slice(&mut self.buffer[..], &mut preview.buffer[..], &(self.rect.2 - preview.size.0, 0, preview.size.0, preview.size.1), safe_rect.2);
                }

                // Render the behavior tree buttons

                self.behavior_tree_rects = vec![];

                let left_start = if self.graph_type == BehaviorType::Behaviors || self.graph_type == BehaviorType::GameLogic { 180 } else { 5 };
                let mut total_width = safe_rect.2 - left_start - 5;
                if let Some(preview) = &mut self.preview {
                    total_width -= preview.size.0;
                }
                let mut bt_rect = (left_start, 3, 170, 25);
                for bt_index in &self.behavior_tree_indices {

                    let mut selected = false;
                    if let Some(curr_index) = self.curr_behavior_tree_index {
                        if curr_index == *bt_index {
                            selected = true;
                        }
                    }

                    let color = if selected { context.color_gray } else {context.color_black };
                    context.draw2d.draw_rounded_rect(&mut self.buffer[..], &bt_rect, safe_rect.2, &((bt_rect.2) as f64, (bt_rect.3) as f64), &color, &(0.0, 0.0, 0.0, 0.0));

                    self.behavior_tree_rects.push(bt_rect.clone());

                    context.draw2d.draw_text_rect(&mut self.buffer[..], &bt_rect, safe_rect.2, &asset.get_editor_font("OpenSans"), 16.0, &self.nodes[*bt_index].text[0], &context.color_white, &color, crate::draw2d::TextAlignment::Center);

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
            let node_offset = preview.graph_offset.clone();
            for atom in &mut preview.widgets {
                atom.emb_offset = node_offset.clone();
                atom.draw_overlay(frame, &self.rect, anim_counter, asset, context);
            }
        }
    }

    /// Returns the rectangle for the given node either in relative or absolute coordinates
    fn get_node_rect(&self, node_index: usize, relative: bool) -> (isize, isize, usize, usize) {
        let mut x = self.nodes[node_index].user_data.position.0 + self.offset.0;
        let mut y = self.nodes[node_index].user_data.position.1 + self.offset.1;

        if self.nodes[node_index].is_corner_node {
            x = -7;
            y = -7;
        }

        if self.nodes[node_index].is_variable_node {
            x = -7;
            y = 130;

            for i in 0..self.nodes.len() {
                if i < node_index {
                    if self.nodes[i].is_variable_node {
                        y += 36;
                    }
                } else {
                    break;
                }
            }
        }

        if relative == false {
            x += self.rect.0 as isize;
            y += self.rect.1 as isize;
        }

        (x, y, self.nodes[node_index].size.0, self.nodes[node_index].size.1)
    }

    /// Updates a node value from the dialog
    fn update_from_dialog(&mut self, context: &mut ScreenContext) {
        if context.dialog_entry == DialogEntry::NodeName {
            // Node based
            for node_index in 0..self.nodes.len() {
                if self.nodes[node_index].id == context.dialog_node_behavior_id.0 {
                    self.nodes[node_index].text[0] = context.dialog_node_behavior_value.4.clone();
                    self.nodes[node_index].dirty = true;
                    self.dirty = true;

                    context.data.set_behavior_node_name((self.get_curr_behavior_id(context), context.dialog_node_behavior_id.0),context.dialog_node_behavior_value.4.clone(), self.graph_type);
                    break;
                }
            }
            return
        }

        // Atom base
        for node_index in 0..self.nodes.len() {
            for atom_index in 0..self.nodes[node_index].widgets.len() {
                if let Some(id) = &self.nodes[node_index].widgets[atom_index].behavior_id {
                    if id.0 == context.dialog_node_behavior_id.0 && id.1 == context.dialog_node_behavior_id.1 && id.2 == context.dialog_node_behavior_id.2 {
                        self.nodes[node_index].widgets[atom_index].atom_data.data = context.dialog_node_behavior_value.clone();
                        self.nodes[node_index].widgets[atom_index].dirty = true;
                        self.nodes[node_index].dirty = true;
                        self.dirty = true;
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
            for index in 0..self.nodes.len() {
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
                            if atom.curr_index != index {
                                atom.curr_index = index;
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
                    }

                    rc = true;
                }
            }

            // Check the nodes
            for index in 0..self.nodes.len() {

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
                    self.curr_behavior_tree_index = Some(self.behavior_tree_indices[index]);
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

                            // Update the told selection
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
            // let mut clicked_region_id : Option<(usize, isize, isize)> = None;
            if let Some(preview) = &mut self.preview {
                if context.contains_pos_for(pos, preview.rect) {
                    if preview.mouse_down((pos.0 - preview.rect.0, pos.1 - preview.rect.1), asset, context) {

                        // // Region id clicked ?
                        // if let Some(region_id) = preview.clicked_region_id {
                        //     clicked_region_id = Some(region_id);
                        //     preview.clicked_region_id = None;
                        // }

                        if preview.clicked {
                            self.dirty = true;
                            return true;
                        } else {
                            self.preview_drag_start = (pos.0 as isize, pos.1 as isize);
                        }
                    }
                }
            }

            // if let Some(clicked_region_id) = clicked_region_id {
            //     if let Some(active_position_id) = &context.active_position_id {
            //         self.set_node_atom_data(active_position_id.clone(), (clicked_region_id.0 as f64, clicked_region_id.1 as f64, clicked_region_id.2 as f64, 0.0, "".to_string()), context);
            //         context.active_position_id = None;
            //     }
            // }

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

            let mut widget_index = 0;
            if self.graph_type == BehaviorType::Tiles {
                context.switch_editor_state = Some(super::EditorState::TilesDetail);
                widget_index = 1;
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

            if let Some(toolbar) = toolbar {
                toolbar.widgets[widget_index].selected = false;
                toolbar.widgets[widget_index].right_selected = true;
                toolbar.widgets[widget_index].dirty = true;
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
                        context.dialog_node_behavior_id = (self.nodes[index].id, 0, "".to_string());
                        context.dialog_node_behavior_value = (0.0, 0.0, 0.0, 0.0, self.nodes[index].text[0].clone());
                    } else
                    if "Delete".to_string() == menu_activated {
                        if self.nodes.len() > 1 {
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
                            }
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
                        context.dialog_node_behavior_id = (self.nodes[index].id, 0, "".to_string());
                        context.dialog_node_behavior_value = (0.0, 0.0, 0.0, 0.0, self.nodes[index].text[0].clone());
                    } else
                    if "Disconnect".to_string() == menu_activated {
                        // Disconnect node
                        self.disconnect_node(self.nodes[index].id, context);
                    } else
                    if "Delete".to_string() == menu_activated {
                        // Delete node
                        self.delete_node(self.nodes[index].id, context);
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
            if preview.mouse_up(pos, asset, context) {
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
            for atom in &mut preview.widgets {
                if atom.mouse_dragged(pos, asset, context) {
                    return true;
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

        false
    }

    fn mouse_wheel(&mut self, delta: (isize, isize), asset: &mut Asset, context: &mut ScreenContext, _options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {
        if let Some(preview) = &mut self.preview {
            if context.contains_pos_for(self.mouse_hover_pos, preview.rect) {
                preview.mouse_wheel(delta, asset, context);
                self.dirty = true;
                return true;
            }
        }
        self.offset.0 -= delta.0 / 20;
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
    fn set_behavior_id(&mut self, id: usize, context: &mut ScreenContext) {

        self.nodes = vec![];
        self.behavior_tree_indices = vec![];
        self.curr_behavior_tree_index = None;

        self.behavior_id = id;

        if let Some(behavior) = context.data.get_behavior(id, self.graph_type) {
            let sorted_keys = behavior.data.nodes.keys().sorted();

            for i in sorted_keys {
                let mut node_widget = NodeWidget::new_from_behavior_data(&behavior.data,  &behavior.data.nodes[i]);
                self.init_node_widget(&behavior.data, &behavior.data.nodes[i], &mut node_widget, context);
                self.nodes.push(node_widget);
            }
        }
        self.dirty = true;
        self.check_node_visibility(context);
        context.jump_to_position = context.data.get_behavior_default_position(id);
    }

    /// Adds a node of the type identified by its name
    fn add_node_of_name(&mut self, name: String, position: (isize, isize), context: &mut ScreenContext) {

        let mut node_widget : Option<NodeWidget> =  None;
        let mut id : usize = 0;

        // Create the node
        if let Some(behavior) = context.data.get_mut_behavior(self.get_curr_behavior_id(context), self.graph_type) {

            let node_type = match name.as_str() {
                "Expression" => BehaviorNodeType::Expression,
                "Number" => BehaviorNodeType::VariableNumber,
                "Position" => BehaviorNodeType::VariablePosition,
                "Message" if self.graph_type != BehaviorType::Regions => BehaviorNodeType::Message,
                "Pathfinder" => BehaviorNodeType::Pathfinder,
                "Script" => BehaviorNodeType::Script,
                "Lookout" => BehaviorNodeType::Lookout,
                "Close In" => BehaviorNodeType::CloseIn,
                "Call System" => BehaviorNodeType::CallSystem,
                "Call Behavior" => BehaviorNodeType::CallBehavior,
                "Lock Tree" => BehaviorNodeType::LockTree,
                "Unlock" => BehaviorNodeType::UnlockTree,
                "Sequence" => BehaviorNodeType::Sequence,
                "Set State" => BehaviorNodeType::SetState,
                "Linear" => BehaviorNodeType::Linear,
                "Move" => BehaviorNodeType::Move,

                "Always" => BehaviorNodeType::Always,
                "Enter Area" => BehaviorNodeType::EnterArea,
                "Leave Area" => BehaviorNodeType::LeaveArea,
                "Inside Area" => BehaviorNodeType::InsideArea,
                "Spawn" => BehaviorNodeType::Spawn,
                "Displace Tiles" => BehaviorNodeType::DisplaceTiles,

                "Screen" => BehaviorNodeType::Screen,
                "Widget" => BehaviorNodeType::Widget,
                "Settings" => BehaviorNodeType::Settings,

                "Teleport" if self.graph_type == BehaviorType::Regions => BehaviorNodeType::TeleportArea,
                "Message" if self.graph_type == BehaviorType::Regions => BehaviorNodeType::MessageArea,
                "Audio" if self.graph_type == BehaviorType::Regions => BehaviorNodeType::AudioArea,
                "Light" if self.graph_type == BehaviorType::Regions => BehaviorNodeType::LightArea,

                _ => BehaviorNodeType::BehaviorTree
            };

            id = behavior.add_node(node_type, name.clone());
            if let Some(node) = behavior.data.nodes.get_mut(&id) {
                node.position = position;
            }

            let node = NodeWidget::new_from_behavior_data(&behavior.data, &behavior.data.nodes.get(&id).unwrap());
             node_widget = Some(node);

            behavior.save_data();
        }

        // Add the atom widgets
        if let Some(mut node) = node_widget {
            if self.graph_type == BehaviorType::Regions {
                let region = context.data.regions.get(&context.data.regions_ids[context.curr_region_index]).unwrap();
                self.init_node_widget(&region.behaviors[context.curr_region_area_index].data, &region.behaviors[context.curr_region_area_index].data.nodes.get(&id).unwrap(), &mut node, context);
            } else
            if self.graph_type == BehaviorType::Behaviors {
                let behavior = context.data.behaviors.get(&context.data.behaviors_ids[context.curr_behavior_index]).unwrap();
                self.init_node_widget(&behavior.data, &behavior.data.nodes.get(&id).unwrap(), &mut node, context);
            } else
            if self.graph_type == BehaviorType::Systems {
                let behavior = context.data.systems.get(&context.data.systems_ids[context.curr_systems_index]).unwrap();
                self.init_node_widget(&behavior.data, &behavior.data.nodes.get(&id).unwrap(), &mut node, context);
            } else
            if self.graph_type == BehaviorType::Items {
                let behavior = context.data.items.get(&context.data.items_ids[context.curr_items_index]).unwrap();
                self.init_node_widget(&behavior.data, &behavior.data.nodes.get(&id).unwrap(), &mut node, context);
            } else
            if self.graph_type == BehaviorType::GameLogic {
                if let Some(behavior) = context.data.get_behavior(0, self.graph_type) {
                    self.init_node_widget(&behavior.data, &behavior.data.nodes.get(&id).unwrap(), &mut node, context);
                }
            }
            self.nodes.push(node);
        }

        self.check_node_visibility(context);
        self.dirty = true;
    }

    /// Inits the node widget (atom widgets, id)
    fn init_node_widget(&mut self, behavior_data: &GameBehaviorData, node_data: &BehaviorNode, node_widget: &mut NodeWidget, context: &ScreenContext) {

        if node_data.behavior_type == BehaviorNodeType::BehaviorType {

            if self.graph_type == BehaviorType::Behaviors {
                node_widget.is_corner_node = true;

                let mut atom1 = AtomWidget::new(vec!["Hero".to_string(), "Neutral".to_string(), "Monster".to_string()], AtomWidgetType::NodeMenuButton,AtomData::new_as_int("type".to_string(), 0));
                atom1.atom_data.text = "Type".to_string();
                let id = (behavior_data.id, node_data.id, "type".to_string());
                atom1.behavior_id = Some(id.clone());
                atom1.curr_index = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type).0 as usize;
                node_widget.widgets.push(atom1);
                node_widget.color = context.color_black.clone();

                // Position
                let mut position_atom = AtomWidget::new(vec![], AtomWidgetType::NodePositionButton,
                AtomData::new_as_int("position".to_string(), 0));
                position_atom.atom_data.text = "position".to_string();
                let id = (behavior_data.id, node_data.id, "position".to_string());
                position_atom.behavior_id = Some(id.clone());
                position_atom.atom_data.data = context.data.get_behavior_id_value(id, (-1.0,0.0,0.0,0.0, "".to_string()), self.graph_type);
                node_widget.widgets.push(position_atom);

                // Default Character Tile
                let mut tile_atom = AtomWidget::new(vec![], AtomWidgetType::NodeCharTileButton,
                    AtomData::new_as_int("tile".to_string(), 0));
                tile_atom.atom_data.text = "tile".to_string();
                let id = (behavior_data.id, node_data.id, "tile".to_string());
                tile_atom.behavior_id = Some(id.clone());
                tile_atom.atom_data.data = context.data.get_behavior_id_value(id, (-1.0,0.0,0.0,0.0, "".to_string()), self.graph_type);
                node_widget.widgets.push(tile_atom);
            } else
            if self.graph_type == BehaviorType::GameLogic {
                node_widget.is_corner_node = true;

                // Name of the startup tree
                let mut startup_atom = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
                AtomData::new_as_int("startup".to_string(), 0));
                startup_atom.atom_data.text = "startup".to_string();
                let id = (behavior_data.id, node_data.id, "startup".to_string());
                startup_atom.behavior_id = Some(id.clone());
                startup_atom.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "Game".to_string()), self.graph_type);
                node_widget.widgets.push(startup_atom);
            }
            return;
        }

        // Node menu

        let mut menu_text : Vec<String> = vec!["Rename".to_string()];
        if node_data.behavior_type != BehaviorNodeType::VariableNumber {
            menu_text.push( "Disconnect".to_string());
        }
        menu_text.push( "Delete".to_string());

        let mut node_menu_atom = AtomWidget::new(menu_text, AtomWidgetType::NodeMenu,
        AtomData::new_as_int("menu".to_string(), 0));
        node_menu_atom.atom_data.text = "menu".to_string();
        let id = (behavior_data.id, node_data.id, "menu".to_string());
        node_menu_atom.behavior_id = Some(id.clone());
        node_widget.menu = Some(node_menu_atom);

        // Behavior

        if node_data.behavior_type == BehaviorNodeType::BehaviorTree {
            let mut atom1 = AtomWidget::new(vec!["Always".to_string(), "On Startup".to_string(), "On Target".to_string()], AtomWidgetType::NodeMenuButton,
            AtomData::new_as_int("execute".to_string(), 0));
            atom1.atom_data.text = "Execute".to_string();
            let id = (behavior_data.id, node_data.id, "execute".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.curr_index = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type).0 as usize;
            node_widget.widgets.push(atom1);
            node_widget.color = context.color_green.clone();

            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
            if self.graph_type != BehaviorType::GameLogic {
                node_widget.node_connector.insert(BehaviorNodeConnector::Bottom1, NodeConnector { rect: (0,0,0,0) } );
                node_widget.node_connector.insert(BehaviorNodeConnector::Bottom2, NodeConnector { rect: (0,0,0,0) } );
                node_widget.node_connector.insert(BehaviorNodeConnector::Bottom3, NodeConnector { rect: (0,0,0,0) } );
                node_widget.node_connector.insert(BehaviorNodeConnector::Bottom4, NodeConnector { rect: (0,0,0,0) } );
            }

            // Add the node to the behavior tree ids
            self.behavior_tree_indices.push(self.nodes.len());
            if self.curr_behavior_tree_index == None {
                self.curr_behavior_tree_index = Some(self.nodes.len());
            }
        } else
        if node_data.behavior_type == BehaviorNodeType::Linear {
            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom1, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom2, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom3, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom4, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::Sequence {
            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom1, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom2, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom3, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom4, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::Expression {
            let mut atom1 = AtomWidget::new(vec!["Expression".to_string()], AtomWidgetType::NodeExpressionButton,
            AtomData::new_as_int("expression".to_string(), 0));
            atom1.atom_data.text = "Expression".to_string();
            let id = (behavior_data.id, node_data.id, "expression".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "false".to_string()), self.graph_type);
            node_widget.widgets.push(atom1);

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::Script {
            let mut atom1 = AtomWidget::new(vec!["Script".to_string()], AtomWidgetType::NodeScriptButton,
            AtomData::new_as_int("script".to_string(), 0));
            atom1.atom_data.text = "Script".to_string();
            let id = (behavior_data.id, node_data.id, "script".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type);
            node_widget.widgets.push(atom1);

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::VariableNumber {

            let mut atom1 = AtomWidget::new(vec!["Value".to_string()], AtomWidgetType::NodeNumberButton,
            AtomData::new_as_int("value".to_string(), 0));
            atom1.atom_data.text = "Value".to_string();
            let id = (behavior_data.id, node_data.id, "value".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type);
            node_widget.widgets.push(atom1);

            node_widget.color = context.color_orange.clone();
            node_widget.is_variable_node = true;
        } else
        if node_data.behavior_type == BehaviorNodeType::Message || node_data.behavior_type == BehaviorNodeType::MessageArea {
            let mut atom1 = AtomWidget::new(vec!["Status".to_string(), "Say".to_string(), "Yell".to_string(), "Private".to_string(), "Debug".to_string()], AtomWidgetType::NodeMenuButton,
            AtomData::new_as_int("type".to_string(), 0));
            atom1.atom_data.text = "Type".to_string();
            let id = (behavior_data.id, node_data.id, "type".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.curr_index = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type).0 as usize;
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec!["Text".to_string()], AtomWidgetType::NodeTextButton,
            AtomData::new_as_int("text".to_string(), 0));
            atom2.atom_data.text = "Text".to_string();
            let id = (behavior_data.id, node_data.id, "text".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "Hello".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::Pathfinder {
            let mut atom1 = AtomWidget::new(vec!["Destination".to_string()], AtomWidgetType::NodePositionButton,
            AtomData::new_as_int("destination".to_string(), 0));
            atom1.atom_data.text = "Destination".to_string();
            let id = (behavior_data.id, node_data.id, "destination".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.data = context.data.get_behavior_id_value(id, (-1.0,0.0,0.0,0.0, "".to_string()), self.graph_type);
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec!["Speed".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new_as_int("speed".to_string(), 0));
            atom2.atom_data.text = "Speed".to_string();
            let id = (behavior_data.id, node_data.id, "speed".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "8".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::Lookout {
            let mut atom1 = AtomWidget::new(vec!["Expression".to_string()], AtomWidgetType::NodeExpressionButton,
            AtomData::new_as_int("expression".to_string(), 0));
            atom1.atom_data.text = "Expression".to_string();
            let id = (behavior_data.id, node_data.id, "expression".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type);
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec!["Max Distance".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new_as_int("max_distance".to_string(), 0));
            atom2.atom_data.text = "Max Distance".to_string();
            let id = (behavior_data.id, node_data.id, "max_distance".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "7".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::CloseIn {
            let mut atom1 = AtomWidget::new(vec!["To Distance".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new_as_int("to_distance".to_string(), 0));
            atom1.atom_data.text = "To Distance".to_string();
            let id = (behavior_data.id, node_data.id, "to_distance".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "1".to_string()), self.graph_type);
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec!["Speed".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new_as_int("speed".to_string(), 0));
            atom2.atom_data.text = "Speed".to_string();
            let id = (behavior_data.id, node_data.id, "speed".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "8".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::CallSystem {

            let mut atom1 = AtomWidget::new(context.data.systems_names.clone(), AtomWidgetType::NodeTextButton,
            AtomData::new_as_int("system".to_string(), 0));
            atom1.atom_data.text = "System Name".to_string();
            let id = (behavior_data.id, node_data.id, "system".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type);
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new_as_int("tree".to_string(), 0));
            atom2.atom_data.text = "Tree Name".to_string();
            let id = (behavior_data.id, node_data.id, "tree".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.color = context.color_blue.clone();

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::CallBehavior {
            let mut atom1 = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new_as_int("tree".to_string(), 0));
            atom1.atom_data.text = "Tree Name".to_string();
            let id = (behavior_data.id, node_data.id, "tree".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type);
            node_widget.widgets.push(atom1);

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::LockTree {
            let mut atom1 = AtomWidget::new(vec!["Self".to_string(), "Target".to_string()], AtomWidgetType::NodeMenuButton,
            AtomData::new_as_int("execute_for".to_string(), 0));
            atom1.atom_data.text = "Execute For".to_string();
            let id = (behavior_data.id, node_data.id, "execute_for".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.curr_index = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type).0 as usize;
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new_as_int("tree".to_string(), 0));
            atom2.atom_data.text = "Tree Name".to_string();
            let id = (behavior_data.id, node_data.id, "tree".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type);
            node_widget.widgets.push(atom2);

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::UnlockTree {

            let mut atom1 = AtomWidget::new(vec!["Self".to_string(), "Target".to_string()], AtomWidgetType::NodeMenuButton,
            AtomData::new_as_int("execute_for".to_string(), 0));
            atom1.atom_data.text = "Execute For".to_string();
            let id = (behavior_data.id, node_data.id, "execute_for".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.curr_index = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type).0 as usize;
            node_widget.widgets.push(atom1);

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::SetState {
            let mut atom1 = AtomWidget::new(vec!["Self".to_string(), "Target".to_string()], AtomWidgetType::NodeMenuButton,
            AtomData::new_as_int("for".to_string(), 0));
            atom1.atom_data.text = "For".to_string();
            let id = (behavior_data.id, node_data.id, "for".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.curr_index = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type).0 as usize;
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec!["Normal".to_string(), "Hidden".to_string(), "Killed".to_string(), "Purged".to_string()], AtomWidgetType::NodeMenuButton,
            AtomData::new_as_int("state".to_string(), 0));
            atom2.atom_data.text = "State".to_string();
            let id = (behavior_data.id, node_data.id, "state".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.curr_index = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type).0 as usize;
            node_widget.widgets.push(atom2);

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::Move {
            let mut atom1 = AtomWidget::new(vec!["Speed".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new_as_int("speed".to_string(), 0));
            atom1.atom_data.text = "Speed".to_string();
            let id = (behavior_data.id, node_data.id, "speed".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "8".to_string()), self.graph_type);
            node_widget.widgets.push(atom1);

            node_widget.color = context.color_gray.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        }

        // Area
        if node_data.behavior_type == BehaviorNodeType::InsideArea {
            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::Always {
            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::EnterArea {
            let mut atom1 = AtomWidget::new(vec!["Everyone".to_string(), "First Only".to_string()], AtomWidgetType::NodeMenuButton,
            AtomData::new_as_int("character".to_string(), 0));
            atom1.atom_data.text = "Character".to_string();
            let id = (behavior_data.id, node_data.id, "character".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.curr_index = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type).0 as usize;
            node_widget.widgets.push(atom1);

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::LeaveArea {
            let mut atom1 = AtomWidget::new(vec!["Everyone".to_string(), "Last Only".to_string()], AtomWidgetType::NodeMenuButton,
            AtomData::new_as_int("character".to_string(), 0));
            atom1.atom_data.text = "Character".to_string();
            let id = (behavior_data.id, node_data.id, "character".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.curr_index = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type).0 as usize;
            node_widget.widgets.push(atom1);

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::DisplaceTiles {

            let mut tile_filter_atom = AtomWidget::new(vec!["Filter".to_string()], AtomWidgetType::NodeEnvTileButton,
                    AtomData::new_as_int("filter".to_string(), 0));
            tile_filter_atom.atom_data.text = "Filter".to_string();
            let id = (behavior_data.id, node_data.id, "filter".to_string());
            tile_filter_atom.behavior_id = Some(id.clone());
            tile_filter_atom.atom_data.data = context.data.get_behavior_id_value(id, (-1.0,0.0,0.0,0.0, "".to_string()), self.graph_type);
            node_widget.widgets.push(tile_filter_atom);

            let mut tile_atom = AtomWidget::new(vec!["Tile".to_string()], AtomWidgetType::NodeEnvTileButton,
                    AtomData::new_as_int("tile".to_string(), 0));
            tile_atom.atom_data.text = "Tile".to_string();
            let id = (behavior_data.id, node_data.id, "tile".to_string());
            tile_atom.behavior_id = Some(id.clone());
            tile_atom.atom_data.data = context.data.get_behavior_id_value(id, (-1.0,0.0,0.0,0.0, "".to_string()), self.graph_type);
            node_widget.widgets.push(tile_atom);

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::TeleportArea {
            // Position
            let mut position_atom = AtomWidget::new(vec![], AtomWidgetType::NodePositionButton,
            AtomData::new_as_int("position".to_string(), 0));
            position_atom.atom_data.text = "Position".to_string();
            let id = (behavior_data.id, node_data.id, "position".to_string());
            position_atom.behavior_id = Some(id.clone());
            position_atom.atom_data.data = context.data.get_behavior_id_value(id, (-1.0,0.0,0.0,0.0, "".to_string()), self.graph_type);
            node_widget.widgets.push(position_atom);

            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::AudioArea {
            let mut atom1 = AtomWidget::new(vec![], AtomWidgetType::NodeTextButton,
            AtomData::new_as_int("audio".to_string(), 0));
            atom1.atom_data.text = "Audio".to_string();
            let id = (behavior_data.id, node_data.id, "audio".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type);
            node_widget.widgets.push(atom1);

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::LightArea {
            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
        }

        //

        if node_data.behavior_type == BehaviorNodeType::Screen {
            let mut atom1 = AtomWidget::new(vec!["Script".to_string()], AtomWidgetType::NodeScreenButton,
            AtomData::new_as_int("script".to_string(), 0));
            atom1.atom_data.text = "Script".to_string();
            let id = (behavior_data.id, node_data.id, "script".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()), self.graph_type);
            node_widget.widgets.push(atom1);

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        }

    }

    /// Converts the index of a node widget to a node id
    fn widget_index_to_node_id(&self, index: usize) -> usize {
        self.nodes[index].id
    }

    /// Converts the id of a node to a widget index
    fn node_id_to_widget_index(&self, id: usize) -> usize {
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
    fn disconnect_node(&mut self, id: usize, context: &mut ScreenContext) {

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
    fn delete_node(&mut self, id: usize, context: &mut ScreenContext) {
        self.disconnect_node(id, context);

        // Remove node widget
        for index in 0..self.nodes.len() {
            if self.nodes[index].id == id {
                self.nodes.remove(index);
                break
            }
        }

        // Remove node data
        if let Some(behavior) = context.data.get_mut_behavior(self.get_curr_behavior_id(context), self.graph_type) {
            behavior.data.nodes.remove(&id);
            behavior.save_data();
        }

        // If this is an BehaviorTree remove it from the current indices
        if let Some(index) = self.behavior_tree_indices.iter().position(|&x| x == id) {
            self.behavior_tree_indices.remove(index);
        }
    }

    /// Sets the widget and behavior data for the given atom id
    fn set_node_atom_data(&mut self, node_atom_id: (usize, usize, String), data: (f64, f64, f64, f64, String), context: &mut ScreenContext) {
        for index in 0..self.nodes.len() {
            if self.nodes[index].id == node_atom_id.1 {
                for atom_index in 0..self.nodes[index].widgets.len() {
                    if self.nodes[index].widgets[atom_index].atom_data.id == node_atom_id.2 {
                        self.nodes[index].widgets[atom_index].atom_data.data = data.clone();
                        self.nodes[index].widgets[atom_index].dirty = true;
                        self.nodes[index].dirty = true;
                        self.dirty = true;

                        context.data.set_behavior_id_value(node_atom_id.clone(), data.clone(), self.graph_type);

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

        if let Some(tree_index) = self.curr_behavior_tree_index {

            let tree_id = self.widget_index_to_node_id(tree_index);
            self.visible_node_ids.push(tree_id);
            self.mark_connections_visible(tree_id, context);

            // Find unconnected nodes and mark them visible
            for index in 0..self.nodes.len() {

                let mut connected = false;
                if let Some(behavior) = context.data.get_behavior(self.get_curr_behavior_id(context), self.graph_type) {

                    // Skip behavior tree nodes
                    if let Some(node_data) = behavior.data.nodes.get(&self.widget_index_to_node_id(index)) {
                        if node_data.behavior_type == BehaviorNodeType::BehaviorTree {
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
        }
    }

    /// Marks all connected nodes as visible
    fn mark_connections_visible(&mut self, id: usize, context: &ScreenContext) {
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
    fn belongs_to_standalone_branch(&mut self, id: usize, context: &ScreenContext) -> bool {
        if let Some(behavior) = context.data.get_behavior(self.get_curr_behavior_id(context), self.graph_type) {

            for (source_node_id , _source_connector, dest_node_id, _dest_connector) in &behavior.data.connections {
                if *dest_node_id == id {
                    return self.belongs_to_standalone_branch(*source_node_id, context);
                }
            }

            if let Some(node_data) = behavior.data.nodes.get(&id) {
                if node_data.behavior_type != BehaviorNodeType::BehaviorTree {
                    return true;
                }
            }
        }

        false
    }

    /// Collects the children indices of the given node id so that they can all be dragged at once
    fn collect_drag_children_indices(&mut self, id: usize, context: &ScreenContext) {
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
    fn get_curr_behavior_id(&self, context: &ScreenContext) -> usize {
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
            return context.data.items_ids[context.curr_items_index];
        }
        0
    }

    /// Returns the current node id for the given graph type
    fn get_curr_node_id(&self, context: &ScreenContext) -> Option<usize> {

        if let Some(behavior) = context.data.get_behavior(self.behavior_id, self.graph_type) {
            return behavior.data.curr_node_id;
        }
        None
    }
}