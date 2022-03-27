use crate::editor::node::{NodeConnector, NodeWidget};
use crate::atom:: { AtomData, AtomWidget, AtomWidgetType };
use crate::editor::node_preview::NodePreviewWidget;
use crate::editor::dialog::{ DialogState, DialogEntry };

use zeno::{Mask, Stroke};

use server::gamedata::behavior::{GameBehaviorData, BehaviorNodeType, BehaviorNode, BehaviorNodeConnector};

use server::{asset::Asset };
use crate::editor::ScreenContext;

#[derive(PartialEq)]
pub enum GraphMode {
    Overview,
    Detail
}

#[derive(PartialEq)]
pub enum GraphType {
    Tiles,
    Areas,
    Behavior,
}

pub struct NodeGraph {
    pub rect                    : (usize, usize, usize, usize),
    dirty                       : bool,
    buffer                      : Vec<u8>,
    graph_mode                  : GraphMode,
    graph_type                  : GraphType,
    nodes                       : Vec<NodeWidget>,

    pub offset                  : (isize, isize),

    drag_index                  : Option<usize>,
    drag_offset                 : (isize, isize),
    drag_node_pos               : (isize, isize),

    // For connecting nodes
    source_conn                 : Option<(BehaviorNodeConnector,usize)>,
    dest_conn                   : Option<(BehaviorNodeConnector,usize)>,

    mouse_pos                   : (usize, usize),

    pub clicked                 : bool,

    pub preview                 : Option<NodePreviewWidget>,
    preview_drag_start          : (isize, isize),

    mouse_hover_pos             : (usize, usize)
}

impl NodeGraph {

