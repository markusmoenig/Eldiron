//! Visible recipe icons are immutable GPU sources owned by the geometry host.
//! Filter/controller teardown retires handles; CPU requests survive device loss.
use super::{
    native_dialog_preview::{Request, fitting::Fitter},
    native_thumbnails::Renderer,
    *,
};
use renderkit::{ImageHandle, ImageRegistry};
use std::{collections::HashMap, sync::Arc};
#[cfg(test)]
mod tests;
pub(super) type Requests = HashMap<Uuid, Arc<Request>>;
struct Published {
    request: Arc<Request>,
    image: Option<ImageHandle>,
}
#[derive(Default)]
pub(super) struct Icons {
    owner: Option<u64>,
    entries: HashMap<Uuid, Published>,
    renderer: Option<Renderer>,
    fitter: Option<Fitter>,
}
impl Icons {
    pub fn prepare(
        &mut self,
        requests: Option<&Requests>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        images: &mut ImageRegistry,
    ) -> (bool, Vec<String>) {
        if self.owner != Some(images.owner_id()) {
            for id in self.entries.keys() {
                if let Some(item) = ui.get_widget_id(*id).and_then(|w| w.as_list_item()) {
                    item.set_icon_handle(None);
                }
            }
            self.entries.clear();
            self.renderer = None;
            self.fitter = None;
            self.owner = Some(images.owner_id());
        }
        self.entries.retain(|id, old| {
            let keep = requests
                .and_then(|r| r.get(id))
                .is_some_and(|r| Arc::ptr_eq(r, &old.request))
                && ui
                    .get_widget_id(*id)
                    .and_then(|w| w.as_list_item())
                    .is_some();
            if !keep {
                if let Some(item) = ui.get_widget_id(*id).and_then(|w| w.as_list_item()) {
                    item.set_icon_handle(None);
                }
                if let Some(image) = old.image {
                    images.remove(image);
                }
            }
            keep
        });
        let mut errors = Vec::new();
        if let Some((id, request)) = requests.into_iter().flat_map(|r| r.iter()).find(|(id, _)| {
            !self.entries.contains_key(*id)
                && ui
                    .get_widget_id(**id)
                    .and_then(|w| w.as_list_item())
                    .is_some()
        }) {
            let result = (|| {
                let Some(value) = &request.value else {
                    return Ok(None);
                };
                if self.renderer.is_none() {
                    self.renderer = Some(Renderer::new(images.context().clone())?);
                }
                if self.fitter.is_none() {
                    self.fitter = Some(Fitter::new(images.context().clone()));
                }
                let source = self.renderer.as_mut().unwrap().render_value(
                    value,
                    &request.palette,
                    &TheDarkTheme::new(),
                    &mut ctx.text,
                    images,
                )?;
                let fitted = self
                    .fitter
                    .as_ref()
                    .unwrap()
                    .render(&source, (40, 40), 40)?;
                images.insert(fitted).map(Some).map_err(|e| e.to_string())
            })();
            let image = match result {
                Ok(image) => image,
                Err(error) => {
                    errors.push(format!("Recipe icon: {error}"));
                    None
                }
            };
            self.entries.insert(
                *id,
                Published {
                    request: request.clone(),
                    image,
                },
            );
        }
        for (id, published) in &self.entries {
            if let Some(item) = ui.get_widget_id(*id).and_then(|w| w.as_list_item()) {
                item.set_icon_handle(published.image);
            }
        }
        let waiting = requests.into_iter().flat_map(|r| r.keys()).any(|id| {
            !self.entries.contains_key(id)
                && ui
                    .get_widget_id(*id)
                    .and_then(|w| w.as_list_item())
                    .is_some()
        });
        if self.entries.is_empty() && !waiting {
            self.renderer = None;
            self.fitter = None;
        }
        (waiting, errors)
    }
}
