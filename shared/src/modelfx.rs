use crate::prelude::*;
use rayon::prelude::*;
use theframework::prelude::*;

//const RED: RGBA = [209, 42, 42, 255];
// const GREEN: RGBA = [10, 245, 5, 255];
//const YELLOW: RGBA = [238, 251, 28, 255];
//const BLUE: RGBA = [44, 52, 214, 255];
const RED: RGBA = [212, 128, 77, 255];
const YELLOW: RGBA = [224, 200, 114, 255];
//const PALE_YELLOW: RGBA = [217, 172, 139, 255];
const BLUE: RGBA = [36, 61, 92, 255];

// const COLOR1: [u8; 4] = [217, 172, 139, 255];
// const COLOR2: [u8; 4] = [62, 105, 88, 255];
// const COLOR3: [u8; 4] = [177, 165, 141, 255];
// const COLOR4: [u8; 4] = [98, 76, 60, 255];
// const COLOR5: [u8; 4] = [36, 61, 92, 255];
// const COLOR6: [u8; 4] = [224, 200, 114, 255];
// const COLOR7: [u8; 4] = [176, 58, 72, 255];
// const COLOR8: [u8; 4] = [212, 128, 77, 255];
// const COLOR9: [u8; 4] = [92, 139, 147, 255];
// const COLOR10: [u8; 4] = [227, 207, 180, 255];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ModelFXNodeAction {
    None,
    DragNode,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ModelFX {
    pub action: ModelFXNodeAction,

    pub nodes: Vec<ModelFXNode>,

    #[serde(skip)]
    pub node_rects: Vec<(usize, usize, usize, usize)>,
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
            node_rects: vec![],
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
                .set("_pos", TheValue::Int2(vec2i(10, 10)));
            self.selected_node = Some(self.nodes.len());
            self.nodes.push(node);
            return true;
        }
        false
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

    pub fn draw(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        let min_x = 0; //std::i32::MAX;
        let min_y = 0; //std::i32::MAX;
        let mut max_x = std::i32::MIN;
        let mut max_y = std::i32::MIN;

        let zoom = self.zoom;
        let node_size = 60;
        let node_size_scaled = (node_size as f32 * zoom) as i32;
        let preview_border_scaled = (4.0 * zoom) as i32;
        let preview_size_scaled = node_size_scaled - 2 * preview_border_scaled;

        let mut preview_buffer: Option<TheRGBABuffer> = None;

        if !self.nodes.is_empty() {
            for node in &self.nodes {
                if let Some(TheValue::Int2(v)) = node.collection().get("_pos") {
                    //min_x = min_x.min(v.x - 10);
                    //min_y = min_y.min(v.y - 10);
                    max_x = max_x.max(v.x + node_size_scaled + 10);
                    max_y = max_y.max(v.y + node_size_scaled + 10);
                }
            }

            let mut buffer = TheRGBABuffer::new(TheDim::sized(120, 120));
            self.render_preview(&mut buffer);
            preview_buffer = Some(buffer);
        }

        let mut width = (max_x - min_x).max(20);
        let mut height = (max_y - min_y).max(20);
        self.node_rects.clear();

        if let Some(node_layout) = ui.get_rgba_layout("ModelFX RGBA Layout") {
            if let Some(node_view) = node_layout.rgba_view_mut().as_rgba_view() {
                let dim = node_view.dim();

                width = width.max(dim.width - 13);
                height = height.max(dim.height - 13);

                let mut buffer = TheRGBABuffer::new(TheDim::sized(width, height));
                buffer.fill([74, 74, 74, 255]);

                if let Some(preview_buffer) = preview_buffer {
                    buffer.copy_into(
                        width - preview_buffer.dim().width,
                        0, //height - preview_buffer.dim().height,
                        &preview_buffer,
                    )
                }

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
                            1.5,
                        );

                        let mut preview_buffer = TheRGBABuffer::new(TheDim::sized(
                            preview_size_scaled,
                            preview_size_scaled,
                        ));

                        self.render_node_preview(&mut preview_buffer, node);
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

                        self.node_rects.push(rect);
                    }
                }

                node_view.set_buffer(buffer);
            }
            node_layout.relayout(ctx);
        }
    }

    pub fn clicked(&mut self, coord: Vec2i, _ui: &mut TheUI, _ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        if let Some(index) = self.get_node_at(coord) {
            self.drag_start = coord;
            self.action = ModelFXNodeAction::DragNode;
            if Some(index) != self.selected_node {
                self.selected_node = Some(index);
                redraw = true;
            }
        }
        redraw
    }
    pub fn dragged(&mut self, coord: Vec2i, _ui: &mut TheUI, _ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        if self.action == ModelFXNodeAction::DragNode {
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
    pub fn released(&mut self, _ui: &mut TheUI, _ctx: &mut TheContext) {}
    pub fn hovered(&mut self, coord: Vec2i, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        if let Some(node_layout) = ui.get_rgba_layout("ModelFX RGBA Layout") {
            if let Some(node_view) = node_layout.rgba_view_mut().as_rgba_view() {
                if let Some(index) = self.get_node_at(coord) {
                    ctx.ui.send(TheEvent::SetStatusText(
                        node_view.id().clone(),
                        self.nodes[index].to_kind(),
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
        for (i, rect) in self.node_rects.iter().enumerate() {
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

    pub fn render_preview(&mut self, buffer: &mut TheRGBABuffer) {
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

                            let max_t = 3.0 * 1.732;
                            let mut t = 0.0;

                            let mut p = ray.at(t);

                            while t < max_t {
                                let d = self.distance(p);
                                if d < 0.001 {
                                    break;
                                }
                                t += d;
                                p = ray.at(t);
                            }

                            if t < max_t {
                                let normal = self.normal(p);
                                let c = dot(normal, normalize(vec3f(1.0, 2.0, 3.0))) * 0.5 + 0.5;
                                color.x = c;
                                color.y = c;
                                color.z = c;
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

    pub fn render_node_preview(&self, buffer: &mut TheRGBABuffer, node: &ModelFXNode) {
        //}, palette: &ThePalette) {
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
                                let d = node.distance(p);
                                if d < 0.001 {
                                    break;
                                }
                                t += d;
                                p = ray.at(t);
                            }

                            if t < max_t {
                                let normal = self.normal_node(p, node);

                                let nx = normal.x.abs();
                                let ny = normal.y.abs();
                                let nz = normal.z.abs();

                                if nx > ny && nx > nz {
                                    // X-face
                                    color = TheColor::from_u8_array(RED).to_vec4f();
                                } else if ny > nx && ny > nz {
                                    // Y-face
                                    color = TheColor::from_u8_array(YELLOW).to_vec4f();
                                } else {
                                    // Z-face
                                    color = TheColor::from_u8_array(BLUE).to_vec4f();
                                }
                            }

                            /*
                            if let Some(hit) = hit {
                                if hit.face == HitFace::XFace {
                                    color = TheColor::from_u8_array(RED).to_vec4f();
                                }
                                if hit.face == HitFace::YFace {
                                    color = TheColor::from_u8_array(YELLOW).to_vec4f();
                                }
                                if hit.face == HitFace::ZFace {
                                    color = TheColor::from_u8_array(BLUE).to_vec4f();
                                }
                            }
                            */

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

    /// Get the distance at the given position for all nodes.
    #[inline(always)]
    pub fn distance(&self, p: Vec3f) -> f32 {
        let mut d = f32::MAX;
        for fx in self.nodes.iter() {
            d = d.min(fx.distance(p));
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
}