    pub fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), _asset: &Asset, _context: &ScreenContext, graph_type: GraphType, nodes: Vec<NodeWidget>) -> Self {
        Self {
            rect,
            dirty               : true,
            buffer              : vec![0;rect.2 * rect.3 * 4],
            graph_mode          : GraphMode::Overview,
            graph_type,
            nodes,
            offset              : (0, 0),
            drag_index          : None,
            drag_offset         : (0, 0),
            drag_node_pos       : (0, 0),

            mouse_pos           : (0,0),
            clicked             : false,

            source_conn         : None,
            dest_conn           : None,

            preview             : None,
            preview_drag_start  : (0,0),

            mouse_hover_pos     : (0, 0)
        }
    }

    pub fn update(&mut self, context: &mut ScreenContext) {
        if context.is_running {
            context.data.tick();
            self.dirty = true;
            if let Some(preview) = &mut self.preview {
                preview.dirty = true;
            }
        }
    }

    pub fn set_mode(&mut self, mode: GraphMode, context: &ScreenContext) {
        if mode == GraphMode::Detail && self.preview.is_none() {
            self.preview = Some(NodePreviewWidget::new(context));
        }
        self.graph_mode = mode;
    }

    pub fn set_mode_and_rect(&mut self, mode: GraphMode, rect: (usize, usize, usize, usize), context: &ScreenContext) {
        if mode == GraphMode::Detail && self.preview.is_none() {
            self.preview = Some(NodePreviewWidget::new(context));
        }
        self.graph_mode = mode;
        self.rect = rect;
        self.resize(rect.2, rect.3, context)
    }

    pub fn _set_mode_and_nodes(&mut self, mode: GraphMode, nodes: Vec<NodeWidget>, _context: &ScreenContext) {
        self.graph_mode = mode;
        self.nodes = nodes;
        self.mark_all_dirty();
    }

    pub fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.buffer.resize(width * height * 4, 0);
        self.dirty = true;
        self.rect.2 = width;
        self.rect.3 = height;
    }

    pub fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {
        let safe_rect = (0_usize, 0_usize, self.rect.2, self.rect.3);

        if self.dirty {
            context.draw2d.draw_square_pattern(&mut self.buffer[..], &safe_rect, safe_rect.2, &[44, 44, 46, 255], &[56, 56, 56, 255], 40);

            if self.graph_mode == GraphMode::Overview {
                for index in 0..self.nodes.len() {

                    let mut selected = false;

                    if self.graph_type == GraphType::Tiles {
                        if index == context.curr_tileset_index {
                            selected = true;
                        }
                    } else
                    if self.graph_type == GraphType::Areas {
                        if index == context.curr_area_index {
                            selected = true;
                        }
                    } else
                    if self.graph_type == GraphType::Behavior {
                        if index == context.curr_behavior_index {
                            selected = true;
                        }
                    }

                    if self.nodes[index].dirty {
                        let mut preview_buffer = vec![0; 100 * 100 * 4];
                        if self.graph_type == GraphType::Tiles {
                            // For tile maps draw the default_tile
                            if let Some(map)= asset.tileset.maps.get_mut(&index) {
                                if let Some(default_tile) = map.settings.default_tile {
                                    context.draw2d.draw_animated_tile(&mut preview_buffer[..], &(0, 0), map, 100, &default_tile, 0, 100);
                                }
                            }
                        } else
                        if self.graph_type == GraphType::Areas {
                            // For Areas draw the center of the map
                            if let Some(area)= context.data.areas.get_mut(&index) {
                                let offset = area.get_center_offset_for_visible_size((10, 10));
                                context.draw2d.draw_area(&mut preview_buffer[..], area, &(0, 0, 100, 100), &offset, 100, 10, anim_counter, asset);
                            }
                        }

                        self.nodes[index].draw_overview(frame, anim_counter, asset, context, selected, &preview_buffer);
                    }

                    let rect= self.get_node_rect(index, true);
                    context.draw2d.blend_slice_safe(&mut self.buffer[..], &self.nodes[index].buffer[..], &rect, safe_rect.2, &safe_rect);
                }
            } else {
                // Detail View

                let mut corner_index = 0;

                // Draw nodes
                for index in 0..self.nodes.len() {

                    // Check for corner node
                    if self.nodes[index].is_corner_node {
                        corner_index = index;
                        continue;
                    }

                    if self.nodes[index].dirty {

                        let mut selected = false;

                        if self.nodes[index].id == context.curr_behavior_node_id {
                            selected = true;
                        }

                        self.nodes[index].draw(frame, anim_counter, asset, context, selected);
                    }

                    let rect= self.get_node_rect(index, true);
                    self.nodes[index].graph_offset = (rect.0, rect.1);
                    context.draw2d.blend_slice_safe(&mut self.buffer[..], &self.nodes[index].buffer[..], &rect, safe_rect.2, &safe_rect);
                }

                // Corner node
                if self.nodes[corner_index].dirty {
                    self.nodes[corner_index].draw(frame, anim_counter, asset, context, false);
                }

                let rect= self.get_node_rect(corner_index, true);
                self.nodes[corner_index].graph_offset = (rect.0, rect.1);
                self.nodes[corner_index].graph_offset = (rect.0, rect.1);
                context.draw2d.blend_slice_safe(&mut self.buffer[..], &self.nodes[corner_index].buffer[..], &rect, safe_rect.2, &safe_rect);

                // --

                let mut mask : Vec<u8> = vec![0; safe_rect.2 * safe_rect.3];
                let mut path : String = "".to_string();

                // Draw connections
                if let Some(behavior) = context.data.behaviors.get(&context.curr_behavior_index) {

                    for (source_node_id , source_connector, dest_node_id, dest_connector) in &behavior.data.connections {

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

                        if *source_connector == BehaviorNodeConnector::Right {
                            start_x = source_rect.0 + s_connector.rect.0 as isize + s_connector.rect.2 as isize;
                            start_y = source_rect.1 + s_connector.rect.1 as isize + s_connector.rect.3 as isize / 2;
                            control_start_x = start_x + 50;
                            control_start_y = start_y;
                        } else {
                            start_x = source_rect.0 + s_connector.rect.0 as isize + s_connector.rect.2 as isize / 2;
                            start_y = source_rect.1 + s_connector.rect.1 as isize + s_connector.rect.3 as isize;
                            control_start_x = start_x;
                            control_start_y = start_y + 50;
                        }

                        if *dest_connector == BehaviorNodeConnector::Left {
                            end_x = dest_rect.0 + d_connector.rect.0 as isize + d_connector.rect.2 as isize / 2;
                            end_y = dest_rect.1 + d_connector.rect.1 as isize + d_connector.rect.3 as isize / 2;
                            control_end_x = end_x - 50;
                            control_end_y = end_y;
                        } else {
                            end_x = dest_rect.0 + d_connector.rect.0 as isize + d_connector.rect.2 as isize / 2;
                            end_y = dest_rect.1 + d_connector.rect.1 as isize + d_connector.rect.3 as isize / 2;
                            control_end_x = end_x ;
                            control_end_y = end_y - 50;
                        }

                        //context.draw2d.draw_line_safe(&mut self.buffer[..], &(start_x, start_y), &(end_x, end_y), &safe_rect, safe_rect.2, &context.node_connector_color);

                        //path += format!("M {},{} L {},{}", start_x, start_y, end_x, end_y).as_str();

                        path += format!("M {},{} C {},{} {},{} {},{}", start_x, start_y, control_start_x, control_start_y, control_end_x, control_end_y, end_x, end_y).as_str();
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

                    //context.draw2d.draw_line_safe(&mut self.buffer[..], &(start_x, start_y), &(end_x, end_y), &safe_rect, safe_rect.2, &context.node_connector_color);
                    path += format!("M {},{} L {},{}", start_x, start_y, end_x, end_y).as_str();
                }

                // Draw the path if not empty
                if !path.is_empty() {
                    Mask::new(path.as_str())
                    .size(safe_rect.2 as u32, safe_rect.3 as u32)
                    .style(
                       Stroke::new(2.0)
                    )
                    .render_into(&mut mask, None);

                    context.draw2d.blend_mask(&mut self.buffer[..], &safe_rect, safe_rect.2, &mask[..], &(safe_rect.2, safe_rect.3), &context.node_connector_color);
                }

                // Render the preview widget
                if let Some(preview) = &mut self.preview {
                    preview.draw(frame, anim_counter, asset, context);
                    preview.rect = (self.rect.0 + self.rect.2 - preview.size.0, self.rect.1, preview.size.0, preview.size.1);
                    preview.graph_offset = (preview.rect.0 as isize, preview.rect.1 as isize);
                    context.draw2d.blend_slice(&mut self.buffer[..], &mut preview.buffer[..], &(self.rect.2 - preview.size.0, 0, preview.size.0, preview.size.1), safe_rect.2);
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

        if relative == false {
            x += self.rect.0 as isize;
            y += self.rect.1 as isize;
        }

        (x, y, self.nodes[node_index].size.0, self.nodes[node_index].size.1)
    }

    /// Updates a node value from the dialog
    pub fn update_from_dialog(&mut self, context: &mut ScreenContext) {
        if context.dialog_entry == DialogEntry::NodeName {
            // Node based
            for node_index in 0..self.nodes.len() {
                if self.nodes[node_index].id == context.dialog_node_behavior_id.0 {
                    self.nodes[node_index].text[0] = context.dialog_node_behavior_value.4.clone();
                    self.nodes[node_index].dirty = true;
                    self.dirty = true;

                    context.data.set_behavior_node_name((context.curr_behavior_index, context.dialog_node_behavior_id.0),context.dialog_node_behavior_value.4.clone());
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

    pub fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {

        if self.graph_mode == GraphMode::Overview {
            for index in 0..self.nodes.len() {
                let rect= self.get_node_rect(index, false);

                if context.contains_pos_for_isize(pos, rect) {

                    self.drag_index = Some(index);
                    self.drag_offset = (pos.0 as isize, pos.1 as isize);
                    self.drag_node_pos= (self.nodes[index].user_data.position.0 as isize, self.nodes[index].user_data.position.1 as isize);

                    if self.graph_type == GraphType::Tiles {
                        if context.curr_tileset_index != index {

                            self.nodes[context.curr_tileset_index].dirty = true;
                            context.curr_tileset_index = index;
                            self.nodes[index].dirty = true;
                            self.dirty = true;
                            self.clicked = true;
                        }
                    }
                    if self.graph_type == GraphType::Areas {
                        if context.curr_area_index != index {

                            self.nodes[context.curr_area_index].dirty = true;
                            context.curr_area_index = index;
                            self.nodes[index].dirty = true;
                            self.dirty = true;
                            self.clicked = true;
                        }
                    }
                    if self.graph_type == GraphType::Behavior {
                        if context.curr_behavior_index != index {

                            self.nodes[context.curr_behavior_index].dirty = true;
                            context.curr_behavior_index = index;
                            self.nodes[index].dirty = true;
                            self.dirty = true;
                            self.clicked = true;
                        }
                    }

                    return true;
                }
            }
        } else
        if self.graph_mode == GraphMode::Detail {

            for index in 0..self.nodes.len() {

                if self.nodes[index].is_corner_node {
                    continue;
                }

                let rect= self.get_node_rect(index, false);

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

                    self.drag_index = Some(index);
                    self.drag_offset = (pos.0 as isize, pos.1 as isize);
                    self.drag_node_pos= (self.nodes[index].user_data.position.0 as isize, self.nodes[index].user_data.position.1 as isize);

                    if self.graph_type == GraphType::Behavior {
                        if context.curr_behavior_node_id != self.nodes[index].id {

                            let sel_index = self.node_id_to_widget_index(context.curr_behavior_node_id);

                            self.nodes[sel_index].dirty = true;
                            context.curr_behavior_node_id = self.nodes[index].id;
                            self.nodes[index].dirty = true;
                            self.dirty = true;
                            self.clicked = true;
                            return true;
                        }
                    }
                }
            }

            // Check Preview

            let mut clicked_area_id : Option<(usize, isize, isize)> = None;
            if let Some(preview) = &mut self.preview {
                if context.contains_pos_for(pos, preview.rect) {
                    if preview.mouse_down((pos.0 - preview.rect.0, pos.1 - preview.rect.1), asset, context) {

                        // Area id clicked ?
                        if let Some(area_id) = preview.clicked_area_id {
                            clicked_area_id = Some(area_id);
                            preview.clicked_area_id = None;
                        }

                        if preview.clicked {
                            self.dirty = true;
                            return true;
                        } else {
                            self.preview_drag_start = (pos.0 as isize, pos.1 as isize);
                        }
                    }
                }
            }

            if let Some(clicked_area_id) = clicked_area_id {
                if let Some(active_position_id) = &context.active_position_id {
                    self.set_node_atom_data(active_position_id.clone(), (clicked_area_id.0 as f64, clicked_area_id.1 as f64, clicked_area_id.2 as f64, 0.0, "".to_string()), context);
                    context.active_position_id = None;
                }
            }
        }

        // Check the atom widgets
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

        false
    }

    pub fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        if self.drag_index != None {
            self.drag_index = None;
            context.target_fps = context.default_fps;

            // Save the new node position
            if self.graph_type == GraphType::Behavior && self.graph_mode == GraphMode::Detail {
                if let Some(behavior) = context.data.behaviors.get_mut(&context.curr_behavior_index) {

                    for node_widget in &self.nodes {
                        if node_widget.id == context.curr_behavior_node_id {
                            let position = node_widget.user_data.position.clone();
                            if let Some(behavior_node) = behavior.data.nodes.get_mut(&context.curr_behavior_node_id) {
                                behavior_node.position = position;
                                behavior.save_data();
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Node connection
        if let Some(source_conn) = &self.source_conn {
            if let Some(dest_conn) = &self.dest_conn {

                if let Some(behavior) = context.data.behaviors.get_mut(&context.curr_behavior_index) {

                    // Add the connection in the order of source connector -> dest connector
                    if self.connector_is_source(dest_conn.0) {
                        behavior.data.connections.push((self.widget_index_to_node_id(dest_conn.1), dest_conn.0, self.widget_index_to_node_id(source_conn.1), source_conn.0));
                        self.nodes[source_conn.1].dirty = true;
                    } else {
                        behavior.data.connections.push((self.widget_index_to_node_id(source_conn.1), source_conn.0, self.widget_index_to_node_id(dest_conn.1), dest_conn.0));
                        self.nodes[dest_conn.1].dirty = true;
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

            let mut menu_activated : Option<usize> = None;
            if let Some(menu) = &mut self.nodes[index].menu {
                if menu.mouse_up(local, asset, context) {
                    if menu.new_selection.is_some() {
                        menu_activated = Some(menu.curr_index);
                        menu.dirty = true;
                        self.dirty = true;
                    }
                }
            }

            // If a menu was activated, mark the node as dirty
            if let Some(menu_activated) = menu_activated {
                self.nodes[index].dirty = true;
                match menu_activated {
                    0 => {
                        // Rename node
                        context.dialog_state = DialogState::Opening;
                        context.dialog_height = 0;
                        context.target_fps = 60;
                        context.dialog_entry = DialogEntry::NodeName;
                        context.dialog_node_behavior_id = (self.nodes[index].id, 0, "".to_string());
                        context.dialog_node_behavior_value = (0.0, 0.0, 0.0, 0.0, self.nodes[index].text[0].clone());
                    },
                    1 => {
                        // Disconnect node
                        self.disconnect_node(self.nodes[index].id, context);
                    },
                    2 => {
                        // Delete node
                        self.delete_node(self.nodes[index].id, context);
                    }
                    _ => {},
                }
                return true;
            }

            if self.nodes[index].mouse_up(local, asset, context) {
                self.dirty = true;
                if let Some(behavior) = context.data.behaviors.get_mut(&context.curr_behavior_index) {
                    behavior.save_data();
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

    pub fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {

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
                if atom.atom_widget_type == AtomWidgetType::MenuButton || atom.atom_widget_type == AtomWidgetType::NodeMenuButton {
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
        if let Some(index) = self.drag_index {
            let dx = pos.0 as isize - self.drag_offset.0;
            let dy = pos.1 as isize - self.drag_offset.1;

            self.nodes[index].user_data.position.0 = self.drag_node_pos.0 + dx;
            self.nodes[index].user_data.position.1 = self.drag_node_pos.1 + dy;
            self.dirty = true;

            context.target_fps = 60;

            return true;
        }

        self.dest_conn = None;
        // Dragging a connection, check for dest connection
        if let Some(source) = self.source_conn {
            for index in 0..self.nodes.len() {
                let rect= self.get_node_rect(index, false);

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

    pub fn mouse_wheel(&mut self, delta: (isize, isize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
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

    pub fn mouse_hover(&mut self, pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        self.mouse_hover_pos = pos;
        false
    }

    /// Marks the two nodes as dirty
    pub fn changed_selection(&mut self, old_selection: usize, new_selection: usize) {
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
    pub fn mark_all_dirty(&mut self) {
        if self.graph_mode == GraphMode::Overview {
            for index in 0..self.nodes.len() {
                self.nodes[index].dirty = true;
            }
        }
        self.dirty = true;
    }

    /// Set the behavior id, this will take the bevhavior node data and create node widgets
    pub fn set_behavior_id(&mut self, _id: usize, context: &ScreenContext) {
        self.nodes = vec![];
        if let Some(behavior) = context.data.behaviors.get(&context.curr_behavior_index) {
            for (_id, node_data) in &behavior.data.nodes {
                let mut node_widget = NodeWidget::new_from_behavior_data(&behavior.data, node_data);
                self.init_node_widget(&behavior.data, node_data, &mut node_widget, context);
                self.nodes.push(node_widget);
            }
        }
    }

    /// Adds a node of the type identified by its name
    pub fn add_node_of_name(&mut self, name: String, position: (isize, isize), context: &mut ScreenContext) {

        let mut node_widget : Option<NodeWidget> =  None;
        let mut id : usize = 0;

        // Create the node
        if let Some(behavior) = context.data.behaviors.get_mut(&context.curr_behavior_index) {

            let node_type = match name.as_str() {
                "Expression" => BehaviorNodeType::Expression,
                "Number" => BehaviorNodeType::VariableNumber,
                "Position" => BehaviorNodeType::VariablePosition,
                "Say" => BehaviorNodeType::Say,
                "Pathfinder" => BehaviorNodeType::Pathfinder,
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
            let behavior = context.data.behaviors.get(&context.curr_behavior_index).unwrap();
            self.init_node_widget(&behavior.data, &behavior.data.nodes.get(&id).unwrap(), &mut node, context);
            self.nodes.push(node);
        }

        self.dirty = true;
    }

    /// Inits the node widget (atom widgets, id)
    pub fn init_node_widget(&mut self, behavior_data: &GameBehaviorData, node_data: &BehaviorNode, node_widget: &mut NodeWidget, context: &ScreenContext) {

        if node_data.behavior_type == BehaviorNodeType::BehaviorType {
            node_widget.is_corner_node = true;

            let mut atom1 = AtomWidget::new(vec!["Character".to_string(), "Area".to_string(), "Module".to_string()], AtomWidgetType::NodeMenuButton,AtomData::new_as_int("type".to_string(), 0));
            atom1.atom_data.text = "Type".to_string();
            let id = (behavior_data.id, node_data.id, "type".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.curr_index = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string())).0 as usize;
            node_widget.widgets.push(atom1);
            node_widget.color = context.color_black.clone();

            self.setup_corner_node_widget(behavior_data, node_data, node_widget, context);
            return;
        }

        // Node menu
        let mut node_menu_atom = AtomWidget::new(vec!["Rename".to_string(), "Disconnect".to_string(), "Delete".to_string()], AtomWidgetType::NodeMenu,
        AtomData::new_as_int("menu".to_string(), 0));
        node_menu_atom.atom_data.text = "menu".to_string();
        let id = (behavior_data.id, node_data.id, "menu".to_string());
        node_menu_atom.behavior_id = Some(id.clone());
        node_widget.menu = Some(node_menu_atom);

        if node_data.behavior_type == BehaviorNodeType::BehaviorTree {
            let mut atom1 = AtomWidget::new(vec!["Always".to_string(), "On Startup".to_string(), "On Demand".to_string()], AtomWidgetType::NodeMenuButton,
            AtomData::new_as_int("execute".to_string(), 0));
            atom1.atom_data.text = "Execute".to_string();
            let id = (behavior_data.id, node_data.id, "execute".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.curr_index = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string())).0 as usize;
            node_widget.widgets.push(atom1);
            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::Expression {

            let mut atom1 = AtomWidget::new(vec!["Expression".to_string()], AtomWidgetType::NodeExpressionButton,
            AtomData::new_as_int("expression".to_string(), 0));
            atom1.atom_data.text = "Expression".to_string();
            let id = (behavior_data.id, node_data.id, "expression".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "false".to_string()));
            node_widget.widgets.push(atom1);

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Left, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        } else
        if node_data.behavior_type == BehaviorNodeType::VariableNumber {

            let mut atom1 = AtomWidget::new(vec!["Value".to_string()], AtomWidgetType::NodeIntButton,
            AtomData::new_as_int("value".to_string(), 0));
            atom1.atom_data.text = "Value".to_string();
            let id = (behavior_data.id, node_data.id, "value".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "".to_string()));
            node_widget.widgets.push(atom1);

            node_widget.color = context.color_orange.clone();
        } else
        if node_data.behavior_type == BehaviorNodeType::Say {
            let mut atom1 = AtomWidget::new(vec!["Text".to_string()], AtomWidgetType::NodeTextButton,
            AtomData::new_as_int("text".to_string(), 0));
            atom1.atom_data.text = "Text".to_string();
            let id = (behavior_data.id, node_data.id, "text".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "Hello".to_string()));
            node_widget.widgets.push(atom1);

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
            atom1.atom_data.data = context.data.get_behavior_id_value(id, (-1.0,0.0,0.0,0.0, "".to_string()));
            node_widget.widgets.push(atom1);

            let mut atom2 = AtomWidget::new(vec!["Delay".to_string()], AtomWidgetType::NodeExpressionValueButton,
            AtomData::new_as_int("delay".to_string(), 0));
            atom2.atom_data.text = "Delay".to_string();
            let id = (behavior_data.id, node_data.id, "delay".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "1".to_string()));
            node_widget.widgets.push(atom2);

            node_widget.color = context.color_blue.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Right, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Success, NodeConnector { rect: (0,0,0,0) } );
            node_widget.node_connector.insert(BehaviorNodeConnector::Fail, NodeConnector { rect: (0,0,0,0) } );
        }
    }

    /// Sets up the corner node widget
    pub fn setup_corner_node_widget(&mut self, behavior_data: &GameBehaviorData, node_data: &BehaviorNode, node_widget: &mut NodeWidget, context: &ScreenContext) {
        let type_index : usize;

        // Get the type index
        if let Some(index) = node_data.values.get("type") {
            type_index = index.0 as usize;
        } else {
            type_index = 0;
        }

        // Remove all atoms except the first one (the type)
        while node_widget.widgets.len() > 1 {
            node_widget.widgets.remove(1);
        }

        if self.graph_type == GraphType::Behavior {
            if type_index == 0 {
                // Character

                // Position
                let mut position_atom = AtomWidget::new(vec![], AtomWidgetType::NodePositionButton,
                AtomData::new_as_int("position".to_string(), 0));
                position_atom.atom_data.text = "position".to_string();
                let id = (behavior_data.id, node_data.id, "position".to_string());
                position_atom.behavior_id = Some(id.clone());
                position_atom.atom_data.data = context.data.get_behavior_id_value(id, (-1.0,0.0,0.0,0.0, "".to_string()));
                node_widget.widgets.push(position_atom);

                // Default Character Tile
                let mut tile_atom = AtomWidget::new(vec![], AtomWidgetType::NodeCharTileButton,
                    AtomData::new_as_int("tile".to_string(), 0));
                tile_atom.atom_data.text = "tile".to_string();
                let id = (behavior_data.id, node_data.id, "tile".to_string());
                tile_atom.behavior_id = Some(id.clone());
                tile_atom.atom_data.data = context.data.get_behavior_id_value(id, (-1.0,0.0,0.0,0.0, "".to_string()));
                node_widget.widgets.push(tile_atom);
            }
        }
    }

    /// Converts the index of a node widget to a node id
    pub fn widget_index_to_node_id(&self, index: usize) -> usize {
        self.nodes[index].id
    }

    /// Converts the id of a node to a widget index
    pub fn node_id_to_widget_index(&self, id: usize) -> usize {
        for index in 0..self.nodes.len() {
            if self.nodes[index].id == id {
                return index;
            }
        }
        0
    }

    /// Returns true if the node connector is a source connector (Right or Bottom)
    pub fn connector_is_source(&self, connector: BehaviorNodeConnector) -> bool {
        if connector == BehaviorNodeConnector::Right || connector == BehaviorNodeConnector::Bottom || connector == BehaviorNodeConnector::Success || connector == BehaviorNodeConnector::Fail {
            return true;
        }
        false
    }

    /// Disconnect the node from all connections
    fn disconnect_node(&mut self, id: usize, context: &mut ScreenContext) {

        if let Some(behavior) = context.data.behaviors.get_mut(&context.curr_behavior_index) {
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
        if let Some(behavior) = context.data.behaviors.get_mut(&context.curr_behavior_index) {
            behavior.data.nodes.remove(&id);
            behavior.save_data();
        }
    }

    /// Sets the widget and behavior data for the given atom id
    pub fn set_node_atom_data(&mut self, node_atom_id: (usize, usize, String), data: (f64, f64, f64, f64, String), context: &mut ScreenContext) {
        for index in 0..self.nodes.len() {
            if self.nodes[index].id == node_atom_id.1 {
                for atom_index in 0..self.nodes[index].widgets.len() {
                    if self.nodes[index].widgets[atom_index].atom_data.id == node_atom_id.2 {
                        self.nodes[index].widgets[atom_index].atom_data.data = data.clone();
                        self.nodes[index].widgets[atom_index].dirty = true;
                        self.nodes[index].dirty = true;
                        self.dirty = true;

                        context.data.set_behavior_id_value(node_atom_id.clone(), data.clone());

                        break;
                    }
                }
            }
        }
    }
}