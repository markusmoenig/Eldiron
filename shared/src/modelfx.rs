use crate::prelude::*;
use line_drawing::Bresenham;
use rayon::prelude::*;
use theframework::prelude::*;

fn default_density() -> u8 {
    24
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub enum ModelFXNodeAction {
    #[default]
    None,
    DragNode,
    ConnectingTerminal(Vec3i),
    CutConnection,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ModelFX {
    #[serde(skip)]
    pub action: ModelFXNodeAction,

    /// The nodes which make up the model.
    pub nodes: Vec<ModelFXNode>,

    /// The node connections: Source node index, source terminal, dest node index, dest terminal
    pub connections: Vec<(u16, u8, u16, u8)>,

    /// The density of the voxel grid.
    #[serde(default = "default_density")]
    pub density: u8,

    /// The voxel grid which is the voxelized state of the model and used for rendering.
    #[serde(skip)]
    #[serde(with = "vectorize")]
    // #[serde(skip_deserializing)]
    // #[serde(skip_serializing)]
    pub voxels: FxHashMap<(u8, u8, u8), Voxel>,

    // 70 x 70
    pub preview_buffer: TheRGBABuffer,

    #[serde(skip)]
    pub node_previews: Vec<Option<TheRGBABuffer>>,

    #[serde(skip)]
    pub node_rects: Vec<(usize, usize, usize, usize)>,
    #[serde(skip)]
    pub terminal_rects: Vec<(Vec3i, (usize, usize, usize, usize))>,
    pub zoom: f32,

    pub selected_node: Option<usize>,

    #[serde(skip)]
    pub drag_start: Vec2i,
    #[serde(skip)]
    pub drag_offset: Vec2i,
}

impl Default for ModelFX {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelFX {
    pub fn new() -> Self {
        Self {
            action: ModelFXNodeAction::None,

            nodes: vec![],
            connections: vec![],

            density: 24,
            voxels: FxHashMap::default(),

            preview_buffer: TheRGBABuffer::new(TheDim::sized(65, 65)),
            node_previews: vec![],

            node_rects: vec![],
            terminal_rects: vec![],
            zoom: 1.0,

            selected_node: None,

            drag_start: Vec2i::zero(),
            drag_offset: Vec2i::zero(),
        }
    }

    pub fn hit(&self, _ray: &Ray) -> Option<Hit> {
        None
    }

    pub fn add(&mut self, fx: String) -> bool {
        if let Some(mut node) = ModelFXNode::new_node(&fx, None) {
            node.collection_mut()
                .set("_pos", TheValue::Int2(vec2i(200, 10)));
            self.selected_node = Some(self.nodes.len());
            self.nodes.push(node);
            self.node_previews.push(None);
            return true;
        }
        false
    }

    /// Deletes the selected node and deletes / adjusts connections involving the node.
    pub fn delete(&mut self) {
        if let Some(deleted_node_index) = self.selected_node {
            self.nodes.remove(deleted_node_index);
            self.node_previews.remove(deleted_node_index);

            // Filter out connections involving the deleted node and adjust indices for others
            self.connections
                .retain_mut(|(src_node_idx, _, dest_node_idx, _)| {
                    let src_index = *src_node_idx as usize;
                    let dest_index = *dest_node_idx as usize;

                    if src_index == deleted_node_index || dest_index == deleted_node_index {
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
                });
        }
    }

    /// Clears all node previews.
    pub fn clear_previews(&mut self) {
        for preview in &mut self.node_previews {
            *preview = None;
        }
    }

    /// Remove the preview of the selected node and all connected nodes.
    pub fn remove_current_node_preview(&mut self) {
        if let Some(selected_node) = self.selected_node {
            self.node_previews[selected_node] = None;

            // Remove previews of downstream connected nodes
            for (src_node_idx, _, dest_node_idx, _) in &self.connections {
                if *src_node_idx as usize == selected_node {
                    self.node_previews[*dest_node_idx as usize] = None;
                }
            }
        }
    }

    pub fn build_ui(_ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut rgba_layout = TheRGBALayout::new(TheId::named("ModelFX RGBA Layout"));
        //rgba_layout.set_buffer(TheRGBABuffer::new(TheDim::sized(600, 400)));
        //rgba_layout.limiter_mut().set_max_size(vec2i(600, 400));
        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
            rgba_view.set_mode(TheRGBAViewMode::TileEditor);
            rgba_view.set_grid(Some(1));
            rgba_view.set_background([74, 74, 74, 255]);
        }
        canvas.set_layout(rgba_layout);

        canvas
    }

    pub fn draw(&mut self, ui: &mut TheUI, ctx: &mut TheContext, palette: &ThePalette) {
        let min_x = 0; //std::i32::MAX;
        let min_y = 0; //std::i32::MAX;
        let mut max_x = std::i32::MIN;
        let mut max_y = std::i32::MIN;

        let zoom = self.zoom;
        let node_size = 60;
        let node_size_scaled = (node_size as f32 * zoom) as i32;
        let preview_border_scaled = (4.0 * zoom) as i32;
        let preview_size_scaled = (node_size as f32 * zoom - 2.0 * 4.0 * zoom) as i32 - 1;

        if !self.nodes.is_empty() {
            for node in &self.nodes {
                if let Some(TheValue::Int2(v)) = node.collection().get("_pos") {
                    //min_x = min_x.min(v.x - 10);
                    //min_y = min_y.min(v.y - 10);
                    max_x = max_x.max(v.x + node_size_scaled + 10);
                    max_y = max_y.max(v.y + node_size_scaled + 10);
                }
            }
        }

        let mut width = (max_x - min_x).max(20);
        let mut height = (max_y - min_y).max(20);
        self.node_rects.clear();
        self.terminal_rects.clear();

        if let Some(node_layout) = ui.get_rgba_layout("ModelFX RGBA Layout") {
            if let Some(node_view) = node_layout.rgba_view_mut().as_rgba_view() {
                let dim = node_view.dim();

                width = width.max(dim.width - 13);
                height = height.max(dim.height - 13);

                let mut buffer = TheRGBABuffer::new(TheDim::sized(width, height));
                buffer.fill([74, 74, 74, 255]);

                let mut terminal_colors: FxHashMap<(i32, i32, i32), TheColor> =
                    FxHashMap::default();

                let scaled = |s: usize| -> usize { (s as f32 * zoom) as usize };

                for (i, node) in self.nodes.iter().enumerate() {
                    if let Some(TheValue::Int2(v)) = node.collection().get("_pos") {
                        let node_x = (v.x - min_x) as usize;
                        let node_y = (v.y - min_y) as usize;
                        let rect = (
                            node_x,
                            node_y,
                            node_size_scaled as usize,
                            node_size_scaled as usize,
                        );

                        let border_color = if Some(i) == self.selected_node {
                            [218, 218, 218, 255]
                        } else {
                            [65, 65, 65, 255]
                        };

                        ctx.draw.rounded_rect_with_border(
                            buffer.pixels_mut(),
                            &rect,
                            width as usize,
                            &[128, 128, 128, 255],
                            &(
                                5.0 * self.zoom,
                                5.0 * self.zoom,
                                5.0 * self.zoom,
                                5.0 * self.zoom,
                            ),
                            &border_color,
                            1.5 * zoom,
                        );

                        if i >= self.node_previews.len() {
                            self.node_previews.resize(i + 1, None);
                        }

                        // Remove preview buffer if size has changed
                        if let Some(preview_buffer) = &self.node_previews[i] {
                            if preview_buffer.dim().width != preview_size_scaled
                                && preview_buffer.dim().height != preview_size_scaled
                            {
                                self.node_previews[i] = None;
                            }
                        }

                        // Create preview if it doesn't exist
                        if self.node_previews[i].is_none() {
                            let mut preview_buffer = TheRGBABuffer::new(TheDim::sized(
                                preview_size_scaled,
                                preview_size_scaled,
                            ));
                            self.render_node_preview(&mut preview_buffer, i, palette);
                            self.node_previews[i] = Some(preview_buffer);
                        }

                        // Copy preview
                        if let Some(preview_buffer) = &self.node_previews[i] {
                            let preview_rect = (
                                rect.0 + preview_border_scaled as usize,
                                rect.1 + preview_border_scaled as usize,
                                preview_size_scaled as usize,
                                preview_size_scaled as usize,
                            );
                            ctx.draw.copy_slice(
                                buffer.pixels_mut(),
                                preview_buffer.pixels(),
                                &preview_rect,
                                width as usize,
                            );
                        }

                        // Input Terminals

                        let terminals = node.input_terminals();
                        let terminal_size = scaled(10);
                        let trf = scaled(2) as f32;
                        for (j, terminal) in terminals.iter().enumerate() {
                            let is_in_use = self.terminal_is_in_use(i, j, false);
                            let terminal_color = terminal.color.color().to_u8_array();

                            terminal_colors
                                .insert((i as i32, j as i32, 0), terminal.color.color().clone());

                            let terminal_x = rect.0 - terminal_size / 2 + scaled(1);
                            let terminal_y = rect.1 + scaled(8) + scaled(15) * j;
                            let terminal_rect =
                                (terminal_x, terminal_y, terminal_size, terminal_size);
                            ctx.draw.rounded_rect_with_border(
                                buffer.pixels_mut(),
                                &terminal_rect,
                                width as usize,
                                if is_in_use {
                                    &terminal_color
                                } else {
                                    &[128, 128, 128, 255]
                                },
                                &(trf, trf, trf, trf),
                                &terminal_color,
                                1.5 * zoom,
                            );

                            self.terminal_rects
                                .push((vec3i(i as i32, j as i32, 0), terminal_rect));
                        }

                        // Output Terminals

                        let terminals = node.output_terminals();
                        let terminal_size = scaled(10);
                        let trf = scaled(2) as f32;
                        for (j, terminal) in terminals.iter().enumerate() {
                            let is_in_use = self.terminal_is_in_use(i, j, true);
                            let terminal_color = terminal.color.color().to_u8_array();

                            terminal_colors
                                .insert((i as i32, j as i32, 1), terminal.color.color().clone());

                            let terminal_x = rect.0 + rect.2 - terminal_size / 2 - scaled(1);
                            let terminal_y = rect.1 + scaled(8) + scaled(15) * j;
                            let terminal_rect =
                                (terminal_x, terminal_y, terminal_size, terminal_size);
                            ctx.draw.rounded_rect_with_border(
                                buffer.pixels_mut(),
                                &terminal_rect,
                                width as usize,
                                if is_in_use {
                                    &terminal_color
                                } else {
                                    &[128, 128, 128, 255]
                                },
                                &(trf, trf, trf, trf),
                                &terminal_color,
                                1.5 * zoom,
                            );

                            self.terminal_rects
                                .push((vec3i(i as i32, j as i32, 1), terminal_rect));
                        }

                        self.node_rects.push(rect);
                    }
                }

                for c in self.connections.iter() {
                    if let Some(from_rect) =
                        self.get_terminal_rect(vec3i(c.0 as i32, c.1 as i32, 1))
                    {
                        let mut from_color = TheColor::white();
                        if let Some(color) = terminal_colors.get(&(c.0 as i32, c.1 as i32, 1)) {
                            from_color = color.clone();
                        }

                        if let Some(to_rect) =
                            self.get_terminal_rect(vec3i(c.2 as i32, c.3 as i32, 0))
                        {
                            let mut to_color = TheColor::white();
                            if let Some(color) = terminal_colors.get(&(c.2 as i32, c.3 as i32, 0)) {
                                to_color = color.clone();
                            }

                            let from_center = (
                                (from_rect.0 + from_rect.2 / 2) as i32,
                                (from_rect.1 + from_rect.3 / 2) as i32,
                            );
                            let to_center = (
                                (to_rect.0 + to_rect.2 / 2) as i32,
                                (to_rect.1 + to_rect.3 / 2) as i32,
                            );

                            let line: Vec<(i32, i32)> =
                                Bresenham::new(from_center, to_center).collect();
                            for (index, (x, y)) in line.iter().enumerate() {
                                let color =
                                    from_color.mix(&to_color, index as f32 / line.len() as f32);
                                buffer.set_pixel(*x, *y, &color.to_u8_array());
                            }
                        }
                    }
                }

                if let ModelFXNodeAction::ConnectingTerminal(_) = self.action {
                    buffer.draw_line(
                        self.drag_start.x,
                        self.drag_start.y,
                        self.drag_offset.x,
                        self.drag_offset.y,
                        WHITE,
                    )
                } else if let ModelFXNodeAction::CutConnection = self.action {
                    buffer.draw_line(
                        self.drag_start.x,
                        self.drag_start.y,
                        self.drag_offset.x,
                        self.drag_offset.y,
                        [209, 42, 42, 255],
                    )
                }

                node_view.set_buffer(buffer);
            }
            node_layout.relayout(ctx);
        }
    }

    pub fn clicked(&mut self, coord: Vec2i, _ui: &mut TheUI, _ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        if let Some(terminal) = self.get_terminal_at(coord) {
            self.drag_start = coord;
            self.drag_offset = coord;
            if let Some(rect) = self.get_terminal_rect(terminal) {
                self.drag_start = vec2i((rect.0 + rect.2 / 2) as i32, (rect.1 + rect.3 / 2) as i32);
                self.drag_offset = self.drag_start;
            }
            self.action = ModelFXNodeAction::ConnectingTerminal(terminal);
            redraw = true;
        } else if let Some(index) = self.get_node_at(coord) {
            self.drag_start = coord;
            self.action = ModelFXNodeAction::DragNode;
            if Some(index) != self.selected_node {
                self.selected_node = Some(index);
                redraw = true;
            }
        } else {
            self.action = ModelFXNodeAction::CutConnection;
            self.drag_start = coord;
        }
        redraw
    }
    pub fn dragged(&mut self, coord: Vec2i, _ui: &mut TheUI, _ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        if let ModelFXNodeAction::CutConnection = self.action {
            self.drag_offset = coord;
            redraw = true;
        } else if let ModelFXNodeAction::ConnectingTerminal(_) = self.action {
            self.drag_offset = coord;
            redraw = true;
        } else if self.action == ModelFXNodeAction::DragNode {
            if let Some(index) = self.selected_node {
                let collection = self.nodes[index].collection_mut();
                if let Some(TheValue::Int2(value)) = collection.get("_pos") {
                    let mut v = *value;
                    v.x += coord.x - self.drag_start.x;
                    v.y += coord.y - self.drag_start.y;
                    v.x = v.x.max(10);
                    v.y = v.y.max(10);
                    collection.set("_pos", TheValue::Int2(v));
                    self.drag_start = coord;
                    redraw = true;
                }
            }
        }
        redraw
    }
    pub fn released(&mut self, _ui: &mut TheUI, _ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        if let ModelFXNodeAction::CutConnection = self.action {
            redraw = true;
            let mut new_connections = vec![];
            for c in self.connections.iter() {
                if let Some(from_rect) = self.get_terminal_rect(vec3i(c.0 as i32, c.1 as i32, 1)) {
                    if let Some(to_rect) = self.get_terminal_rect(vec3i(c.2 as i32, c.3 as i32, 0))
                    {
                        let from_center = (
                            (from_rect.0 + from_rect.2 / 2) as i32,
                            (from_rect.1 + from_rect.3 / 2) as i32,
                        );
                        let to_center = (
                            (to_rect.0 + to_rect.2 / 2) as i32,
                            (to_rect.1 + to_rect.3 / 2) as i32,
                        );

                        let cut_start = (self.drag_start.x, self.drag_start.y);
                        let cut_end = (self.drag_offset.x, self.drag_offset.y);

                        if !do_intersect(from_center, to_center, cut_start, cut_end) {
                            new_connections.push(*c);
                        }
                    }
                }
            }
            self.connections = new_connections;
        } else if let ModelFXNodeAction::ConnectingTerminal(source) = self.action {
            if let Some(dest) = self.get_terminal_at(self.drag_offset) {
                if source.x != dest.x && source.z != dest.z {
                    // Make sure output terminal is always listed first
                    if source.z == 0 {
                        // Dest is output terminal
                        self.connections.push((
                            dest.x as u16,
                            dest.y as u8,
                            source.x as u16,
                            source.y as u8,
                        ));
                    } else {
                        // Source it output terminal
                        self.connections.push((
                            source.x as u16,
                            source.y as u8,
                            dest.x as u16,
                            dest.y as u8,
                        ));
                    }
                }
            }
            redraw = true;
        }
        self.action = ModelFXNodeAction::None;
        redraw
    }
    pub fn hovered(&mut self, coord: Vec2i, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        if let Some(node_layout) = ui.get_rgba_layout("ModelFX RGBA Layout") {
            if let Some(node_view) = node_layout.rgba_view_mut().as_rgba_view() {
                if let Some(index) = self.get_node_at(coord) {
                    ctx.ui.send(TheEvent::SetStatusText(
                        node_view.id().clone(),
                        self.nodes[index].name(),
                    ));
                } else {
                    ctx.ui
                        .send(TheEvent::SetStatusText(node_view.id().clone(), str!("")));
                }
            }
        }
        false
    }

    /// Get the node index at the given coordinate.
    pub fn get_node_at(&self, coord: Vec2i) -> Option<usize> {
        for (i, rect) in self.node_rects.iter().enumerate().rev() {
            if rect.0 as i32 <= coord.x
                && coord.x <= rect.0 as i32 + rect.2 as i32
                && rect.1 as i32 <= coord.y
                && coord.y <= rect.1 as i32 + rect.3 as i32
            {
                return Some(i);
            }
        }
        None
    }

    /// Get the terminal index at the given coordinate.
    pub fn get_terminal_at(&self, coord: Vec2i) -> Option<Vec3i> {
        for (terminal, rect) in self.terminal_rects.iter().rev() {
            if rect.0 as i32 <= coord.x
                && coord.x <= rect.0 as i32 + rect.2 as i32
                && rect.1 as i32 <= coord.y
                && coord.y <= rect.1 as i32 + rect.3 as i32
            {
                return Some(*terminal);
            }
        }
        None
    }

    /// Get the terminal rect for the given terminal.
    pub fn get_terminal_rect(&self, terminal: Vec3i) -> Option<(usize, usize, usize, usize)> {
        for (t, rect) in self.terminal_rects.iter() {
            if *t == terminal {
                return Some(*rect);
            }
        }
        None
    }

    /// Returns true if the given terminal is in use.
    pub fn terminal_is_in_use(&self, node: usize, terminal_index: usize, output: bool) -> bool {
        if output {
            for (o, ot, _, _) in &self.connections {
                if *o == node as u16 && *ot == terminal_index as u8 {
                    return true;
                }
            }
        } else {
            for (_, _, i, it) in &self.connections {
                if *i == node as u16 && *it == terminal_index as u8 {
                    return true;
                }
            }
        }
        false
    }

    /// Returns the connected output node for the given input node and terminal.
    pub fn find_connected_output_node(&self, node: usize, terminal_index: usize) -> Option<usize> {
        for (o, _, i, it) in &self.connections {
            if *i == node as u16 && *it == terminal_index as u8 {
                return Some(*o as usize);
            }
        }
        None
    }

    /// After exiting a geometry node follow the trail of material nodes to calculate the final color.
    pub fn follow_trail(
        &self,
        node: usize,
        terminal_index: usize,
        hit: &mut Hit,
        palette: &ThePalette,
    ) {
        let mut connections = vec![];

        for (o, ot, i, it) in &self.connections {
            if *o == node as u16 && *ot == terminal_index as u8 {
                connections.push((*i, *it));
            }
        }

        match connections.len() {
            0 => {}
            1 => {
                let o = connections[0].0 as usize;

                let mut noise = 1.0;
                if let Some(noise_index) = self.find_connected_output_node(o, 1) {
                    if let ModelFXNode::Noise3D(_coll) = &self.nodes[noise_index] {
                        noise = self.nodes[noise_index].noise(hit);
                        hit.uv += 7.23;
                        let noise2 = self.nodes[noise_index].noise(hit);
                        let wobble = vec2f(noise, noise2);
                        hit.uv -= 7.23;
                        hit.uv += wobble * 0.5;
                    }
                }

                if let Some(ot) = self.nodes[o].material(&connections[0].1, hit, palette, noise) {
                    self.follow_trail(o, ot as usize, hit, palette);
                }
            }
            _ => {
                let index = (hit.hash * connections.len() as f32).floor() as usize;
                if let Some(random_connection) = connections.get(index) {
                    let o = random_connection.0 as usize;
                    let mut noise = 1.0;
                    if let Some(noise_index) = self.find_connected_output_node(o, 1) {
                        if let ModelFXNode::Noise3D(_coll) = &self.nodes[noise_index] {
                            noise = self.nodes[noise_index].noise(hit);
                        }
                    }
                    if let Some(ot) =
                        self.nodes[o].material(&random_connection.1, hit, palette, noise)
                    {
                        self.follow_trail(o, ot as usize, hit, palette);
                    }
                }
            }
        }
    }

    /// Create the voxels for the model in the given density.
    pub fn create_voxels(&mut self, density: u8, key: &Vec3f, palette: &ThePalette) {
        self.density = density;
        self.voxels.clear();

        let density = density as i32;
        let density_f = density as f32;

        for z in 0..density {
            for y in 0..density {
                for x in 0..density {
                    let p = vec3f(
                        x as f32 / density_f,
                        y as f32 / density_f,
                        z as f32 / density_f,
                    );
                    let mut hit = Hit::default();
                    let d = self.distance_hit(p, &mut hit);

                    if d < 0.05 {
                        hit.normal = self.normal(p);
                        hit.hit_point = p + key;
                        hit.key = *key;

                        let terminal_index = self.nodes[hit.node].color_index_for_hit(&mut hit).1;
                        self.follow_trail(hit.node, terminal_index as usize, &mut hit, palette);

                        let c = TheColor::from_vec4f(hit.color).to_u8_array();

                        let voxel = Voxel {
                            color: [c[0], c[1], c[2]],
                        };

                        self.voxels.insert((x as u8, y as u8, z as u8), voxel);
                    }
                }
            }
        }
    }

    pub fn render(
        &self,
        ray: &Ray,
        max_distance: f32,
        key: Vec3f,
        palette: &ThePalette,
    ) -> Option<Hit> {
        let max_t = max_distance * 1.732;
        let mut t = 0.0;

        let mut hit = Hit::default();
        let mut p = ray.at(t);

        while t < max_t {
            let d = self.distance_hit(p, &mut hit);
            if d < 0.001 {
                hit.distance = t;
                break;
            }
            t += d;
            p = ray.at(t);
        }

        if t < max_t {
            hit.normal = self.normal(p);
            hit.hit_point = p + key;

            let c = dot(hit.normal, normalize(vec3f(1.0, 2.0, 3.0))) * 0.5 + 0.5;
            hit.color = vec4f(c, c, c, 1.0);

            let terminal_index = self.nodes[hit.node].color_index_for_hit(&mut hit).1;
            self.follow_trail(hit.node, terminal_index as usize, &mut hit, palette);

            Some(hit)
        } else {
            None
        }
    }

    pub fn render_preview(&mut self, buffer: &mut TheRGBABuffer, palette: &ThePalette) {
        //}, palette: &ThePalette) {
        let width = buffer.dim().width as usize;
        let height = buffer.dim().height as usize;

        let ro = vec3f(2.0, 2.0, 2.0);
        let rd = vec3f(0.0, 0.0, 0.0);

        let aa = 2;
        let aa_f = aa as f32;

        let camera = Camera::new(ro, rd, 160.0);
        let bgc = 74.0 / 255.0;

        buffer
            .pixels_mut()
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let xx = (i % width) as f32;
                    let yy = (i / width) as f32;

                    let mut total = Vec4f::zero();

                    for m in 0..aa {
                        for n in 0..aa {
                            let camera_offset =
                                vec2f(m as f32 / aa_f, n as f32 / aa_f) - vec2f(0.5, 0.5);

                            let mut color = vec4f(bgc, bgc, bgc, 1.0);

                            let ray = camera.create_ortho_ray(
                                vec2f(xx / width as f32, 1.0 - yy / height as f32),
                                vec2f(width as f32, height as f32),
                                camera_offset,
                            );

                            if let Some(hit) = self.render(&ray, 3.0, Vec3f::zero(), palette) {
                                color = hit.color;
                            }

                            total += color;
                        }
                    }

                    let aa_aa = aa_f * aa_f;
                    total[0] /= aa_aa;
                    total[1] /= aa_aa;
                    total[2] /= aa_aa;
                    total[3] /= aa_aa;

                    pixel.copy_from_slice(&TheColor::from_vec4f(total).to_u8_array());
                }
            });
    }

    pub fn render_node_preview(
        &self,
        buffer: &mut TheRGBABuffer,
        node_index: usize,
        palette: &ThePalette,
    ) {
        let width = buffer.dim().width as usize;
        let height = buffer.dim().height as usize;

        let ro = vec3f(2.0, 2.0, 2.0);
        let rd = vec3f(0.0, 0.0, 0.0);

        let aa = 2;
        let aa_f = aa as f32;

        let camera = Camera::new(ro, rd, 160.0);
        let bgc = 128.0 / 255.0;

        buffer
            .pixels_mut()
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let xx = (i % width) as f32;
                    let yy = (i / width) as f32;

                    let mut total = Vec4f::zero();

                    let role = self.nodes[node_index].role();
                    if role == ModelFXNodeRole::Geometry {
                        for m in 0..aa {
                            for n in 0..aa {
                                let camera_offset =
                                    vec2f(m as f32 / aa_f, n as f32 / aa_f) - vec2f(0.5, 0.5);

                                let mut color = vec4f(bgc, bgc, bgc, 1.0);

                                let ray = camera.create_ortho_ray(
                                    vec2f(xx / width as f32, 1.0 - yy / height as f32),
                                    vec2f(width as f32, height as f32),
                                    camera_offset,
                                );

                                let max_t = 3.0 * 1.732;
                                let mut t = 0.0;

                                let mut p = ray.at(t);

                                while t < max_t {
                                    let d = self.nodes[node_index].distance(p);
                                    if d < 0.001 {
                                        break;
                                    }
                                    t += d;
                                    p = ray.at(t);
                                }

                                if t < max_t {
                                    let mut hit = Hit {
                                        uv: vec2f(xx / width as f32, yy / height as f32),
                                        normal: self.normal_node(p, &self.nodes[node_index]),
                                        ..Default::default()
                                    };
                                    let c = ModelFXColor::create(
                                        self.nodes[node_index].color_index_for_hit(&mut hit).0,
                                    );
                                    color = c.color().to_vec4f();
                                }

                                total += color;
                            }
                        }

                        let aa_aa = aa_f * aa_f;
                        total[0] /= aa_aa;
                        total[1] /= aa_aa;
                        total[2] /= aa_aa;
                        total[3] /= aa_aa;
                    } else if role == ModelFXNodeRole::Material {
                        // Material node

                        let mut noise = 1.0;
                        let mut wobble = Vec2f::zero();

                        if let Some(noise_index) = self.find_connected_output_node(node_index, 1) {
                            if let ModelFXNode::Noise3D(_coll) = &self.nodes[noise_index] {
                                let uv = vec2f(xx / width as f32, yy / height as f32);
                                let mut hit = Hit {
                                    uv: uv * 3.0,
                                    hit_point: vec3f(uv.x, 0.0, uv.y) * 3.0,
                                    ..Default::default()
                                };
                                noise = self.nodes[noise_index].noise(&hit);
                                hit.uv += 7.23;
                                let noise2 = self.nodes[noise_index].noise(&hit);
                                wobble = vec2f(noise, noise2);
                            }
                        }

                        let mut hit = Hit {
                            uv: vec2f(xx / width as f32, yy / height as f32) * 3.0,
                            ..Default::default()
                        };
                        hit.uv += wobble * 0.5;
                        if let ModelFXNode::Material(_coll) = &self.nodes[node_index] {
                            self.nodes[node_index].material(&0, &mut hit, palette, noise);
                            total = hit.color;
                        } else {
                            let c = ModelFXColor::create(
                                self.nodes[node_index].color_index_for_hit(&mut hit).0,
                            );
                            total = c.color().to_vec4f();
                        }
                    } else if role == ModelFXNodeRole::Noise {
                        // Noise node
                        let uv = vec2f(xx / width as f32, yy / height as f32);
                        let hit = Hit {
                            uv: uv * 3.0,
                            hit_point: vec3f(uv.x, 0.0, uv.y) * 3.0,
                            ..Default::default()
                        };
                        if let ModelFXNode::Noise3D(_coll) = &self.nodes[node_index] {
                            let n = self.nodes[node_index].noise(&hit);
                            total = vec4f(n, n, n, 1.0);
                        }
                    }

                    pixel.copy_from_slice(&TheColor::from_vec4f(total).to_u8_array());
                }
            });
    }

    /// Get the distance at the given position for all nodes and save the closest node in the hit structure.
    #[inline(always)]
    pub fn distance_hit(&self, p: Vec3f, hit: &mut Hit) -> f32 {
        let mut d = f32::MAX;
        for (index, node) in self.nodes.iter().enumerate() {
            if node.role() == ModelFXNodeRole::Geometry {
                let dist = node.distance(p);
                if dist < d {
                    d = dist;
                    hit.node = index;
                }
            }
        }
        d
    }

    /// Get the distance at the given position for all geometry nodes.
    #[inline(always)]
    pub fn distance(&self, p: Vec3f) -> f32 {
        let mut d = f32::MAX;
        for node in self.nodes.iter() {
            if node.role() == ModelFXNodeRole::Geometry {
                d = d.min(node.distance(p));
            }
        }
        d
    }

    pub fn normal(&self, p: Vec3f) -> Vec3f {
        let scale = 0.5773 * 0.0005;
        let e = vec2f(1.0 * scale, -1.0 * scale);

        // IQs normal function

        let e1 = vec3f(e.x, e.y, e.y);
        let e2 = vec3f(e.y, e.y, e.x);
        let e3 = vec3f(e.y, e.x, e.y);
        let e4 = vec3f(e.x, e.x, e.x);

        let n = e1 * self.distance(p + e1)
            + e2 * self.distance(p + e2)
            + e3 * self.distance(p + e3)
            + e4 * self.distance(p + e4);
        normalize(n)
    }

    pub fn normal_node(&self, p: Vec3f, node: &ModelFXNode) -> Vec3f {
        let scale = 0.5773 * 0.0005;
        let e = vec2f(1.0 * scale, -1.0 * scale);

        // IQs normal function

        let e1 = vec3f(e.x, e.y, e.y);
        let e2 = vec3f(e.y, e.y, e.x);
        let e3 = vec3f(e.y, e.x, e.y);
        let e4 = vec3f(e.x, e.x, e.x);

        let n = e1 * node.distance(p + e1)
            + e2 * node.distance(p + e2)
            + e3 * node.distance(p + e3)
            + e4 * node.distance(p + e4);
        normalize(n)
    }

    /// Load a model from a JSON string.
    pub fn from_json(json: &str) -> Self {
        let mut modelfx: ModelFX = serde_json::from_str(json).unwrap_or_default();
        let cnt = modelfx.nodes.len();
        for _ in 0..cnt {
            modelfx.node_previews.push(None);
        }
        modelfx
    }

    /// Convert the model to a JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}
