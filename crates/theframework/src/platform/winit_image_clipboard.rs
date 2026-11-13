//! Host-owned image clipboard transport; completion never mutates a document.
use super::{ClipboardState, HostEvent};
use crate::platform::{ImageClipboardRequest, ImageClipboardResult};
use winit::event_loop::EventLoopProxy;
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
pub(super) fn run(
    operation: ImageClipboardRequest,
    clipboard: &mut ClipboardState,
    proxy: EventLoopProxy<HostEvent>,
) {
    use crate::platform::image_clipboard::validate;
    if clipboard.is_none() {
        *clipboard = arboard::Clipboard::new().ok();
    }
    let (request, result) = match operation {
        ImageClipboardRequest::Read { request } => {
            let image = clipboard
                .as_mut()
                .and_then(|c| c.get_image().ok())
                .and_then(|image| {
                    if image.width > 8192
                        || image.height > 8192
                        || image.width.checked_mul(image.height)?.checked_mul(4)?
                            > crate::platform::image_clipboard::MAX_BYTES
                    {
                        return None;
                    }
                    let image = theframework_core::ImageAsset::rgba(
                        image.width as u32,
                        image.height as u32,
                        image.bytes.into_owned(),
                    )
                    .ok()?;
                    validate(&image).ok()?;
                    Some(image)
                });
            (request, ImageClipboardResult::Image(image))
        }
        ImageClipboardRequest::Write { request, image } => {
            let success = validate(&image).is_ok()
                && clipboard.as_mut().is_some_and(|c| {
                    c.set_image(arboard::ImageData {
                        width: image.width() as usize,
                        height: image.height() as usize,
                        bytes: std::borrow::Cow::Borrowed(image.pixels()),
                    })
                    .is_ok()
                });
            (request, ImageClipboardResult::Written(success))
        }
    };
    let _ = proxy.send_event(HostEvent::ClipboardImage(request, result));
}
#[cfg(target_arch = "wasm32")]
pub(super) fn run(
    operation: ImageClipboardRequest,
    clipboard: &mut ClipboardState,
    proxy: EventLoopProxy<HostEvent>,
) {
    // Clipboard promises cannot be aborted. Bound retained Rust image payloads
    // even if the browser's permission prompt or platform callback never settles.
    const MAX_IN_FLIGHT: usize = 2;
    if clipboard.image_in_flight.get() >= MAX_IN_FLIGHT {
        let (request, result) = match operation {
            ImageClipboardRequest::Read { request } => (request, ImageClipboardResult::Image(None)),
            ImageClipboardRequest::Write { request, .. } => {
                (request, ImageClipboardResult::Written(false))
            }
        };
        let _ = proxy.send_event(HostEvent::ClipboardImage(request, result));
        return;
    }
    struct Slot(std::rc::Rc<std::cell::Cell<usize>>);
    impl Drop for Slot {
        fn drop(&mut self) {
            self.0.set(self.0.get().saturating_sub(1));
        }
    }
    clipboard
        .image_in_flight
        .set(clipboard.image_in_flight.get() + 1);
    let slot = Slot(clipboard.image_in_flight.clone());
    wasm_bindgen_futures::spawn_local(async move {
        let _slot = slot;
        let (request, result) = match operation {
            ImageClipboardRequest::Read { request } => {
                (request, ImageClipboardResult::Image(web::read().await.ok()))
            }
            ImageClipboardRequest::Write { request, image } => (
                request,
                ImageClipboardResult::Written(web::write(image).await.is_ok()),
            ),
        };
        let _ = proxy.send_event(HostEvent::ClipboardImage(request, result));
    });
}
#[cfg(not(any(
    target_arch = "wasm32",
    target_os = "macos",
    target_os = "windows",
    target_os = "linux"
)))]
pub(super) fn run(
    operation: ImageClipboardRequest,
    _: &mut ClipboardState,
    proxy: EventLoopProxy<HostEvent>,
) {
    let (request, result) = match operation {
        ImageClipboardRequest::Read { request } => (request, ImageClipboardResult::Image(None)),
        ImageClipboardRequest::Write { request, .. } => {
            (request, ImageClipboardResult::Written(false))
        }
    };
    let _ = proxy.send_event(HostEvent::ClipboardImage(request, result));
}
#[cfg(target_arch = "wasm32")]
mod web {
    use crate::platform::image_clipboard::{MAX_BYTES, decode_png, encode_png};
    use theframework_core::ImageAsset;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;
    type Result<T> = std::result::Result<T, JsValue>;
    async fn call(object: &JsValue, method: &str, argument: Option<&JsValue>) -> Result<JsValue> {
        let function =
            js_sys::Reflect::get(object, &method.into())?.dyn_into::<js_sys::Function>()?;
        let result = if let Some(argument) = argument {
            function.call1(object, argument)?
        } else {
            function.call0(object)?
        };
        JsFuture::from(js_sys::Promise::resolve(&result)).await
    }
    fn clipboard() -> Result<JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("Browser unavailable"))?;
        js_sys::Reflect::get(window.navigator().as_ref(), &"clipboard".into())
    }
    pub async fn read() -> Result<ImageAsset> {
        let items = call(&clipboard()?, "read", None).await?;
        for item in js_sys::Array::from(&items) {
            let types = js_sys::Array::from(&js_sys::Reflect::get(&item, &"types".into())?);
            if types
                .iter()
                .any(|t| t.as_string().as_deref() == Some("image/png"))
            {
                let blob = call(&item, "getType", Some(&"image/png".into())).await?;
                let length = js_sys::Reflect::get(&blob, &"size".into())?
                    .as_f64()
                    .ok_or_else(|| JsValue::from_str("Invalid clipboard image"))?;
                if !length.is_finite() || length < 0. || length > MAX_BYTES as f64 {
                    return Err(JsValue::from_str("Clipboard PNG exceeds the byte limit"));
                }
                let buffer = call(&blob, "arrayBuffer", None).await?;
                let bytes = js_sys::Uint8Array::new(&buffer);
                if bytes.length() as usize > MAX_BYTES {
                    return Err(JsValue::from_str("Clipboard PNG exceeds the byte limit"));
                }
                return decode_png(&bytes.to_vec()).map_err(|e| JsValue::from_str(&e));
            }
        }
        Err(JsValue::from_str("Clipboard has no PNG image"))
    }
    pub async fn write(image: ImageAsset) -> Result<()> {
        let bytes = encode_png(&image).map_err(|e| JsValue::from_str(&e))?;
        let parts = js_sys::Array::new();
        parts.push(&js_sys::Uint8Array::from(bytes.as_slice()));
        let options = web_sys::BlobPropertyBag::new();
        options.set_type("image/png");
        let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options)?;
        let formats = js_sys::Object::new();
        js_sys::Reflect::set(&formats, &"image/png".into(), &blob)?;
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("Browser unavailable"))?;
        let constructor = js_sys::Reflect::get(window.as_ref(), &"ClipboardItem".into())?
            .dyn_into::<js_sys::Function>()?;
        let arguments = js_sys::Array::new();
        arguments.push(&formats);
        let item = js_sys::Reflect::construct(&constructor, &arguments)?;
        let items = js_sys::Array::new();
        items.push(&item);
        call(&clipboard()?, "write", Some(items.as_ref())).await?;
        Ok(())
    }
}
