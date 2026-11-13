//! Retained palette plane and presentation-scoped analytical color input.
use super::*;
use std::sync::Arc;
use theframework::{
    paint::{FrameReceipt, PaintList, PaintOp, Rect, new_receipt_owner},
    thergbabuffer::hsl_hue_waveform_color as sample,
    theui::{
        paint::{UiPaintResult, WidgetPainter},
        thewidget::therenderview::RenderViewPaintSource,
    },
};

#[derive(Clone, PartialEq)]
struct Binding {
    document: Uuid,
    index: usize,
    view: TheId,
    rect: [i32; 4],
    density: u32,
}
struct Frame {
    binding: Binding,
    color: Option<[u8; 3]>,
    paint: PaintList,
    receipt: FrameReceipt,
}
impl RenderViewPaintSource for Frame {
    fn paint(&self, p: &mut WidgetPainter<'_>, viewport: [f32; 2]) -> UiPaintResult {
        if viewport == [self.binding.rect[2] as f32, self.binding.rect[3] as f32] {
            p.list.ops.extend(self.paint.ops.iter().cloned());
            p.list.ops.push(PaintOp::Receipt(self.receipt));
        }
        Ok(())
    }
}
#[derive(Clone)]
pub(crate) struct Token(Arc<Frame>);
impl Token {
    pub(crate) fn receipt(&self) -> FrameReceipt {
        self.0.receipt
    }
}
pub(crate) struct PaletteOverview {
    owner: u64,
    serial: u64,
    enabled: bool,
    current: Option<Arc<Frame>>,
    presented: Option<Arc<Frame>>,
}
impl Default for PaletteOverview {
    fn default() -> Self {
        Self {
            owner: new_receipt_owner(),
            serial: 0,
            enabled: false,
            current: None,
            presented: None,
        }
    }
}
impl PaletteOverview {
    fn binding(
        project: &Project,
        server: &ServerContext,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) -> Option<Binding> {
        if !palette_minimap_active(server) {
            return None;
        }
        let view = ui.get_render_view("MiniMap")?;
        let dim = *view.dim();
        if dim.width <= 0 || dim.height <= 0 {
            return None;
        }
        Some(Binding {
            document: server.document_instance,
            index: project.palette.current_index as usize,
            view: view.id().clone(),
            rect: [dim.x, dim.y, dim.width, dim.height],
            density: ctx.scale_factor.to_bits(),
        })
    }
    pub(crate) fn publish(
        &mut self,
        project: &Project,
        server: &ServerContext,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) -> bool {
        if Self::binding(project, server, ui, ctx).is_none() {
            return false;
        }
        self.enabled = true;
        self.prepare(project, server, ui, ctx);
        true
    }
    pub(crate) fn prepare(
        &mut self,
        project: &Project,
        server: &ServerContext,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) -> Option<Token> {
        if !self.enabled {
            return None;
        }
        let Some(binding) = Self::binding(project, server, ui, ctx) else {
            self.invalidate(ui);
            return None;
        };
        let color = project
            .palette
            .get_current_color()
            .map(|c| c.to_u8_array_3());
        if let Some(frame) = &self.current
            && frame.binding == binding
            && frame.color == color
        {
            return Some(Token(frame.clone()));
        }
        let presented = self
            .presented
            .clone()
            .filter(|frame| frame.binding == binding);
        self.invalidate(ui);
        self.presented = presented;
        let size = [binding.rect[2], binding.rect[3]];
        let mut paint = PaintList::default();
        paint.ops.push(PaintOp::HueLightness {
            rect: Rect::new(0., 0., size[0] as f32, size[1] as f32),
        });
        if let Some(color) = color {
            let mut closest = (u32::MAX, [0, 0]);
            // Retain the exact original tie-breaking order. Only color/size
            // publication does this search; paint and pointer sampling are O(1).
            for y in 0..size[1] {
                for x in 0..size[0] {
                    let pixel = sample(size[0], size[1], x, y).unwrap();
                    let distance = (0..3)
                        .map(|c| {
                            let d = pixel[c] as i32 - color[c] as i32;
                            (d * d) as u32
                        })
                        .sum();
                    if distance < closest.0 {
                        closest = (distance, [x, y]);
                    }
                }
            }
            let (x, y) = (closest.1[0] as f32 - 4., closest.1[1] as f32 - 4.);
            paint.ops.push(PaintOp::PixelRects {
                rects: vec![
                    Rect::new(x, y, 8., 1.),
                    Rect::new(x, y + 7., 8., 1.),
                    Rect::new(x, y, 1., 8.),
                    Rect::new(x + 7., y, 1., 8.),
                ]
                .into_iter()
                .map(|rect| rect.intersection(Rect::new(0., 0., size[0] as f32, size[1] as f32)))
                .filter(|rect| rect.width > 0. && rect.height > 0.)
                .collect::<Vec<_>>()
                .into(),
                color: [1.; 4],
            });
        }
        self.serial = self
            .serial
            .checked_add(1)
            .expect("palette receipt exhausted");
        let frame = Arc::new(Frame {
            binding,
            color,
            paint,
            receipt: FrameReceipt {
                owner: self.owner,
                serial: self.serial,
            },
        });
        ui.get_render_view("MiniMap")
            .unwrap()
            .set_paint_source(Some(frame.clone()));
        self.current = Some(frame.clone());
        Some(Token(frame))
    }
    pub(crate) fn presented(&mut self, token: Option<Token>) {
        self.presented = token
            .filter(|token| {
                self.current.as_ref().is_some_and(|current| {
                    token.receipt().owner == self.owner && token.0.binding == current.binding
                })
            })
            .map(|token| token.0);
    }
    pub(crate) fn invalidate(&mut self, ui: &mut TheUI) {
        self.presented = None;
        if let Some(frame) = self.current.take() {
            let expected: Arc<dyn RenderViewPaintSource> = frame.clone();
            if let Some(view) = ui.get_render_view(&frame.binding.view.name)
                && view.id() == &frame.binding.view
            {
                view.clear_paint_source(&expected);
            }
        }
    }
    pub(crate) fn deactivate(&mut self, ui: &mut TheUI) {
        self.invalidate(ui);
        self.enabled = false;
    }
    pub(crate) fn color(
        &self,
        id: &TheId,
        at: Vec2<i32>,
        project: &Project,
        server: &ServerContext,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) -> Result<Option<[u8; 4]>, ()> {
        let binding = Self::binding(project, server, ui, ctx).ok_or(())?;
        let frame = self.presented.as_ref().ok_or(())?;
        if frame.binding != binding || frame.binding.view != *id {
            return Err(());
        }
        Ok(sample(binding.rect[2], binding.rect[3], at.x, at.y))
    }
}

