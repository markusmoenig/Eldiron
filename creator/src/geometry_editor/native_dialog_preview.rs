//! Modal previews retain original evaluated values and publish a GPU card. The
//! geometry host owns registry retirement independently of modal/controller drop.
use super::*;
use renderkit::{GpuImage, ImageHandle, ImageRegistry};
use std::sync::Arc;
use theframework::paint::{Rect, Sampling};
use theframework::theui::{
    paint::{UiPaintResult, WidgetPainter},
    thewidget::therenderview::RenderViewPaintSource,
};
pub(super) mod fitting;
#[cfg(test)]
mod tests;

pub(super) struct Request {
    pub value: Option<Value>,
    pub palette: ThePalette,
    pub max_side: i32,
}
struct Source {
    image: Option<ImageHandle>,
}
impl RenderViewPaintSource for Source {
    fn paint(&self, painter: &mut WidgetPainter<'_>, viewport: [f32; 2]) -> UiPaintResult {
        if let Some(image) = self.image {
            painter.list.image(
                image,
                Rect::new(0., 0., viewport[0], viewport[1]),
                Sampling::Nearest,
            );
        } else {
            painter.list.rect(
                Rect::new(0., 0., viewport[0], viewport[1]),
                0.,
                [30. / 255., 32. / 255., 35. / 255., 1.],
            );
        }
        Ok(())
    }
}
pub(super) fn pending(ui: &mut TheUI, view: &str) {
    if let Some(view) = ui.get_render_view(view) {
        view.set_paint_source(Some(Arc::new(Source { image: None })));
    }
}
#[derive(Default)]
pub(super) struct Preview {
    owner: Option<u64>,
    request: Option<Arc<Request>>,
    image: Option<ImageHandle>,
    card: Option<Arc<GpuImage>>,
    size: Option<(u32, u32)>,
    fitter: Option<fitting::Fitter>,
    source: Option<Arc<dyn RenderViewPaintSource>>,
    renderer: Option<super::native_thumbnails::Renderer>,
}
impl Preview {
    fn clear(&mut self, images: &mut ImageRegistry) {
        if self.owner == Some(images.owner_id())
            && let Some(image) = self.image.take()
        {
            images.remove(image);
        }
        self.image = None;
        self.source = None;
        self.request = None;
        self.renderer = None;
        self.card = None;
        self.size = None;
        self.fitter = None;
    }
    pub fn prepare(
        &mut self,
        request: Option<&Arc<Request>>,
        view: &str,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        images: &mut ImageRegistry,
    ) -> Result<(), String> {
        if self.owner != Some(images.owner_id()) {
            self.image = None;
            self.clear(images);
            self.owner = Some(images.owner_id());
        }
        let Some(request) = request.filter(|_| ui.get_render_view(view).is_some()) else {
            self.clear(images);
            if let Some(view) = ui.get_render_view(view) {
                view.set_paint_source(None);
            }
            return Ok(());
        };
        let dim = *ui.get_render_view(view).unwrap().dim();
        let size = (dim.width.max(1) as u32, dim.height.max(1) as u32);
        let changed = self
            .request
            .as_ref()
            .is_none_or(|old| !Arc::ptr_eq(old, request));
        if changed {
            self.card = None;
            if let Some(value) = &request.value {
                if self.renderer.is_none() {
                    self.renderer = Some(super::native_thumbnails::Renderer::new(
                        images.context().clone(),
                    )?);
                }
                self.card = Some(self.renderer.as_mut().unwrap().render_value(
                    value,
                    &request.palette,
                    &TheDarkTheme::new(),
                    &mut ctx.text,
                    images,
                )?);
            }
        }
        if changed || self.size != Some(size) {
            let next = if let Some(card) = &self.card {
                if self.fitter.is_none() {
                    self.fitter = Some(fitting::Fitter::new(images.context().clone()));
                }
                let image = self
                    .fitter
                    .as_ref()
                    .unwrap()
                    .render(card, size, request.max_side)?;
                Some(images.insert(image).map_err(|error| error.to_string())?)
            } else {
                None
            };
            if let Some(old) = std::mem::replace(&mut self.image, next) {
                images.remove(old);
            }
            self.source = Some(Arc::new(Source { image: self.image }));
            self.request = Some(request.clone());
            self.size = Some(size);
        }
        ui.get_render_view(view)
            .unwrap()
            .set_paint_source(self.source.clone());
        Ok(())
    }
}
