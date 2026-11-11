use super::*;
use crate::{
    paint::{LineCap, LineJoin, PaintOp, Path, PathElement, Rect, Sampling, Stroke as PaintStroke},
    theui::paint::{UiPaintResult, WidgetPainter, bounds, rgba},
};
impl TheNodeCanvasView {
    pub(super) fn prepare_commands(&mut self) {
        // Retain only interaction geometry. Source previews are explicit assets;
        // neither the viewport nor any individual node gets a raster cache.
        self.render_buffer = TheRGBABuffer::empty();
        self.node_rects.clear();
        self.terminal_rects.clear();
        let width = self.canvas.node_width;
        for node in &self.canvas.nodes {
            let terminals = node.inputs.len().max(node.outputs.len()) as i32;
            let body = 14 + terminals * 10 + (terminals - 1).max(0) * 4;
            let preview = if node.supports_preview && node.preview_is_open {
                118
            } else {
                0
            };
            self.node_rects.push(TheDim::new(
                node.position.x - self.canvas.offset.x,
                node.position.y - self.canvas.offset.y,
                width,
                38 + body + preview,
            ));
            let inputs = (0..node.inputs.len())
                .map(|i| TheDim::new(7, 26 + i as i32 * 14, 10, 10))
                .collect();
            let outputs = (0..node.outputs.len())
                .map(|i| TheDim::new(width - 17, 26 + i as i32 * 14, 10, 10))
                .collect();
            self.terminal_rects.push((inputs, outputs));
        }
    }
    pub(super) fn paint_commands(&mut self, p: &mut WidgetPainter<'_>) -> UiPaintResult {
        let r = bounds(&self.dim);
        p.list.ops.push(PaintOp::PushClip(r));
        p.fill(r, NodeCanvasBackground);
        let mut x = (-self.canvas.offset.x).rem_euclid(40) as f32;
        while x < r.width {
            p.fill(Rect::new(r.x + x, r.y, 1., r.height), NodeCanvasGrid);
            x += 40.;
        }
        let mut y = (-self.canvas.offset.y).rem_euclid(40) as f32;
        while y < r.height {
            p.fill(Rect::new(r.x, r.y + y, r.width, 1.), NodeCanvasGrid);
            y += 40.;
        }
        if let Some(overlay) = self.overlay.as_ref().filter(|b| b.is_valid()) {
            let image = p.images.image(
                &format!("node-overlay:{}", self.id.uuid),
                overlay,
                self.overlay_dirty,
            )?;
            self.overlay_dirty = false;
            if self.overlay_tiled {
                let w = overlay.dim().width as f32;
                let h = overlay.dim().height as f32;
                p.list.ops.push(PaintOp::Image {
                    image,
                    rect: r,
                    uv: Rect::new(
                        self.canvas.offset.x as f32 / w,
                        self.canvas.offset.y as f32 / h,
                        r.width / w,
                        r.height / h,
                    ),
                    sampling: Sampling::RepeatNearest,
                    tint: [1.; 4],
                });
            } else {
                p.list.image(
                    image,
                    Rect::new(
                        r.x + 12.,
                        r.y + (r.height - overlay.dim().height as f32 - 12.).max(12.),
                        overlay.dim().width as f32,
                        overlay.dim().height as f32,
                    ),
                    Sampling::Nearest,
                );
            }
        }
        p.list
            .ops
            .push(PaintOp::PushTransform(crate::paint::Transform::translate(
                r.x, r.y,
            )));
        for index in 0..self.canvas.nodes.len() {
            if Some(index) != self.canvas.selected_node {
                self.paint_node(index, p, r.width, r.height)?;
            }
        }
        if let Some(index) = self
            .canvas
            .selected_node
            .filter(|i| *i < self.canvas.nodes.len())
        {
            self.paint_node(index, p, r.width, r.height)?;
        }
        let mut connections = Path::default();
        for &(source, output, dest, input) in &self.canvas.connections {
            let (Some(src_node), Some(dst_node)) = (
                self.canvas.nodes.get(source as usize),
                self.canvas.nodes.get(dest as usize),
            ) else {
                continue;
            };
            let (Some(a), Some(b)) = (
                self.terminal_rect_for(source as usize, true, output),
                self.terminal_rect_for(dest as usize, false, input),
            ) else {
                continue;
            };
            let a = [
                (src_node.position.x + a.x + 16 - self.canvas.offset.x) as f32,
                (src_node.position.y + a.y + a.height / 2 - self.canvas.offset.y) as f32,
            ];
            let b = [
                (dst_node.position.x + b.x - 6 - self.canvas.offset.x) as f32,
                (dst_node.position.y + b.y + b.height / 2 - self.canvas.offset.y) as f32,
            ];
            let distance = (a[0] - b[0]).hypot(a[1] - b[1]).min(50.);
            connections.elements.extend([
                PathElement::Move(a),
                PathElement::Cubic([a[0] + distance, a[1]], [b[0] - distance, b[1]], b),
            ]);
        }
        if let TheNodeAction::ConnectingTerminal(_, _, _) = self.action {
            connections.elements.extend([
                PathElement::Move([
                    (self.drag_start.x - self.canvas.offset.x) as f32,
                    (self.drag_start.y - self.canvas.offset.y) as f32,
                ]),
                PathElement::Line([self.drag_offset.x as f32, self.drag_offset.y as f32]),
            ]);
        }
        if !connections.elements.is_empty() {
            p.list.ops.push(PaintOp::Stroke {
                path: Arc::new(connections),
                stroke: PaintStroke {
                    width: 2.6,
                    cap: LineCap::Butt,
                    join: LineJoin::Round,
                    brush: p.color(NodeConnection).into(),
                },
            });
        }
        if self.action == TheNodeAction::CutConnection {
            p.list.ops.push(PaintOp::Stroke {
                path: Arc::new(Path {
                    elements: vec![
                        PathElement::Move([self.drag_start.x as f32, self.drag_start.y as f32]),
                        PathElement::Line([self.drag_offset.x as f32, self.drag_offset.y as f32]),
                    ],
                }),
                stroke: PaintStroke {
                    width: 1.,
                    cap: LineCap::Butt,
                    join: LineJoin::Miter,
                    brush: p.color(NodeCutConnection).into(),
                },
            });
        }
        p.list.ops.push(PaintOp::PopTransform);
        if Some(&self.id) == p.ui.focus.as_ref() {
            p.outline(r, *p.theme.color(DefaultSelection), 1.);
        }
        p.list.ops.push(PaintOp::PopClip);
        self.is_dirty = false;
        Ok(())
    }
    fn paint_node(
        &mut self,
        index: usize,
        p: &mut WidgetPainter<'_>,
        width: f32,
        height: f32,
    ) -> UiPaintResult {
        let node = &self.canvas.nodes[index];
        let r = bounds(&self.node_rects[index]);
        if r.intersection(Rect::new(0., 0., width, height)).width <= 0.
            || r.intersection(Rect::new(0., 0., width, height)).height <= 0.
        {
            return Ok(());
        }
        let selected = self.canvas.selected_node == Some(index);
        let (header, body, footer, border) = if selected {
            (
                NodeHeaderSelectedChrome,
                NodeBodySelectedChrome,
                NodeFooterSelectedChrome,
                NodeBorderSelected,
            )
        } else {
            (
                NodeHeaderNormalChrome,
                NodeBodyNormalChrome,
                NodeFooterNormalChrome,
                NodeBorder,
            )
        };
        p.list.rect(r, 4., p.color(border));
        p.theme_fill(r.inset(1.), 3., body);
        let head = Rect::new(r.x + 1., r.y + 1., (r.width - 2.).max(0.), 18.);
        let brush = p.theme_brush(head, header);
        p.list.rect(head, 3., brush.clone());
        p.list
            .rect(Rect::new(head.x, head.y + 8., head.width, 10.), 0., brush);
        p.fill(
            Rect::new(r.x + 1., r.y + 18., (r.width - 2.).max(0.), 1.),
            border,
        );
        let foot = Rect::new(r.x + 1., r.y + r.height - 19., (r.width - 2.).max(0.), 18.);
        let brush = p.theme_brush(foot, footer);
        p.list.rect(foot, 3., brush.clone());
        p.list
            .rect(Rect::new(foot.x, foot.y, foot.width, 10.), 0., brush);
        p.fill(Rect::new(foot.x, foot.y, foot.width, 1.), border);
        let title_color = node
            .outputs
            .first()
            .or_else(|| node.inputs.first())
            .map(|t| self.color_for(&t.category_name).to_u8_array())
            .unwrap_or(*p.theme.color(ListItemText));
        p.label(
            Rect::new(r.x + 12., r.y + 4., (r.width - 24.).max(0.), 13.),
            &node.name,
            10.,
            title_color,
            TheHorizontalAlign::Left,
            TheVerticalAlign::Top,
        );
        for output in [false, true] {
            let terminals = if output { &node.outputs } else { &node.inputs };
            let rects = if output {
                &self.terminal_rects[index].1
            } else {
                &self.terminal_rects[index].0
            };
            for (terminal, rect) in terminals.iter().zip(rects) {
                let t = Rect::new(r.x + rect.x as f32, r.y + rect.y as f32, 10., 10.);
                p.list.rect(t, 5., p.color(NodeBorder));
                p.list.rect(
                    t.inset(1.),
                    4.,
                    rgba(self.color_for(&terminal.category_name).to_u8_array()),
                );
                p.label(
                    Rect::new(if output { t.x - 82. } else { t.x + 12. }, t.y, 80., 10.),
                    &terminal.name,
                    9.5,
                    *p.theme.color(ListItemText),
                    if output {
                        TheHorizontalAlign::Right
                    } else {
                        TheHorizontalAlign::Left
                    },
                    TheVerticalAlign::Center,
                );
            }
        }
        if node.supports_preview && node.preview_is_open {
            let preview = Rect::new(r.x + 3., foot.y - 116., (r.width - 6.).max(0.), 114.);
            p.list.rect(preview, 2., p.color(border));
            p.theme_fill(preview.inset(1.), 1., NodePreviewBackground);
            let cpu = &node.preview;
            let image_size = self
                .preview_images
                .get(&index)
                .map(|(_, size)| [size.x as f32, size.y as f32])
                .or_else(|| {
                    cpu.is_valid()
                        .then_some([cpu.dim().width as f32, cpu.dim().height as f32])
                });
            if let Some([w, h]) = image_size.filter(|[w, h]| *w > 0. && *h > 0.) {
                let rect = Rect::new(
                    r.x + 3. + (r.width - 4. - w) * 0.5,
                    foot.y - 118. + (118. - h) * 0.5,
                    w,
                    h,
                );
                p.list.ops.push(PaintOp::PushClip(preview.inset(1.)));
                if let Some((image, _)) = self.preview_images.get(&index) {
                    p.list.image(*image, rect, Sampling::Linear);
                } else {
                    p.image(
                        &format!("node-preview:{}:{index}", self.id.uuid),
                        cpu,
                        rect,
                        self.preview_dirty.contains(&index),
                        Sampling::Linear,
                    )?;
                    self.preview_dirty.remove(&index);
                }
                p.list.ops.push(PaintOp::PopClip);
            }
        }
        Ok(())
    }
}