/// Palette paint/oracle and ownership cases run in the existing source matrix.
#[cfg(test)]
pub(crate) fn reference_cases() -> Vec<(&'static str, Arc<dyn RenderViewPaintSource>, TheRGBABuffer)>
{
    use crate::{
        dockmanager::DockManagerState,
        editor::{DOCKMANAGER, PALETTE},
        sidebar::Sidebar,
    };
    struct Restore(String, DockManagerState, ThePalette);
    impl Drop for Restore {
        fn drop(&mut self) {
            let mut manager = DOCKMANAGER.write().unwrap();
            manager.dock = self.0.clone();
            manager.state = self.1;
            *PALETTE.write().unwrap() = self.2.clone();
        }
    }
    let _restore = {
        let mut manager = DOCKMANAGER.write().unwrap();
        let saved = Restore(
            manager.dock.clone(),
            manager.state,
            PALETTE.read().unwrap().clone(),
        );
        manager.dock = "Palette".into();
        manager.state = DockManagerState::Editor;
        saved
    };
    let mut project = Project::default();
    let mut server = ServerContext::new();
    server.palette_tool_active = true;
    let mut cases = Vec::new();
    for size in [Vec2::new(143, 83), Vec2::new(84, 144)] {
        let mut ctx = TheContext::new(size.x as usize, size.y as usize, 1.);
        ctx.ui.command_paint = true;
        let mut ui = TheUI::new();
        let mut view = TheRenderView::new(TheId::named("MiniMap"));
        let id = view.id().clone();
        view.set_dim(TheDim::new(13, 29, size.x, size.y), &mut ctx);
        view.render_buffer_mut().resize(size.x, size.y);
        ui.canvas.set_widget(view);
        let mut owner = PaletteOverview::default();
        for color in [[90, 140, 180], [255, 255, 255], [0, 0, 0]] {
            let index = project.palette.current_index as usize;
            project.palette[index] = Some(TheColor::from([color[0], color[1], color[2], 255]));
            assert!(owner.publish(&project, &server, &mut ui, &ctx));
            assert!(
                !ui.get_render_view("MiniMap")
                    .unwrap()
                    .has_cpu_display_storage()
            );
            let token = owner.prepare(&project, &server, &mut ui, &ctx).unwrap();
            let repeated = owner.prepare(&project, &server, &mut ui, &ctx).unwrap();
            assert_eq!(token.receipt(), repeated.receipt());
            if owner.presented.is_none() {
                assert!(
                    owner
                        .color(&id, Vec2::new(70, 42), &project, &server, &mut ui, &ctx)
                        .is_err()
                );
            }
            owner.presented(Some(token));
            assert_eq!(
                owner.color(&id, Vec2::new(70, 42), &project, &server, &mut ui, &ctx),
                Ok(sample(size.x, size.y, 70, 42))
            );
            assert_eq!(
                owner.color(&id, Vec2::new(-1, 42), &project, &server, &mut ui, &ctx),
                Ok(None)
            );
            let mut reference = TheRGBABuffer::new(TheDim::sized(size.x, size.y));
            reference.render_hsl_hue_waveform();
            let closest = reference.find_closest_color_position(color).unwrap();
            reference
                .draw_rect_outline(&TheDim::rect(closest.x - 4, closest.y - 4, 8, 8), &[255; 4]);
            cases.push((
                "palette minimap",
                owner.current.as_ref().unwrap().clone() as Arc<dyn RenderViewPaintSource>,
                reference,
            ));
        }
        project.palette.current_index += 1;
        assert!(
            owner
                .color(&id, Vec2::zero(), &project, &server, &mut ui, &ctx)
                .is_err()
        );
        let old = owner.prepare(&project, &server, &mut ui, &ctx).unwrap();
        ctx.scale_factor = 1.5;
        owner.prepare(&project, &server, &mut ui, &ctx).unwrap();
        owner.presented(Some(old));
        assert!(
            owner
                .color(&id, Vec2::zero(), &project, &server, &mut ui, &ctx)
                .is_err()
        );
        let token = owner.prepare(&project, &server, &mut ui, &ctx).unwrap();
        owner.presented(Some(token));
        let mut document = ServerContext::new();
        document.palette_tool_active = true;
        assert!(
            owner
                .color(&id, Vec2::zero(), &project, &document, &mut ui, &ctx)
                .is_err()
        );
        // Exercise the real Sidebar update and grouped authoring input route.
        let mut sidebar = Sidebar::new();
        assert!(sidebar.handle_event(
            &TheEvent::Custom(TheId::named("Soft Update Minimap"), TheValue::Empty),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server
        ));
        let token = sidebar
            .minimap_palette
            .prepare(&project, &server, &mut ui, &ctx)
            .unwrap();
        sidebar.minimap_palette.presented(Some(token));
        let expected = sample(size.x, size.y, 70, 42).unwrap();
        sidebar.handle_event(
            &TheEvent::RenderViewClicked(id.clone(), Vec2::new(70, 42)),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        assert_eq!(
            project.palette.get_current_color().unwrap().to_u8_array(),
            expected
        );
        assert!(
            !ui.get_render_view("MiniMap")
                .unwrap()
                .has_cpu_display_storage()
        );
        sidebar.minimap_palette.deactivate(&mut ui);
    }
    cases
}
