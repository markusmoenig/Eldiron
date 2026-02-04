use crate::prelude::*;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use zeno::{Mask, Stroke};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TheNodeUIImages {
    NormalTopLeft,
    NormalTopMiddle,
    NormalTopRight,
    SelectedTopLeft,
    SelectedTopMiddle,
    SelectedTopRight,
    NormalBottomLeft,
    NormalBottomMiddle,
    NormalBottomRight,
    SelectedBottomLeft,
    SelectedBottomMiddle,
    SelectedBottomRight,
    PreviewArea,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub enum TheNodeAction {
    #[default]
    None,
    DragNode,
    ConnectingTerminal(usize, bool, u8),
    CutConnection,
}

use TheNodeUIImages::*;

pub struct TheNodeCanvasView {
    id: TheId,
    limiter: TheSizeLimiter,
    state: TheWidgetState,

    action: TheNodeAction,

    render_buffer: TheRGBABuffer,
    wheel_scale: f32,
    accumulated_wheel_delta: Vec2<f32>,

    canvas: TheNodeCanvas,
    node_rects: Vec<TheDim>,
    terminal_rects: Vec<(Vec<TheDim>, Vec<TheDim>)>,

    dim: TheDim,

    is_dirty: bool,

    node_ui_images: FxHashMap<TheNodeUIImages, TheRGBABuffer>,

    drag_start: Vec2<i32>,
    drag_offset: Vec2<i32>,

    action_changed: bool,

    overlay: Option<TheRGBABuffer>,
}

impl TheWidget for TheNodeCanvasView {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(20, 20));
        Self {
            id,
            limiter,
            state: TheWidgetState::None,

            action: TheNodeAction::None,

            render_buffer: TheRGBABuffer::new(TheDim::new(0, 0, 20, 20)),
            wheel_scale: -0.4,
            accumulated_wheel_delta: Vec2::zero(),

            canvas: TheNodeCanvas::default(),
            node_rects: Vec::new(),
            terminal_rects: Vec::new(),

            dim: TheDim::zero(),

            is_dirty: false,

            node_ui_images: FxHashMap::default(),

            drag_start: Vec2::zero(),
            drag_offset: Vec2::zero(),

            action_changed: false,

            overlay: None,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    #[allow(clippy::single_match)]
    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        // println!("event ({}): {:?}", self.widget_id.name, event);
        match event {
            TheEvent::MouseDown(coord) => {
                if self.state == TheWidgetState::Selected {
                    self.state = TheWidgetState::None;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                } else if self.state != TheWidgetState::Selected {
                    self.state = TheWidgetState::Selected;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                }
                ctx.ui.set_focus(self.id());
                self.action_changed = false;

                let selected = self.node_index_at(coord);
                if let Some(index) = selected {
                    if selected != self.canvas.selected_node {
                        self.canvas.selected_node = Some(index);
                        self.is_dirty = true;
                        redraw = true;

                        ctx.ui.send(TheEvent::NodeSelectedIndexChanged(
                            self.id().clone(),
                            self.canvas.selected_node,
                        ));
                    }

                    if let Some(terminal) = self.terminal_at(
                        index,
                        *coord + self.canvas.offset - self.canvas.nodes[index].position,
                    ) {
                        self.drag_start = *coord;
                        self.drag_offset = *coord;
                        if let Some(rect) = self.terminal_rect_for(index, terminal.0, terminal.1) {
                            self.drag_start = Vec2::new(
                                self.canvas.nodes[index].position.x + rect.x + rect.width / 2,
                                self.canvas.nodes[index].position.y + rect.y + rect.height / 2,
                            );
                            self.drag_offset = self.drag_start;
                        }
                        self.action =
                            TheNodeAction::ConnectingTerminal(index, terminal.0, terminal.1);
                    } else {
                        self.drag_start = *coord;
                        self.action = TheNodeAction::DragNode;
                    }
                } else {
                    self.action = TheNodeAction::CutConnection;
                    self.drag_start = *coord;
                }
            }
            TheEvent::MouseDragged(coord) => {
                if let TheNodeAction::CutConnection = self.action {
                    self.drag_offset = *coord;
                    self.is_dirty = true;
                    self.action_changed = true;
                    redraw = true;
                } else if let TheNodeAction::ConnectingTerminal(_, _, _) = self.action {
                    self.drag_offset = *coord;
                    self.is_dirty = true;
                    self.action_changed = true;
                    redraw = true;
                } else if self.action == TheNodeAction::DragNode {
                    if let Some(index) = self.canvas.selected_node {
                        let displacement =
                            Vec2::new(coord.x - self.drag_start.x, coord.y - self.drag_start.y);

                        // Move the selected node
                        self.canvas.nodes[index].position.x += displacement.x;
                        self.canvas.nodes[index].position.y += displacement.y;

                        // Now find and move all connected nodes
                        let connected_nodes = self.find_connected_nodes(index);
                        for &connected_index in &connected_nodes {
                            self.canvas.nodes[connected_index].position.x += displacement.x;
                            self.canvas.nodes[connected_index].position.y += displacement.y;
                        }

                        self.drag_start = *coord;
                        self.is_dirty = true;
                        self.action_changed = true;
                        redraw = true;
                    }
                }
            }
            TheEvent::MouseUp(coord) => {
                if let TheNodeAction::ConnectingTerminal(
                    source_node_index,
                    source_output,
                    source_terminal_index,
                ) = self.action
                {
                    if let Some(dest_node_index) = self.node_index_at(coord) {
                        if let Some((dest_output, dest_terminal_index)) = self.terminal_at(
                            dest_node_index,
                            *coord + self.canvas.offset
                                - self.canvas.nodes[dest_node_index].position,
                        ) {
                            if source_node_index != dest_node_index && source_output != dest_output
                            {
                                if source_output {
                                    self.canvas.connections.push((
                                        source_node_index as u16,
                                        source_terminal_index,
                                        dest_node_index as u16,
                                        dest_terminal_index,
                                    ));
                                } else {
                                    self.canvas.connections.push((
                                        dest_node_index as u16,
                                        dest_terminal_index,
                                        source_node_index as u16,
                                        source_terminal_index,
                                    ));
                                }

                                ctx.ui.send(TheEvent::NodeConnectionAdded(
                                    self.id().clone(),
                                    self.canvas.connections.clone(),
                                ));
                            }
                        }
                    }

                    self.is_dirty = true;
                    redraw = true;
                } else if self.action == TheNodeAction::DragNode && self.action_changed {
                    if let Some(index) = self.canvas.selected_node {
                        ctx.ui.send(TheEvent::NodeDragged(
                            self.id().clone(),
                            index,
                            self.canvas.nodes[index].position,
                        ));
                    }
                } else if self.action == TheNodeAction::CutConnection && self.action_changed {
                    let mut new_connections = vec![];
                    let cut_start = (self.drag_start.x, self.drag_start.y);
                    let cut_end = (self.drag_offset.x, self.drag_offset.y);
                    let mut changed = false;
                    for (
                        source_node_index,
                        source_output_index,
                        dest_node_index,
                        dest_input_index,
                    ) in self.canvas.connections.iter()
                    {
                        if let Some(mut output) = self.terminal_rect_for(
                            *source_node_index as usize,
                            true,
                            *source_output_index,
                        ) {
                            output.x += 10 + 6 - self.canvas.offset.x;
                            output.y -= self.canvas.offset.y;
                            output.y += output.height / 2;
                            output.x += self.canvas.nodes[*source_node_index as usize].position.x;
                            output.y += self.canvas.nodes[*source_node_index as usize].position.y;
                            if let Some(mut input) = self.terminal_rect_for(
                                *dest_node_index as usize,
                                false,
                                *dest_input_index,
                            ) {
                                input.x -= 6 + self.canvas.offset.x;
                                input.y -= self.canvas.offset.y;
                                input.y += input.height / 2;
                                input.x += self.canvas.nodes[*dest_node_index as usize].position.x;
                                input.y += self.canvas.nodes[*dest_node_index as usize].position.y;

                                if !do_intersect(
                                    (output.x, output.y),
                                    (input.x, input.y),
                                    cut_start,
                                    cut_end,
                                ) {
                                    new_connections.push((
                                        *source_node_index,
                                        *source_output_index,
                                        *dest_node_index,
                                        *dest_input_index,
                                    ));
                                } else {
                                    changed = true;
                                }
                            }
                        }
                    }
                    if changed {
                        self.canvas.connections = new_connections;
                        ctx.ui.send(TheEvent::NodeConnectionRemoved(
                            self.id().clone(),
                            self.canvas.connections.clone(),
                        ));
                    }
                    self.is_dirty = true;
                    redraw = true;
                }

                self.action = TheNodeAction::None;
            }
            TheEvent::Hover(_coord) => {
                if !self.id().equals(&ctx.ui.hover) {
                    self.is_dirty = true;
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }
            }
            TheEvent::MouseWheel(delta) => {
                let scale_factor = self.wheel_scale; // * 1.0 / (self.zoom.powf(0.5));

                let aspect_ratio = self.dim().width as f32 / self.dim().height as f32;

                let scale_x = if aspect_ratio > 1.0 {
                    1.0 / aspect_ratio
                } else {
                    1.0
                };
                let scale_y = if aspect_ratio < 1.0 {
                    aspect_ratio
                } else {
                    1.0
                };

                // Update accumulated deltas
                self.accumulated_wheel_delta.x += delta.x as f32 * scale_factor * scale_x;
                self.accumulated_wheel_delta.y += delta.y as f32 * scale_factor * scale_y;

                let minimum_delta_threshold = 2.0;

                // Check if accumulated deltas exceed the threshold
                if self.accumulated_wheel_delta.x.abs() > minimum_delta_threshold
                    || self.accumulated_wheel_delta.y.abs() > minimum_delta_threshold
                {
                    // Convert accumulated deltas to integer and reset
                    let d = Vec2::new(
                        self.accumulated_wheel_delta.x as i32,
                        self.accumulated_wheel_delta.y as i32,
                    );
                    self.accumulated_wheel_delta = Vec2::zero();

                    self.canvas.offset += d;

                    ctx.ui.send(TheEvent::NodeViewScrolled(
                        self.id().clone(),
                        self.canvas.offset,
                    ));

                    self.is_dirty = true;
                    redraw = true;
                }
            }
            TheEvent::KeyCodeDown(code) => {
                if code.to_key_code().unwrap() == TheKeyCode::Delete {
                    if let Some(deleted_node_index) = self.canvas.selected_node {
                        if self.canvas.nodes[deleted_node_index].can_be_deleted {
                            self.canvas.nodes.remove(deleted_node_index);
                            self.node_rects.remove(deleted_node_index);

                            // Filter out connections involving the deleted node and adjust indices for others
                            self.canvas.connections.retain_mut(
                                |(src_node_idx, _, dest_node_idx, _)| {
                                    let src_index = *src_node_idx as usize;
                                    let dest_index = *dest_node_idx as usize;

                                    if src_index == deleted_node_index
                                        || dest_index == deleted_node_index
                                    {
                                        // Connection involves the deleted node, so remove it
                                        false
                                    } else {
                                        // Adjust indices for remaining connections
                                        if src_index > deleted_node_index {
                                            *src_node_idx -= 1;
                                        }
                                        if dest_index > deleted_node_index {
                                            *dest_node_idx -= 1;
                                        }
                                        true
                                    }
                                },
                            );

                            ctx.ui.send(TheEvent::NodeDeleted(
                                self.id().clone(),
                                deleted_node_index,
                                self.canvas.connections.clone(),
                            ));

                            self.is_dirty = true;
                            redraw = true;
                        }
                    }
                }
            }
            _ => {}
        }
        redraw
    }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
            self.is_dirty = true;
            self.render_buffer.resize(dim.width, dim.height);
        }
    }

    fn set_state(&mut self, state: TheWidgetState) {
        self.state = state;
        self.is_dirty = true;
    }

    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }

    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }

    fn needs_redraw(&mut self) -> bool {
        self.is_dirty
    }

    fn set_needs_redraw(&mut self, redraw: bool) {
        self.is_dirty = redraw;
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim().is_valid() {
            return;
        }

        if self.node_ui_images.is_empty() {
            self.fill_node_ui_images(ctx);
        }

        //self.render_buffer.fill([128, 128, 128, 255]);

        let width = self.render_buffer.dim().width as usize;
        let height = self.render_buffer.dim().height;

        let pixels = self.render_buffer.pixels_mut();
        pixels
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    fn mix_color(a: &[u8; 4], b: &[u8; 4], v: f32) -> [u8; 4] {
                        [
                            (((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v) * 255.0)
                                as u8,
                            (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v) * 255.0)
                                as u8,
                            (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v) * 255.0)
                                as u8,
                            255,
                        ]
                    }

                    let mut color = [128, 128, 128, 255];

                    let xx = (i % width) as i32;
                    let yy = height - (i / width) as i32;

                    let m_x = (xx + self.canvas.offset.x) % 40;
                    let m_y = (yy + self.canvas.offset.y) % 40;

                    // if (m_x.abs() <= 1 && xx != -1) || (m_y.abs() <= 1 && yy != -1) {
                    if m_x == 0 || m_y == 0 {
                        color = [81, 81, 81, 255];
                    }

                    if let Some(overlay) = &self.overlay {
                        //let w = overlay.dim().width;
                        let h = overlay.dim().height;

                        if yy > height - h && xx < width as i32 {
                            let y = height - h;
                            if let Some(c) = overlay.get_pixel(xx, yy - y) {
                                color = mix_color(&color, &c, c[3] as f32 / 255.0);
                            }
                        }
                    }

                    pixel.copy_from_slice(&color);
                }
            });

        let rbw = self.render_buffer.dim().width as usize;
        let rbh = self.render_buffer.dim().height as usize;

        let node_width = self.canvas.node_width;
        let node_rects = Arc::new(Mutex::new(Vec::new()));

        // Draw a node
        let draw_node = |index: usize, node: &TheNode| {
            let max_terminals = node.inputs.len().max(node.outputs.len()) as i32;
            let mut body_height = 7 + max_terminals * 10 + (max_terminals - 1) * 4 + 7;
            let mut node_height = 19 + body_height + 19;
            let mut preview_height = 0;

            if node.supports_preview && node.preview_is_open {
                preview_height = 118;
                node_height += preview_height;
                body_height += preview_height;
            }

            let dim = TheDim::new(
                node.position.x - self.canvas.offset.x,
                node.position.y - self.canvas.offset.y,
                node_width,
                node_height,
            );

            let mut nb = TheRGBABuffer::new(TheDim::sized(node_width, node_height));

            let is_selected = Some(index) == self.canvas.selected_node;

            // Header

            if is_selected {
                nb.copy_into(0, 0, self.node_ui_images.get(&SelectedTopLeft).unwrap());

                for i in 0..(node_width - 18) {
                    nb.copy_into(
                        9 + i,
                        0,
                        self.node_ui_images.get(&SelectedTopMiddle).unwrap(),
                    );
                }

                nb.copy_into(
                    node_width - 9,
                    0,
                    self.node_ui_images.get(&SelectedTopRight).unwrap(),
                );
            } else {
                nb.copy_into(2, 2, self.node_ui_images.get(&NormalTopLeft).unwrap());

                for i in 0..(node_width - 18) {
                    nb.copy_into(9 + i, 2, self.node_ui_images.get(&NormalTopMiddle).unwrap());
                }

                nb.copy_into(
                    node_width - 9,
                    2,
                    self.node_ui_images.get(&NormalTopRight).unwrap(),
                );
            }

            if let Some(font) = &ctx.ui.font {
                nb.draw_text(
                    Vec2::new(12, 4),
                    font,
                    &node.name,
                    10.0,
                    [188, 188, 188, 255],
                    TheHorizontalAlign::Left,
                    TheVerticalAlign::Top,
                );
            }

            // Body
            for _y in 0..body_height {
                let y = _y + 19;
                for x in 0..node_width {
                    if x < 2 {
                        if is_selected {
                            if x == 0 {
                                nb.set_pixel(x, y, &[255, 255, 255, 55]);
                            } else {
                                nb.set_pixel(x, y, &[255, 255, 255, 166]);
                            }
                        }
                        continue;
                    } else if x >= node_width - 2 {
                        if is_selected {
                            if x == node_width - 1 {
                                nb.set_pixel(x, y, &[255, 255, 255, 55]);
                            } else {
                                nb.set_pixel(x, y, &[255, 255, 255, 166]);
                            }
                        }
                        continue;
                    }

                    if x == node_width - 3 || _y == body_height - 1 {
                        nb.set_pixel(x, y, &[44, 44, 44, 255]);
                    } else if x == node_width - 4 && _y > 1 {
                        nb.set_pixel(x, y, &[162, 162, 162, 255]);
                    } else if x == 2 {
                        nb.set_pixel(x, y, &[112, 112, 112, 255]);
                    } else if x == 3 && _y > 0 {
                        nb.set_pixel(x, y, &[137, 137, 137, 255]);
                    } else if _y == 0 {
                        nb.set_pixel(x, y, &[82, 82, 82, 255]);
                    } else if _y == 1 {
                        nb.set_pixel(x, y, &[137, 137, 137, 255]);
                    } else if _y == body_height - 2 {
                        nb.set_pixel(x, y, &[162, 162, 162, 255]);
                    } else {
                        nb.set_pixel(x, y, &[179, 179, 179, 255]);
                    }
                }
            }

            // Terminals

            let mut terminal_rects: (Vec<TheDim>, Vec<TheDim>) = (Vec::new(), Vec::new());

            let mut y = 19 + 7;
            for i in &node.inputs {
                let dim = TheDim::new(2 + 5, y, 10, 10);

                if let Some(font) = &ctx.ui.font {
                    let text_width = 80;
                    let mut tb = TheRGBABuffer::new(TheDim::sized(text_width, 10));
                    tb.draw_text(
                        Vec2::new(0, 0),
                        font,
                        &i.name,
                        9.5,
                        [82, 82, 82, 255],
                        TheHorizontalAlign::Left,
                        TheVerticalAlign::Center,
                    );
                    nb.blend_into(dim.x + 10 + 2, y, &tb)
                }

                nb.draw_disc(
                    &dim,
                    &self.color_for(&i.category_name).to_u8_array(),
                    1.0,
                    &[105, 105, 105, 255],
                );
                terminal_rects.0.push(dim);
                y += 10 + 4;
            }

            let mut y = 19 + 7;
            for o in &node.outputs {
                let dim = TheDim::new(node_width - 2 - 5 - 10, y, 10, 10);

                if let Some(font) = &ctx.ui.font {
                    let text_width = 80;
                    let mut tb = TheRGBABuffer::new(TheDim::sized(text_width, 10));
                    tb.draw_text(
                        Vec2::new(0, 0),
                        font,
                        &o.name,
                        9.5,
                        [82, 82, 82, 255],
                        TheHorizontalAlign::Right,
                        TheVerticalAlign::Center,
                    );
                    nb.blend_into(dim.x - 80 - 2, y, &tb)
                }

                // nb.draw_disc(&dim, &[245, 245, 245, 255], 1.0, &[105, 105, 105, 255]);
                nb.draw_disc(
                    &dim,
                    &self.color_for(&o.category_name).to_u8_array(),
                    1.0,
                    &[105, 105, 105, 255],
                );
                terminal_rects.1.push(dim);
                y += 10 + 4;
            }

            // Preview

            if preview_height > 0 {
                let mut y = node_height - 19 - preview_height;
                nb.copy_into(2, y, self.node_ui_images.get(&PreviewArea).unwrap());
                if node.preview.is_valid() {
                    let x = 3 + (node_width - 4 - node.preview.dim().width) / 2;
                    y += (preview_height - node.preview.dim().height) / 2 - 1;
                    nb.blend_into(x, y, &node.preview);
                }
            }

            // Footer
            if is_selected {
                nb.copy_into(
                    0,
                    node_height - 19,
                    self.node_ui_images.get(&SelectedBottomLeft).unwrap(),
                );

                for i in 0..(node_width - 18) {
                    nb.copy_into(
                        9 + i,
                        node_height - 19,
                        self.node_ui_images.get(&SelectedBottomMiddle).unwrap(),
                    );
                }

                nb.copy_into(
                    node_width - 9,
                    node_height - 19,
                    self.node_ui_images.get(&SelectedBottomRight).unwrap(),
                );
            } else {
                nb.copy_into(
                    2,
                    node_height - 19,
                    self.node_ui_images.get(&NormalBottomLeft).unwrap(),
                );

                for i in 0..(node_width - 18) {
                    nb.copy_into(
                        9 + i,
                        node_height - 19,
                        self.node_ui_images.get(&NormalBottomMiddle).unwrap(),
                    );
                }

                nb.copy_into(
                    node_width - 9,
                    node_height - 19,
                    self.node_ui_images.get(&NormalBottomRight).unwrap(),
                );
            }

            let mut node_rects = node_rects.lock().unwrap();
            node_rects.push((index, dim, nb, terminal_rects));
        };

        // Parallel iteration over the nodes to draw them
        self.canvas
            .nodes
            .par_iter()
            .enumerate()
            .for_each(|(index, node)| {
                draw_node(index, node);
            });

        // Sort the node rects by index
        let mut sorted_node_rects = Arc::try_unwrap(node_rects).unwrap().into_inner().unwrap();
        sorted_node_rects.sort_by_key(|&(index, _, _, _)| index);

        // First pass to draw all nodes except the selected one
        self.node_rects.clear();
        self.terminal_rects.clear();
        for (index, dim, nb, tr) in &sorted_node_rects {
            if Some(*index) != self.canvas.selected_node {
                self.render_buffer.blend_into(dim.x, dim.y, nb);
            }
            self.node_rects.push(*dim);
            self.terminal_rects.push(tr.clone());
        }

        // Second pass to draw the selected node
        for (index, dim, nb, _) in &sorted_node_rects {
            if Some(*index) == self.canvas.selected_node {
                self.render_buffer.blend_into(dim.x, dim.y, nb);
            }
        }

        // Draw Connections

        let mut line_mask: Vec<u8> =
            vec![0; (self.render_buffer.dim().width * self.render_buffer.dim().height) as usize];
        let mut line_path: String = str!("");

        for (source_node_index, source_output_index, dest_node_index, dest_input_index) in
            self.canvas.connections.iter()
        {
            if let Some(mut output) =
                self.terminal_rect_for(*source_node_index as usize, true, *source_output_index)
            {
                output.x += 10 + 6 - self.canvas.offset.x;
                output.y -= self.canvas.offset.y;
                output.y += output.height / 2;
                output.x += self.canvas.nodes[*source_node_index as usize].position.x;
                output.y += self.canvas.nodes[*source_node_index as usize].position.y;
                if let Some(mut input) =
                    self.terminal_rect_for(*dest_node_index as usize, false, *dest_input_index)
                {
                    input.x -= 6 + self.canvas.offset.x;
                    input.y -= self.canvas.offset.y;
                    input.y += input.height / 2;
                    input.x += self.canvas.nodes[*dest_node_index as usize].position.x;
                    input.y += self.canvas.nodes[*dest_node_index as usize].position.y;

                    let dx = output.x - input.x;
                    let dy = output.y - input.y;

                    let d = ((dx * dx + dy * dy) as f32).sqrt().clamp(0.0, 50.0) as i32;

                    let control_start_x = output.x + d;
                    let control_start_y = output.y as isize;

                    let control_end_x = input.x - d;
                    let control_end_y = input.y as isize;

                    line_path += format!(
                        "M {},{} C {},{} {},{} {},{}",
                        output.x,
                        output.y,
                        control_start_x,
                        control_start_y,
                        control_end_x,
                        control_end_y,
                        input.x,
                        input.y
                    )
                    .as_str();
                }
            }
        }

        // Draw ongoing connection attempt
        if let TheNodeAction::ConnectingTerminal(_, _, _) = self.action {
            line_path += format!(
                "M {},{} L {},{}",
                self.drag_start.x - self.canvas.offset.x,
                self.drag_start.y - self.canvas.offset.y,
                self.drag_offset.x,
                self.drag_offset.y
            )
            .as_str();
        }

        if !line_path.is_empty() {
            Mask::new(line_path.as_str())
                .size(
                    self.render_buffer.dim().width as u32,
                    self.render_buffer.dim().height as u32,
                )
                .style(Stroke::new(1.5))
                .render_into(&mut line_mask, None);

            ctx.draw.blend_mask(
                self.render_buffer.pixels_mut(),
                &(0, 0, rbw, rbh),
                rbw,
                &line_mask[..],
                &(rbw, rbh),
                &[90, 90, 90, 255],
            );
        }

        // Draw ongoing cut connection attempt
        if let TheNodeAction::CutConnection = self.action {
            self.render_buffer.draw_line(
                self.drag_start.x,
                self.drag_start.y,
                self.drag_offset.x,
                self.drag_offset.y,
                [209, 42, 42, 255],
            );
        }

        // Copy the render buffer to the main buffer
        buffer.copy_into(self.dim.buffer_x, self.dim.buffer_y, &self.render_buffer);

        // Draw the focus rectangle if necessary
        let stride = buffer.stride();
        if Some(self.id.clone()) == ctx.ui.focus {
            let tuple = self.dim().to_buffer_utuple();
            ctx.draw.rect_outline(
                buffer.pixels_mut(),
                &tuple,
                stride,
                style.theme().color(DefaultSelection),
            );
        }
        self.is_dirty = false;
    }

    fn supports_hover(&mut self) -> bool {
        true
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_node_canvas_view(&mut self) -> Option<&mut dyn TheNodeCanvasViewTrait> {
        Some(self)
    }
}

