use crate::editor::node::{NodeConnector, NodeWidget};
use crate::atom:: { AtomData, AtomWidget, AtomWidgetType };
use crate::editor::node_preview::NodePreviewWidget;

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

                // Draw nodes
                for index in 0..self.nodes.len() {
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

                        let source_connector = &source_node.node_connector[source_connector];
                        let dest_connector = &dest_node.node_connector[dest_connector];

                        let start_x = source_rect.0 + source_connector.rect.0 as isize + source_connector.rect.2 as isize / 2;
                        let start_y = source_rect.1 + source_connector.rect.1 as isize + source_connector.rect.3 as isize / 2;

                        let end_x = dest_rect.0 + dest_connector.rect.0 as isize + dest_connector.rect.2 as isize / 2;
                        let end_y = dest_rect.1 + dest_connector.rect.1 as isize + dest_connector.rect.3 as isize / 2;

                        //context.draw2d.draw_line_safe(&mut self.buffer[..], &(start_x, start_y), &(end_x, end_y), &safe_rect, safe_rect.2, &context.node_connector_color);

                        //path += format!("M {},{} L {},{}", start_x, start_y, end_x, end_y).as_str();
                        path += format!("M {},{} C {},{} {},{} {},{}", start_x, start_y, start_x, start_y + 50, end_x, end_y - 50, end_x, end_y).as_str();
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

        if relative == false {
            x += self.rect.0 as isize;
            y += self.rect.1 as isize;
        }

        (x, y, self.nodes[node_index].size.0, self.nodes[node_index].size.1)
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
            if let Some(preview) = &mut self.preview {
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
                    } else {
                        behavior.data.connections.push((self.widget_index_to_node_id(source_conn.1), source_conn.0, self.widget_index_to_node_id(dest_conn.1), dest_conn.0));
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

    pub fn mouse_wheel(&mut self, delta: (isize, isize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        self.offset.0 -= delta.0 / 20;
        self.offset.1 += delta.1 / 20;
        self.dirty = true;
        true
    }

    // pub fn mouse_hover(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {
    //     false
    // }

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

            let mut node_type : BehaviorNodeType = BehaviorNodeType::BehaviorTree;

            if name == "Dice Roll" {
                node_type = BehaviorNodeType::DiceRoll;
            }

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
    pub fn init_node_widget(&mut self, behavior_data: &GameBehaviorData, behavior_node: &BehaviorNode, node_widget: &mut NodeWidget, context: &ScreenContext) {

        if behavior_node.behavior_type == BehaviorNodeType::BehaviorTree {
            let mut atom1 = AtomWidget::new(vec!["Always".to_string(), "On Startup".to_string(), "On Demand".to_string()], AtomWidgetType::NodeMenuButton,
            AtomData::new_as_int("execute".to_string(), 0));
            atom1.atom_data.text = "Execute".to_string();
            let id = (behavior_data.id, behavior_node.id, "execute".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.curr_index = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0)).0 as usize;
            node_widget.widgets.push(atom1);
            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Bottom, NodeConnector { rect: (0,0,0,0) } );
        } else
        if behavior_node.behavior_type == BehaviorNodeType::DiceRoll {

            let mut atom1 = AtomWidget::new(vec!["Throws".to_string()], AtomWidgetType::NodeIntSlider,
            AtomData::new_as_int_range("throws".to_string(), 1, 1, 5, 1));
            atom1.atom_data.text = "Throws".to_string();
            let id = (behavior_data.id, behavior_node.id, "throws".to_string());
            atom1.behavior_id = Some(id.clone());
            atom1.atom_data.data = context.data.get_behavior_id_value(id, (1.0,1.0,5.0,1.0));
            node_widget.widgets.push(atom1);

            //a d4, a d6, a d8, one or two d10s, a d12 and a d20.
            let mut atom2 = AtomWidget::new(vec!["D4".to_string(), "D6".to_string(), "D8".to_string(), "D10".to_string(), "D12".to_string(),  "D20".to_string()], AtomWidgetType::NodeMenuButton,
            AtomData::new_as_int("dice".to_string(), 0));
            atom2.atom_data.text = "Dice".to_string();
            let id = (behavior_data.id, behavior_node.id, "dice".to_string());
            atom2.behavior_id = Some(id.clone());
            atom2.curr_index = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0)).0 as usize;
            node_widget.widgets.push(atom2);

            node_widget.color = context.color_green.clone();
            node_widget.node_connector.insert(BehaviorNodeConnector::Top, NodeConnector { rect: (0,0,0,0) } );
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
        if connector == BehaviorNodeConnector::Right || connector == BehaviorNodeConnector::Bottom {
            return true;
        }
        false
    }
}