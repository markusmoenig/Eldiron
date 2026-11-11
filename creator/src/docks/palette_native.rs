//! Native palette board: shared authored layout, procedural swatches and bounds.
use super::*;
use theframework::{
    paint::{PaintOp, Rect},
    theui::paint::{UiPaintResult, WidgetPainter, bounds, rgba},
};
impl PaletteDockBoard {
    pub(super) fn prepare_native_geometry(&mut self) {
        let key = (self.dim.width, self.dim.height, self.visible_count());
        if self.native_layout_key == Some(key) {
            return;
        }
        let (columns, rows, cell) = self.calc_layout();
        self.rectangles.clear();
        for row in 0..rows {
            for column in 0..columns {
                let index = self.rectangles.len();
                if index >= self.visible_count() {
                    break;
                }
                self.rectangles.push((
                    index,
                    TheDim::new(10 + column * (cell + 1), 8 + row * (cell + 1), cell, cell),
                ));
            }
        }
        self.native_layout_key = Some(key);
    }
    pub(super) fn paint_native(&mut self, painter: &mut WidgetPainter<'_>) -> UiPaintResult {
        self.prepare_native_geometry();
        let outer = bounds(&self.dim);
        painter.list.ops.push(PaintOp::PushClip(outer));
        painter.fill(outer, ListLayoutBackground);
        for (index, dim) in &self.rectangles {
            let cell = Rect::new(
                outer.x + dim.x as f32,
                outer.y + dim.y as f32,
                dim.width as f32,
                dim.height as f32,
            );
            if cell.intersection(outer).width == 0. || cell.intersection(outer).height == 0. {
                continue;
            }
            if self.index == *index || self.hovered_index == Some(*index) {
                painter.outline(
                    cell,
                    *painter.theme.color(if self.index == *index {
                        DefaultSelection
                    } else {
                        ListItemHover
                    }),
                    1.,
                );
            }
            painter.outline(cell.inset(1.), *painter.theme.color(ListItemIconBorder), 1.);
            let color = self
                .palette
                .colors
                .get(*index)
                .and_then(Option::as_ref)
                .map_or_else(
                    || *painter.theme.color(DefaultWidgetDarkBackground),
                    TheColor::to_u8_array,
                );
            painter.list.rect(cell.inset(2.), 0., rgba(color));
        }
        painter.list.ops.push(PaintOp::PopClip);
        self.is_dirty = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_palette_hit_geometry_exists_before_paint_and_cancelled_drags_never_swap_later() {
        let mut ctx = TheContext::new(240, 180, 1.);
        let mut ui = TheUI::new();
        ui.init(&mut ctx);
        ctx.ui.command_paint = true;
        let (sender, receiver) = std::sync::mpsc::channel();
        ctx.ui.state_events_sender = Some(sender);
        let mut board = PaletteDockBoard::new(TheId::named(PALETTE_DOCK_PICKER));
        board.palette.colors = vec![
            Some(TheColor::from([255u8, 0, 0, 255])),
            None,
            Some(TheColor::from([0u8, 0, 255, 255])),
            None,
        ];
        board.set_dim(TheDim::new(17, 23, 180, 140), &mut ctx);
        board.prepare_paint(&mut ctx, &TheBlackBlueTheme::new());
        let a = board.rectangles[0].1;
        let b = board.rectangles[2].1;
        let point = |r: TheDim| Vec2::new(r.x + r.width / 2, r.y + r.height / 2);
        board.on_event(&TheEvent::MouseDown(point(a)), &mut ctx);
        assert_eq!(board.index, 0);
        let mut synchronized = board.palette.clone();
        synchronized.current_index = 2;
        board.set_palette(synchronized);
        board.on_event(&TheEvent::MouseUp(point(b)), &mut ctx);
        assert_eq!(
            receiver
                .try_iter()
                .filter(|e| matches!(e, TheEvent::PaletteEntriesSwapped(_, 0, 2)))
                .count(),
            1
        );
        board.on_event(&TheEvent::MouseDown(point(a)), &mut ctx);
        let mut replacement = board.palette.clone();
        replacement.colors[0] = Some(TheColor::from([0u8, 255, 0, 255]));
        board.set_palette(replacement);
        board.on_event(&TheEvent::MouseUp(point(b)), &mut ctx);
        assert!(
            !receiver
                .try_iter()
                .any(|e| matches!(e, TheEvent::PaletteEntriesSwapped(..)))
        );
        for read_only in [true, false] {
            board.on_event(&TheEvent::MouseDown(point(a)), &mut ctx);
            if read_only {
                board.set_read_only(true);
                board.set_read_only(false);
            } else {
                board.on_event(&TheEvent::LostFocus(board.id.clone()), &mut ctx);
            }
            board.on_event(&TheEvent::MouseUp(point(b)), &mut ctx);
            assert!(
                !receiver
                    .try_iter()
                    .any(|e| matches!(e, TheEvent::PaletteEntriesSwapped(..)))
            );
        }
        board.on_event(&TheEvent::Hover(point(a)), &mut ctx);
        board.is_dirty = false;
        assert!(board.on_event(&TheEvent::LostHover(board.id.clone()), &mut ctx));
        assert!(board.is_dirty);
    }
    #[cfg(feature = "native-render")]
    #[test]
    #[ignore = "requires an actual native GPU adapter"]
    fn native_palette_gpu_paints_sparse_slots_with_no_source_image_uploads() {
        use renderkit::{GpuContext, GpuImage, ImageDescriptor, ReadbackTicket};
        use theframework::{
            paint::PaintList,
            render_wgpu::{PaintRenderer, UiImages},
        };
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let mut images = UiImages::new(gpu.clone());
        let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
        let mut ctx = TheContext::new(240, 180, 1.5);
        let mut ui = TheUI::new();
        ui.init(&mut ctx);
        let mut theme = TheBlackBlueTheme::new();
        let mut board = PaletteDockBoard::new(TheId::named(PALETTE_DOCK_PICKER));
        board.palette.colors = vec![
            Some(TheColor::from([255u8, 0, 0, 255])),
            None,
            Some(TheColor::from([0u8, 0, 255, 255])),
            None,
        ];
        board.set_dim(TheDim::new(17, 23, 180, 140), &mut ctx);
        board.prepare_paint(&mut ctx, &theme);
        let output = GpuImage::new(&gpu, ImageDescriptor::rgba(360, 270)).unwrap();
        for hovered in [None, Some(2)] {
            board.hovered_index = hovered;
            let mut list = PaintList::default();
            TheWidget::paint(
                &mut board,
                &mut WidgetPainter {
                    list: &mut list,
                    text: &mut ctx.text,
                    images: &mut images,
                    ui: &ctx.ui,
                    theme: &mut theme,
                },
            )
            .unwrap();
            list.validate().unwrap();
            assert!(
                !list
                    .ops
                    .iter()
                    .any(|op| matches!(op, PaintOp::Image { .. } | PaintOp::GlyphRun { .. }))
            );
            renderer
                .render(
                    &list,
                    &images.registry,
                    &output.view,
                    output.descriptor.format,
                    (360, 270),
                    1.5,
                    [0.; 4],
                )
                .unwrap();
        }
        assert_eq!(gpu.transfers().uploaded_bytes, 0);
        assert_eq!(gpu.transfers().readback_bytes, 0);
        let mut ticket = ReadbackTicket::capture(&gpu, output).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        let pixels = ticket.try_finish(&gpu).unwrap().unwrap();
        for (index, expected) in [
            (0, [255u8, 0, 0, 255]),
            (1, *theme.color(DefaultWidgetDarkBackground)),
            (2, [0u8, 0, 255, 255]),
        ] {
            let r = board.rectangles[index].1;
            let x = ((board.dim.x + r.x + r.width / 2) as f32 * 1.5) as usize;
            let y = ((board.dim.y + r.y + r.height / 2) as f32 * 1.5) as usize;
            for channel in 0..4 {
                assert!(
                    pixels.pixels()[(y * 360 + x) * 4 + channel].abs_diff(expected[channel]) <= 1
                );
            }
        }
        std::fs::write(
            "/private/tmp/eldiron-native-palette-board.png",
            TheRGBABuffer::from(pixels.pixels().to_vec(), 360, 270)
                .to_png()
                .unwrap(),
        )
        .unwrap();
    }
}
