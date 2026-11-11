//! Editable source pixels are uploaded assets. View display has no raster buffer.
use super::*;
use crate::paint::{PaintOp, Rect, Sampling};
use crate::theui::paint::{UiPaintResult, WidgetPainter, bounds, rgba};
impl TheRGBAView {
    pub(super) fn paint_gpu(&mut self, p: &mut WidgetPainter<'_>) -> UiPaintResult {
        let view = bounds(&self.dim);
        p.list.rect(view, 0., rgba(self.background));
        if !self.buffer.is_valid() || !self.zoom.is_finite() || self.zoom <= 0. {
            self.is_dirty = false;
            return Ok(());
        }
        let width = self.buffer.dim().width as f32;
        let height = self.buffer.dim().height as f32;
        let scaled = [width * self.zoom, height * self.zoom];
        let x = if scaled[0] < view.width {
            (view.width - scaled[0]) * 0.5
        } else {
            -self.scroll_offset.x as f32
        };
        let y = if scaled[1] < view.height {
            (view.height - scaled[1]) * 0.5
        } else {
            -self.scroll_offset.y as f32
        };
        let image_rect = Rect::new(view.x + x, view.y + y, scaled[0], scaled[1]);
        let visible = view.intersection(image_rect);
        let image = p.images.image(
            &format!("rgba:{}", self.id.uuid),
            &self.buffer,
            self.source_dirty,
        )?;
        self.source_dirty = false;
        p.list.ops.push(PaintOp::PushClip(visible));
        if self.show_transparency {
            p.list.rect(image_rect, 0., rgba(self.transparency_color));
        }
        p.list.ops.push(PaintOp::Image {
            image,
            rect: image_rect,
            uv: Rect::new(0., 0., 1., 1.),
            sampling: Sampling::Nearest,
            tint: if self.icon_mode {
                [0.5, 0.5, 0.5, 1.]
            } else {
                [1.; 4]
            },
        });
        if let Some(grid) = self.grid {
            let gw = grid.max(1) as f32 * self.zoom;
            let gh = self.cell_height() as f32 * self.zoom;
            let cell = |x: i32, y: i32| {
                Rect::new(
                    image_rect.x + x as f32 * gw,
                    image_rect.y + y as f32 * gh,
                    gw,
                    gh,
                )
            };
            let first_x = ((visible.x - image_rect.x) / gw).floor().max(0.) as i32;
            let first_y = ((visible.y - image_rect.y) / gh).floor().max(0.) as i32;
            let last_x = ((visible.x + visible.width - image_rect.x) / gw)
                .ceil()
                .min((width / grid.max(1) as f32).ceil()) as i32;
            let last_y = ((visible.y + visible.height - image_rect.y) / gh)
                .ceil()
                .min((height / self.cell_height() as f32).ceil()) as i32;
            // Sparse state is painted directly; zooming far out must not loop
            // over every source cell to find empty overlays.
            let on_screen = |r: Rect| {
                let v = r.intersection(visible);
                v.width > 0. && v.height > 0.
            };
            if self.icon_mode {
                for &(cx, cy) in &self.selected {
                    let rect = cell(cx, cy).intersection(image_rect);
                    if on_screen(rect) {
                        p.list.ops.push(PaintOp::Image {
                            image,
                            rect,
                            uv: Rect::new(
                                (rect.x - image_rect.x) / image_rect.width,
                                (rect.y - image_rect.y) / image_rect.height,
                                rect.width / image_rect.width,
                                rect.height / image_rect.height,
                            ),
                            sampling: Sampling::Nearest,
                            tint: [1.; 4],
                        });
                    }
                }
            } else {
                if self.mode == TheRGBAViewMode::TilePicker {
                    for &(cx, cy) in &self.selected {
                        let r = cell(cx, cy);
                        if on_screen(r) {
                            p.list.rect(r, 0., rgba(self.selection_color));
                        }
                    }
                }
                for &(cx, cy) in &self.used {
                    if self.mode == TheRGBAViewMode::TilePicker && self.selected.contains(&(cx, cy))
                    {
                        continue;
                    }
                    let r = cell(cx, cy);
                    if on_screen(r) {
                        p.list.rect(r, 0., rgba(self.used_color));
                    }
                }
                if let (Some((cx, cy)), Some(color)) = (self.hover, self.hover_color) {
                    if !self.used.contains(&(cx, cy))
                        && !(self.mode == TheRGBAViewMode::TilePicker
                            && self.selected.contains(&(cx, cy)))
                    {
                        let r = cell(cx, cy);
                        if on_screen(r) {
                            p.list.rect(r, 0., rgba(color));
                        }
                    }
                }
            }
            if self.mode == TheRGBAViewMode::TileEditor {
                if let Some((cx, cy)) = self.drop {
                    let r = cell(cx, cy);
                    if on_screen(r) {
                        p.list.rect(r, 0., rgba(WHITE));
                    }
                }
            }
            if !self.dont_show_grid && self.mode == TheRGBAViewMode::TileEditor {
                if gw <= 1. || gh <= 1. {
                    p.list.rect(image_rect, 0., rgba(self.grid_color));
                } else {
                    for cx in first_x..=last_x {
                        p.list.rect(
                            Rect::new(
                                image_rect.x + cx as f32 * gw,
                                image_rect.y,
                                1.,
                                image_rect.height,
                            ),
                            0.,
                            rgba(self.grid_color),
                        );
                    }
                    for cy in first_y..=last_y {
                        p.list.rect(
                            Rect::new(
                                image_rect.x,
                                image_rect.y + cy as f32 * gh,
                                image_rect.width,
                                1.,
                            ),
                            0.,
                            rgba(self.grid_color),
                        );
                    }
                    p.list.rect(
                        Rect::new(
                            image_rect.x + image_rect.width - 1.,
                            image_rect.y,
                            1.,
                            image_rect.height,
                        ),
                        0.,
                        rgba(self.grid_color),
                    );
                    p.list.rect(
                        Rect::new(
                            image_rect.x,
                            image_rect.y + image_rect.height - 1.,
                            image_rect.width,
                            1.,
                        ),
                        0.,
                        rgba(self.grid_color),
                    );
                }
            }
        }
        if let Some((preview, position)) = &self.paste_preview {
            if preview.is_valid() {
                let handle = p.images.image(
                    &format!("rgba-paste:{}", self.id.uuid),
                    preview,
                    self.preview_dirty,
                )?;
                self.preview_dirty = false;
                p.list.ops.push(PaintOp::Image {
                    image: handle,
                    rect: Rect::new(
                        image_rect.x + position.x as f32 * self.zoom,
                        image_rect.y + position.y as f32 * self.zoom,
                        preview.dim().width as f32 * self.zoom,
                        preview.dim().height as f32 * self.zoom,
                    ),
                    uv: Rect::new(0., 0., 1., 1.),
                    sampling: Sampling::Nearest,
                    tint: [1., 1., 1., 0.65],
                });
            }
        }
        if let Some(grid) = self.grid {
            let gw = grid.max(1) as f32 * self.zoom;
            let gh = self.cell_height() as f32 * self.zoom;
            if gw >= 1. && gh >= 1. {
                let cell = |x: i32, y: i32| {
                    Rect::new(
                        image_rect.x + x as f32 * gw,
                        image_rect.y + y as f32 * gh,
                        gw,
                        gh,
                    )
                };
                if self.mode == TheRGBAViewMode::TileEditor
                    || self.mode == TheRGBAViewMode::TileSelection
                {
                    for &(x, y) in &self.selected {
                        let r = cell(x, y);
                        if r.intersection(visible).width <= 0.
                            || r.intersection(visible).height <= 0.
                        {
                            continue;
                        }
                        if !self.selected.contains(&(x - 1, y)) {
                            p.list.rect(
                                Rect::new(r.x, r.y, 1., r.height),
                                0.,
                                rgba(self.selection_color),
                            );
                        }
                        if !self.selected.contains(&(x + 1, y)) {
                            p.list.rect(
                                Rect::new(r.x + r.width - 1., r.y, 1., r.height),
                                0.,
                                rgba(self.selection_color),
                            );
                        }
                        if !self.selected.contains(&(x, y - 1)) {
                            p.list.rect(
                                Rect::new(r.x, r.y, r.width, 1.),
                                0.,
                                rgba(self.selection_color),
                            );
                        }
                        if !self.selected.contains(&(x, y + 1)) {
                            p.list.rect(
                                Rect::new(r.x, r.y + r.height - 1., r.width, 1.),
                                0.,
                                rgba(self.selection_color),
                            );
                        }
                    }
                }
                if self.mode == TheRGBAViewMode::TileEditor {
                    for (anchor, color) in &self.anchor_points {
                        let r = cell(anchor.x, anchor.y);
                        p.list
                            .rect(Rect::new(r.x, r.y, r.width, 1.), 0., rgba(*color));
                        p.list.rect(
                            Rect::new(r.x, r.y + r.height - 1., r.width, 1.),
                            0.,
                            rgba(*color),
                        );
                        p.list
                            .rect(Rect::new(r.x, r.y, 1., r.height), 0., rgba(*color));
                        p.list.rect(
                            Rect::new(r.x + r.width - 1., r.y, 1., r.height),
                            0.,
                            rgba(*color),
                        );
                        p.list.rect(
                            Rect::new(r.x + r.width * 0.5 - 1., r.y, 2., r.height),
                            0.,
                            rgba(*color),
                        );
                        p.list.rect(
                            Rect::new(r.x, r.y + r.height * 0.5 - 1., r.width, 2.),
                            0.,
                            rgba(*color),
                        );
                    }
                }
            }
        }
        p.list.ops.push(PaintOp::PopClip);
        if self.id.equals(&p.ui.focus) {
            p.outline(view, *p.theme.color(DefaultSelection), 1.);
        }
        self.is_dirty = false;
        Ok(())
    }
}
