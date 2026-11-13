//! Adapter preserving graph input/document semantics while emitting GPU-ready paint.
use super::*;
use crate::{
    paint::{self, PaintList, PaintOp, Rect},
    text::TextSystem,
};
use std::sync::Arc;
pub trait GraphImageHandles {
    fn image_handle(&self, key: &str) -> Option<paint::ImageHandle>;
    /// A preview may be immutable procedural paint rather than an image asset.
    /// The graph model and interaction contracts remain independent of it.
    fn paint_preview(&self, key: &str, rect: Rect, paint: &mut PaintList) {
        if let Some(image) = self.image_handle(key) {
            paint.image(image, rect, paint::Sampling::Linear);
        }
    }
}
impl GraphImageHandles for () {
    fn image_handle(&self, _: &str) -> Option<paint::ImageHandle> {
        None
    }
}
pub struct PaintGraphPainter<'a> {
    pub paint: &'a mut PaintList,
    pub text: &'a mut TextSystem,
    pub images: &'a dyn GraphImageHandles,
    pub family: &'a str,
}
fn color(c: GraphColor) -> paint::Color {
    c.map(|v| v as f32 / 255.)
}
fn rect(r: GraphRect) -> Rect {
    Rect::new(r.origin[0], r.origin[1], r.size[0], r.size[1])
}
impl GraphPainter for PaintGraphPainter<'_> {
    fn round_rect(&mut self, r: GraphRect, radius: f32, c: GraphColor) {
        self.paint.rect(rect(r), radius, color(c));
    }
    fn text_width(&mut self, s: &str, size: f32) -> f32 {
        self.text.layout(s, self.family, size, None).width()
    }
    fn text(&mut self, r: GraphRect, s: &str, size: f32, c: GraphColor) {
        let layout = self.text.layout(s, self.family, size, None);
        self.paint.ops.push(PaintOp::PushClip(rect(r)));
        for run in layout.runs {
            self.paint.ops.push(PaintOp::GlyphRun {
                run,
                origin: r.origin,
                color: color(c),
            });
        }
        self.paint.ops.push(PaintOp::PopClip);
    }
    fn curve(&mut self, p: [Point; 4], width: f32, c: GraphColor) {
        self.paint.ops.push(PaintOp::Stroke {
            path: Arc::new(paint::Path {
                elements: vec![
                    paint::PathElement::Move(p[0]),
                    paint::PathElement::Cubic(p[1], p[2], p[3]),
                ],
            }),
            stroke: paint::Stroke {
                width,
                cap: paint::LineCap::Round,
                join: paint::LineJoin::Round,
                brush: color(c).into(),
            },
        });
    }
    fn preview(&mut self, r: GraphRect, key: &str) {
        self.images.paint_preview(key, rect(r), self.paint);
    }
}