pub trait TheNodeCanvasViewTrait: TheWidget {
    fn find_connected_nodes(&self, node_index: usize) -> Vec<usize>;
    fn set_canvas(&mut self, canvas: TheNodeCanvas);
    fn set_overlay(&mut self, overlay: Option<TheRGBABuffer>);
    fn set_node_preview(&mut self, index: usize, buffer: TheRGBABuffer);
    fn fill_node_ui_images(&mut self, ctx: &mut TheContext);
    fn node_index_at(&self, coord: &Vec2<i32>) -> Option<usize>;
    fn terminal_at(&self, node_index: usize, coord: Vec2<i32>) -> Option<(bool, u8)>;
    fn terminal_rect_for(&self, node_index: usize, input: bool, index: u8) -> Option<TheDim>;
    fn color_for(&self, name: &str) -> TheColor;
}

impl TheNodeCanvasViewTrait for TheNodeCanvasView {
    fn find_connected_nodes(&self, node_index: usize) -> Vec<usize> {
        let mut connected_nodes = FxHashSet::default(); // Use a set to avoid duplicates
        let mut stack = vec![node_index];

        while let Some(current_node) = stack.pop() {
            // Find all connections where the current node is the source (output terminal)
            for &(src, _, dest, _) in &self.canvas.connections {
                if src as usize == current_node && !connected_nodes.contains(&(dest as usize)) {
                    connected_nodes.insert(dest as usize);
                    stack.push(dest as usize);
                }
            }
        }

        connected_nodes.into_iter().collect()
    }
    fn set_canvas(&mut self, canvas: TheNodeCanvas) {
        self.canvas = canvas;
        self.is_dirty = true;
    }
    fn set_overlay(&mut self, overlay: Option<TheRGBABuffer>) {
        self.overlay = overlay;
        self.is_dirty = true;
    }
    fn set_node_preview(&mut self, index: usize, buffer: TheRGBABuffer) {
        if !self.canvas.nodes.is_empty() {
            self.canvas.nodes[index].preview = buffer;
            self.is_dirty = true;
        }
    }
    fn fill_node_ui_images(&mut self, ctx: &mut TheContext) {
        self.node_ui_images.clear();
        self.node_ui_images.insert(
            SelectedTopLeft,
            ctx.ui.icon("dark_node_selected_topleft").unwrap().clone(),
        );
        self.node_ui_images.insert(
            SelectedTopMiddle,
            ctx.ui.icon("dark_node_selected_topmiddle").unwrap().clone(),
        );
        self.node_ui_images.insert(
            SelectedTopRight,
            ctx.ui.icon("dark_node_selected_topright").unwrap().clone(),
        );
        self.node_ui_images.insert(
            NormalTopLeft,
            ctx.ui.icon("dark_node_normal_topleft").unwrap().clone(),
        );
        self.node_ui_images.insert(
            NormalTopMiddle,
            ctx.ui.icon("dark_node_normal_topmiddle").unwrap().clone(),
        );
        self.node_ui_images.insert(
            NormalTopRight,
            ctx.ui.icon("dark_node_normal_topright").unwrap().clone(),
        );
        self.node_ui_images.insert(
            SelectedBottomLeft,
            ctx.ui
                .icon("dark_node_selected_bottomleft")
                .unwrap()
                .clone(),
        );
        self.node_ui_images.insert(
            SelectedBottomMiddle,
            ctx.ui
                .icon("dark_node_selected_bottommiddle")
                .unwrap()
                .clone(),
        );
        self.node_ui_images.insert(
            SelectedBottomRight,
            ctx.ui
                .icon("dark_node_selected_bottomright")
                .unwrap()
                .clone(),
        );
        self.node_ui_images.insert(
            NormalBottomLeft,
            ctx.ui.icon("dark_node_normal_bottomleft").unwrap().clone(),
        );
        self.node_ui_images.insert(
            NormalBottomMiddle,
            ctx.ui
                .icon("dark_node_normal_bottommiddle")
                .unwrap()
                .clone(),
        );
        self.node_ui_images.insert(
            NormalBottomRight,
            ctx.ui.icon("dark_node_normal_bottomright").unwrap().clone(),
        );
        self.node_ui_images.insert(
            PreviewArea,
            ctx.ui.icon("dark_node_preview_area").unwrap().clone(),
        );
    }
    fn node_index_at(&self, coord: &Vec2<i32>) -> Option<usize> {
        for (i, r) in self.node_rects.iter().enumerate().rev() {
            if r.contains(*coord) {
                return Some(i);
            }
        }
        None
    }
    fn terminal_at(&self, node_index: usize, coord: Vec2<i32>) -> Option<(bool, u8)> {
        if let Some((i_vec, o_vec)) = self.terminal_rects.get(node_index) {
            for (i, r) in i_vec.iter().enumerate() {
                if r.contains(coord) {
                    return Some((false, i as u8));
                }
            }
            for (i, r) in o_vec.iter().enumerate() {
                if r.contains(coord) {
                    return Some((true, i as u8));
                }
            }
        }
        None
    }

