use super::*;
use crate::{
    paint::{PaintOp, PaintVertex, Rect},
    theui::paint::{UiPaintResult, WidgetPainter, bounds, rgba},
};
impl TheColorPicker {
    pub(super) fn paint_commands(&mut self, p: &mut WidgetPainter<'_>) -> UiPaintResult {
        let r = bounds(&self.dim);
        p.list.ops.push(PaintOp::PushClip(r));
        if let Some(background) = self.background {
            p.list.rect(r, 0., rgba(background));
        }
        let size = r.width.min(r.height);
        let center = [r.x + size * 0.5, r.y + size * 0.5];
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        // Geometric tessellation, independent of viewport pixels or device density.
        // Each hue interval is linear in sRGB, matching the color model/controller.
        const SEGMENTS: usize = 192;
        for step in 0..=SEGMENTS {
            let hue = step as f32 / SEGMENTS as f32 * 360.;
            let angle = (hue - 180.).to_radians();
            let color = TheColor::from_hsl(hue, 1., 0.5).to_vec3();
            for radius in [size * 0.375, size * 0.5] {
                vertices.push(PaintVertex {
                    position: [
                        center[0] + angle.sin() * radius,
                        center[1] + angle.cos() * radius,
                    ],
                    color: [color.x, color.y, color.z, 1.],
                });
            }
            if step < SEGMENTS {
                let i = step as u32 * 2;
                indices.extend([i, i + 1, i + 2, i + 2, i + 1, i + 3]);
            }
        }
        let hue = TheColor::from_hsl(self.h, 1., 0.5).to_vec3();
        let first = vertices.len() as u32;
        for (point, color) in [
            (self.ls1_pt, [0., 0., 0., 1.]),
            (self.ls2_pt, [hue.x, hue.y, hue.z, 1.]),
            (self.ls3_pt, [1.; 4]),
        ] {
            vertices.push(PaintVertex {
                position: [r.x + point.x + self.dot_size, r.y + point.y + self.dot_size],
                color,
            });
        }
        indices.extend([first, first + 1, first + 2]);
        p.list.ops.push(PaintOp::Mesh {
            vertices: vertices.into(),
            indices: indices.into(),
        });
        let radius = self.dot_size * 0.8;
        for dot in [self.hue_dot, self.sl_dot] {
            let x = r.x + dot.x + self.dot_size;
            let y = r.y + dot.y + self.dot_size;
            p.list.rect(
                Rect::new(x - radius, y - radius, radius * 2., radius * 2.),
                radius,
                [0., 0., 0., 1.],
            );
        }
        if let Some(border) = self.border {
            p.outline(r, border, 1.);
        }
        p.list.ops.push(PaintOp::PopClip);
        self.is_dirty = false;
        Ok(())
    }
}
