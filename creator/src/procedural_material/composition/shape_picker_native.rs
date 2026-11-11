//! Preset thumbnails are analytic GPU geometry, not per-frame CPU images.
use super::*;
use std::sync::Arc;
use theframework::{
    paint::{FillRule, PaintList, PaintOp, Path, PathElement, Rect, Transform},
    theui::paint::{UiPaintResult, WidgetPainter, bounds, rgba},
};
impl Preset {
    fn paint_preview(&self, list: &mut PaintList, area: Rect, base: RGBA, added: RGBA) {
        let size = area.width;
        list.ops.push(PaintOp::PushClip(area));
        list.rect(area, 0., rgba(base));
        match self.shape {
            CompositionShape::Corner => {
                let center = corner(self.angle);
                let radius = self.position * size;
                list.rect(
                    Rect::new(
                        area.x + center[0] * size - radius,
                        area.y + center[1] * size - radius,
                        radius * 2.,
                        radius * 2.,
                    ),
                    radius,
                    rgba(added),
                );
            }
            CompositionShape::Strips => {
                let extent = self.position * size;
                if self.angle.abs() < 45. {
                    for y in [area.y, area.y + size - extent] {
                        list.rect(Rect::new(area.x, y, size, extent), 0., rgba(added));
                    }
                } else {
                    for x in [area.x, area.x + size - extent] {
                        list.rect(Rect::new(x, area.y, extent, size), 0., rgba(added));
                    }
                }
            }
            CompositionShape::Split
            | CompositionShape::Road
            | CompositionShape::Crossing
            | CompositionShape::AutoRoad => {
                let (sin, cos) = self.angle.to_radians().sin_cos();
                list.ops.push(PaintOp::PushTransform(Transform([
                    cos,
                    sin,
                    -sin,
                    cos,
                    area.x + size * 0.5,
                    area.y + size * 0.5,
                ])));
                let extent = size * 2.;
                let half = self.position * size * 0.5;
                match self.shape {
                    CompositionShape::Split => {
                        list.rect(
                            Rect::new(
                                (self.position - 0.5) * size,
                                -extent,
                                extent * 2.,
                                extent * 2.,
                            ),
                            0.,
                            rgba(added),
                        );
                    }
                    CompositionShape::Road => {
                        list.rect(
                            Rect::new(-extent, -half, extent * 2., half * 2.),
                            0.,
                            rgba(added),
                        );
                    }
                    _ => {
                        // A single union contour prevents overlap seams even
                        // if the active theme uses translucent accent colors.
                        let points = [
                            [-half, -extent],
                            [half, -extent],
                            [half, -half],
                            [extent, -half],
                            [extent, half],
                            [half, half],
                            [half, extent],
                            [-half, extent],
                            [-half, half],
                            [-extent, half],
                            [-extent, -half],
                            [-half, -half],
                        ];
                        let mut elements = vec![PathElement::Move(points[0])];
                        elements.extend(points[1..].iter().copied().map(PathElement::Line));
                        elements.push(PathElement::Close);
                        list.ops.push(PaintOp::Fill {
                            path: Arc::new(Path { elements }),
                            rule: FillRule::NonZero,
                            brush: rgba(added).into(),
                        });
                    }
                }
                list.ops.push(PaintOp::PopTransform);
            }
            _ => {
                let blend = std::array::from_fn(|channel| {
                    ((base[channel] as u16 + added[channel] as u16) / 2) as u8
                });
                list.rect(area, 0., rgba(blend));
            }
        }
        list.ops.push(PaintOp::PopClip);
    }
}
impl ShapePicker {
    pub(super) fn paint_native(&mut self, painter: &mut WidgetPainter<'_>) -> UiPaintResult {
        let outer = bounds(&self.dim);
        let width = self.dim.width / COLUMNS as i32;
        painter.list.ops.push(PaintOp::PushClip(outer));
        painter
            .list
            .rect(outer, 0., rgba(*painter.theme.color(MaterialPanel)));
        if width > 0 {
            for (index, preset) in PRESETS.iter().enumerate() {
                let card = Rect::new(
                    outer.x + (index % COLUMNS) as f32 * width as f32 + 3.,
                    outer.y + (index / COLUMNS) as f32 * CELL_HEIGHT as f32 + 2.,
                    (width as f32 - 6.).max(0.),
                    CELL_HEIGHT as f32 - 5.,
                );
                if card.intersection(outer).height == 0. {
                    continue;
                }
                let border = if self.selected == Some(index) {
                    SelectedTextEditBorder1
                } else if self.hover == Some(index) {
                    MaterialMuted
                } else {
                    MaterialGrid
                };
                painter.outline(card, *painter.theme.color(border), 1.);
                painter.list.ops.push(PaintOp::PushClip(card));
                let preview = Rect::new(
                    card.x + (card.width - PREVIEW_SIZE as f32).max(0.) * 0.5,
                    card.y + 3.,
                    PREVIEW_SIZE as f32,
                    PREVIEW_SIZE as f32,
                );
                preset.paint_preview(
                    painter.list,
                    preview,
                    *painter.theme.color(MaterialCanvas),
                    *painter.theme.color(SelectedTextEditBorder1),
                );
                painter.label(
                    Rect::new(card.x + 1., card.y + 38., (card.width - 2.).max(0.), 14.),
                    preset.label,
                    11.,
                    *painter.theme.color(MaterialText),
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
                painter.list.ops.push(PaintOp::PopClip);
            }
        }
        painter.list.ops.push(PaintOp::PopClip);
        self.dirty = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use theframework::theui::paint::{UiImageResources, UiPaintError};
    struct NoImages;
    impl UiImageResources for NoImages {
        fn image(
            &mut self,
            _: &str,
            _: &TheRGBABuffer,
            _: bool,
        ) -> Result<theframework::paint::ImageHandle, UiPaintError> {
            panic!("shape thumbnails must not request CPU raster images")
        }
    }
    #[test]
    fn native_shape_picker_uses_geometry_at_small_bounds_and_routes_the_visible_preset_once() {
        let mut ctx = TheContext::new(420, 260, 1.);
        let mut ui = TheUI::new();
        ui.init(&mut ctx);
        ctx.ui.command_paint = true;
        let (sender, receiver) = std::sync::mpsc::channel();
        ctx.ui.state_events_sender = Some(sender);
        let mut picker = ShapePicker::new(TheId::named(PICKER));
        let mut theme = TheBlackBlueTheme::new();
        for size in [(400, 232), (80, 60), (2, 2)] {
            picker.set_dim(TheDim::new(7, 9, size.0, size.1), &mut ctx);
            let mut list = PaintList::default();
            let mut p = WidgetPainter {
                list: &mut list,
                text: &mut ctx.text,
                images: &mut NoImages,
                ui: &ctx.ui,
                theme: &mut theme,
            };
            TheWidget::paint(&mut picker, &mut p).unwrap();
            list.validate().unwrap();
            assert!(
                !list
                    .ops
                    .iter()
                    .any(|op| matches!(op, PaintOp::Image { .. }))
            );
        }
        picker.set_dim(TheDim::new(7, 9, 400, 232), &mut ctx);
        picker.on_event(&TheEvent::MouseDown(Vec2::new(4 * 80 + 10, 10)), &mut ctx);
        assert_eq!(picker.selected, Some(4));
        assert_eq!(
            receiver
                .try_iter()
                .filter(|event| matches!(event,TheEvent::IndexChanged(id,4) if id.name==PICKER))
                .count(),
            1
        );
    }
    #[cfg(feature = "native-render")]
    #[test]
    #[ignore = "requires an actual native GPU adapter"]
    fn native_shape_gpu_coverage_matches_every_composition_preset_at_fractional_density() {
        use renderkit::{GpuContext, GpuImage, ImageDescriptor, ReadbackTicket};
        use theframework::render_wgpu::{PaintRenderer, UiImages};
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let mut images = UiImages::new(gpu.clone());
        let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
        let mut ctx = TheContext::new(420, 260, 1.);
        let mut ui = TheUI::new();
        ui.init(&mut ctx);
        let mut theme = TheBlackBlueTheme::new();
        let base = [20, 40, 60, 255];
        let added = [210, 100, 50, 255];
        theme.set_color(MaterialCanvas, base);
        theme.set_color(SelectedTextEditBorder1, added);
        let mut picker = ShapePicker::new(TheId::named(PICKER));
        picker.set_dim(TheDim::new(7, 9, 400, 232), &mut ctx);
        let mut list = PaintList::default();
        TheWidget::paint(
            &mut picker,
            &mut WidgetPainter {
                list: &mut list,
                text: &mut ctx.text,
                images: &mut images,
                ui: &ctx.ui,
                theme: &mut theme,
            },
        )
        .unwrap();
        assert!(
            !list
                .ops
                .iter()
                .any(|op| matches!(op, PaintOp::Image { .. }))
        );
        for density in [1_f32, 1.5, 2.] {
            let size = (
                (420. * density).round() as u32,
                (260. * density).round() as u32,
            );
            let output = GpuImage::new(&gpu, ImageDescriptor::rgba(size.0, size.1)).unwrap();
            renderer
                .render(
                    &list,
                    &images.registry,
                    &output.view,
                    output.descriptor.format,
                    size,
                    density,
                    [0.; 4],
                )
                .unwrap();
            let rasterized = renderer.glyphs.rasterizations;
            let uploaded = gpu.transfers().uploaded_bytes;
            renderer
                .render(
                    &list,
                    &images.registry,
                    &output.view,
                    output.descriptor.format,
                    size,
                    density,
                    [0.; 4],
                )
                .unwrap();
            assert_eq!(renderer.glyphs.rasterizations, rasterized);
            assert_eq!(gpu.transfers().uploaded_bytes, uploaded);
            let mut ticket = ReadbackTicket::capture(&gpu, output).unwrap();
            gpu.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(std::time::Duration::from_secs(30)),
                })
                .unwrap();
            let pixels = ticket.try_finish(&gpu).unwrap().unwrap();
            let mut compared = 0;
            for (index, preset) in PRESETS.iter().enumerate() {
                let origin = [
                    30. + (index % 5) as f32 * 80.,
                    14. + (index / 5) as f32 * 58.,
                ];
                for y in 0..5 {
                    for x in 0..5 {
                        let uv = [(x as f32 + 0.31) / 5., (y as f32 + 0.27) / 5.];
                        let amount = preset.coverage(uv);
                        let inset = 1.25 / 34.;
                        if [-inset, inset].iter().any(|dx| {
                            [-inset, inset]
                                .iter()
                                .any(|dy| preset.coverage([uv[0] + dx, uv[1] + dy]) != amount)
                        }) {
                            continue;
                        }
                        let px = ((origin[0] + uv[0] * 34.) * density) as usize;
                        let py = ((origin[1] + uv[1] * 34.) * density) as usize;
                        let offset = (py * size.0 as usize + px) * 4;
                        for c in 0..3 {
                            let expected =
                                (base[c] as f32 * (1. - amount) + added[c] as f32 * amount) as u8;
                            assert!(
                                pixels.pixels()[offset + c].abs_diff(expected) <= 2,
                                "{} at {uv:?} density {density}: {:?}, expected channel {c} = {expected}",
                                preset.label,
                                &pixels.pixels()[offset..offset + 4]
                            );
                        }
                        compared += 1;
                    }
                }
            }
            assert!(compared > 200);
            if density == 1.5 {
                std::fs::write(
                    "/private/tmp/eldiron-native-composition-shapes.png",
                    TheRGBABuffer::from(pixels.pixels().to_vec(), size.0, size.1)
                        .to_png()
                        .unwrap(),
                )
                .unwrap();
            }
        }
    }
}