    fn terminal_rect_for(&self, node_index: usize, output: bool, index: u8) -> Option<TheDim> {
        if let Some((i_vec, o_vec)) = self.terminal_rects.get(node_index) {
            if !output {
                if let Some(r) = i_vec.get(index as usize) {
                    return Some(*r);
                }
            } else if let Some(r) = o_vec.get(index as usize) {
                return Some(*r);
            }
        }
        None
    }

    fn color_for(&self, name: &str) -> TheColor {
        if let Some(col) = self.canvas.categories.get(name) {
            col.clone()
        } else {
            TheColor::default()
        }
    }
}

// https://www.geeksforgeeks.org/check-if-two-given-line-segments-intersect/
fn do_intersect(p1: (i32, i32), q1: (i32, i32), p2: (i32, i32), q2: (i32, i32)) -> bool {
    // Given three collinear points p, q, r, the function checks if
    // point q lies on line segment 'pr'
    fn on_segment(p: (i32, i32), q: (i32, i32), r: (i32, i32)) -> bool {
        q.0 <= std::cmp::max(p.0, r.0)
            && q.0 >= std::cmp::min(p.0, r.0)
            && q.1 <= std::cmp::max(p.1, r.1)
            && q.1 >= std::cmp::min(p.1, r.1)
    }

    // To find orientation of ordered triplet (p, q, r).
    // The function returns following values
    // 0 --> p, q and r are collinear
    // 1 --> Clockwise
    // 2 --> Counterclockwise
    fn orientation(p: (i32, i32), q: (i32, i32), r: (i32, i32)) -> i32 {
        let val = (q.1 - p.1) * (r.0 - q.0) - (q.0 - p.0) * (r.1 - q.1);
        if val == 0 {
            return 0;
        } // collinear
        if val > 0 {
            1
        } else {
            2
        } // clock or counterclock wise
    }

    // Check if line segments 'p1q1' and 'p2q2' intersect.
    let o1 = orientation(p1, q1, p2);
    let o2 = orientation(p1, q1, q2);
    let o3 = orientation(p2, q2, p1);
    let o4 = orientation(p2, q2, q1);

    // General case
    if o1 != o2 && o3 != o4 {
        return true;
    }

    // Special Cases
    // p1, q1 and p2 are collinear and p2 lies on segment p1q1
    if o1 == 0 && on_segment(p1, p2, q1) {
        return true;
    }

    // p1, q1 and q2 are collinear and q2 lies on segment p1q1
    if o2 == 0 && on_segment(p1, q2, q1) {
        return true;
    }

    // p2, q2 and p1 are collinear and p1 lies on segment p2q2
    if o3 == 0 && on_segment(p2, p1, q2) {
        return true;
    }

    // p2, q2 and q1 are collinear and q1 lies on segment p2q2
    if o4 == 0 && on_segment(p2, q1, q2) {
        return true;
    }

    // Doesn't fall in any of the above cases
    false
}
