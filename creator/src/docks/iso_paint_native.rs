//! GPU chrome/text and cached source-only brush artwork for the Paint dock.
use super::*;
use theframework::{
    paint::{PaintOp, Rect, Sampling},
    theui::paint::{UiPaintResult, WidgetPainter, bounds},
};

struct Preview {
    brush: String,
    palette: Vec<[u8; 4]>,
    mode: IsoPaintPreviewMode,
    pixels: TheRGBABuffer,
}
#[derive(Default)]
pub(super) struct BrushPreviews {
    items: std::collections::HashMap<usize, Preview>,
}
impl BrushPreviews {
    fn source(
        &mut self,
        index: usize,
        brush: &str,
        palette: &[[u8; 4]],
        mode: IsoPaintPreviewMode,
        size: (i32, i32),
    ) -> (&TheRGBABuffer, bool) {
        let changed = self.items.get(&index).is_none_or(|old| {
            old.brush != brush
                || old.palette != palette
                || old.mode != mode
                || old.pixels.dim().width != size.0
                || old.pixels.dim().height != size.1
        });
        if changed {
            let mut pixels = TheRGBABuffer::new(TheDim::sized(size.0, size.1));
            // No fonts, context, destination canvas or GPU is initialized to
            // generate the editable brush's sampled artwork.
            let draw = TheDraw2D {
                mask: None,
                mask_size: (0, 0),
                fonts: Vec::new(),
                code_fonts: Vec::new(),
            };
            IsoPaintBrushBoard::draw_engine_preview(
                &mut pixels,
                &draw,
                &(0, 0, size.0 as usize, size.1 as usize),
                size.0 as usize,
                brush,
                palette,
                mode,
            );
            self.items.insert(
                index,
                Preview {
                    brush: brush.into(),
                    palette: palette.to_vec(),
                    mode,
                    pixels,
                },
            );
        }
        (&self.items.get(&index).unwrap().pixels, changed)
    }
}
impl IsoPaintPresetStrip {
    pub(super) fn paint_native(&mut self, p: &mut WidgetPainter<'_>) -> UiPaintResult {
        let outer = bounds(&self.dim);
        if outer.width <= 0. || outer.height <= 0. {
            return Ok(());
        }
        self.clamp_scroll();
        p.list.ops.push(PaintOp::PushClip(outer));
        p.fill(outer, ListLayoutBackground);
        self.rectangles.clear();
        let width = Self::tile_width();
        let height = (self.dim.height - 8).clamp(30, 38);
        for (index, brush) in IsoPaintDock::BRUSHES.iter().enumerate() {
            let x = 7 + index as i32 * (width + 5) - self.scroll_offset;
            if x + width <= 0 || x >= self.dim.width {
                continue;
            }
            let local = TheDim::new(x, 4, width, height);
            let rect = Rect::new(
                outer.x + x as f32,
                outer.y + 4.,
                width as f32,
                height as f32,
            );
            let selected = self.selected == index;
            let hover = self.hovered == Some(index);
            p.fill(
                rect,
                if selected {
                    ListItemSelected
                } else if hover {
                    ListItemHover
                } else {
                    ListItemNormal
                },
            );
            let palette = self
                .preview_palettes
                .get(index)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let mode = if IsoPaintDock::brush_supports_stamp(brush.key) {
                IsoPaintPreviewMode::Stamp
            } else {
                IsoPaintPreviewMode::Paint
            };
            let (source, changed) = self.native_previews.source(
                index,
                brush.key,
                palette,
                mode,
                (width - 4, height - 4),
            );
            p.image(
                &format!("paint-brush:{}:{index}", self.id.uuid),
                source,
                rect.inset(2.),
                changed,
                Sampling::Nearest,
            )?;
            p.outline(
                rect,
                *p.theme.color(if selected {
                    DefaultSelection
                } else if hover {
                    ListItemHover
                } else {
                    ListItemIconBorder
                }),
                1.,
            );
            self.rectangles.push((index, local));
        }
        p.list.ops.push(PaintOp::PopClip);
        self.is_dirty = false;
        Ok(())
    }
}
impl IsoPaintBrushEditor {
    pub(super) fn paint_native(&mut self, p: &mut WidgetPainter<'_>) -> UiPaintResult {
        let outer = bounds(&self.dim);
        p.list.ops.push(PaintOp::PushClip(outer));
        p.fill(outer, ListLayoutBackground);
        if self.dim.width >= 96 && self.dim.height >= 72 {
            let panel = outer.inset(8.);
            p.fill(panel, ListItemNormal);
            p.outline(panel, *p.theme.color(ListItemIconBorder), 1.);
            let brush = self.selected_key();
            p.label(
                Rect::new(panel.x + 8., panel.y, panel.width - 16., 24.),
                &IsoPaintDock::brush_label(brush),
                11.,
                *p.theme.color(ListItemText),
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );
            let rect = Rect::new(
                panel.x + 7.,
                panel.y + 32.,
                (panel.width - 14.).max(1.),
                (panel.height - 40.).max(1.),
            );
            let (source, changed) = self.native_previews.source(
                0,
                brush,
                &self.preview_palette,
                self.preview_mode,
                (rect.width as i32, rect.height as i32),
            );
            p.image(
                &format!("paint-brush-editor:{}", self.id.uuid),
                source,
                rect,
                changed,
                Sampling::Nearest,
            )?;
        }
        p.list.ops.push(PaintOp::PopClip);
        self.is_dirty = false;
        Ok(())
    }
}
impl IsoPaintMaterialStrip {
    pub(super) fn paint_native(&mut self, p: &mut WidgetPainter<'_>) -> UiPaintResult {
        let outer = bounds(&self.dim);
        p.list.ops.push(PaintOp::PushClip(outer));
        p.fill(outer, ListLayoutBackground);
        self.preset_rects.clear();
        self.finish_rects.clear();
        let available = (self.dim.width - 16).max(1);
        let preset_w = ((available - (MATERIAL_PRESET_VALUES.len() as i32 - 1) * 3)
            / MATERIAL_PRESET_VALUES.len() as i32)
            .max(12);
        let finish_w = ((available - (MATERIAL_FINISH_VALUES.len() as i32 - 1) * 3)
            / MATERIAL_FINISH_VALUES.len() as i32)
            .max(38);
        for (kind, labels, width, y, height, selected, size) in [
            (
                "preset",
                Self::material_labels_short().as_slice(),
                preset_w,
                5,
                21,
                self.material_preset,
                9.5,
            ),
            (
                "finish",
                Self::finish_labels_short().as_slice(),
                finish_w,
                31,
                22,
                self.material_finish,
                10.,
            ),
        ] {
            for (index, label) in labels.iter().enumerate() {
                let local = TheDim::new(8 + index as i32 * (width + 3), y, width, height);
                let rect = Rect::new(
                    outer.x + local.x as f32,
                    outer.y + y as f32,
                    width as f32,
                    height as f32,
                );
                let active = selected == index;
                let hover = self.hovered == Some((kind, index));
                let (fill, border) = if active {
                    (GroupButtonSelectedBackground, GroupButtonSelectedBorder)
                } else if hover {
                    (GroupButtonHoverBackground, GroupButtonHoverBorder)
                } else {
                    (GroupButtonNormalBackground, GroupButtonNormalBorder)
                };
                p.fill(rect, fill);
                p.outline(rect, *p.theme.color(border), 1.);
                p.label(
                    rect.inset(1.),
                    label,
                    size,
                    *p.theme.color(ListItemText),
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
                if kind == "preset" {
                    self.preset_rects.push((index, local));
                } else {
                    self.finish_rects.push((index, local));
                }
            }
        }
        p.list.ops.push(PaintOp::PopClip);
        self.is_dirty = false;
        Ok(())
    }
}
