//! Small host service adapters. Browser services use Web APIs rather than
//! linking desktop process launchers into the WASM application.
pub(crate) fn open_url(url: &str) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        open::that(url).map_err(|error| error.to_string())
    }
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .ok_or("Browser window unavailable")?
            .open_with_url_and_target_and_features(url, "_blank", "noopener,noreferrer")
            .map(|_| ())
            .map_err(|error| format!("Unable to open URL: {error:?}"))
    }
}

#[cfg(target_arch = "wasm32")]
struct FetchTimer {
    window: web_sys::Window,
    controller: web_sys::AbortController,
    id: i32,
    _callback: wasm_bindgen::closure::Closure<dyn FnMut()>,
}
#[cfg(target_arch = "wasm32")]
impl Drop for FetchTimer {
    fn drop(&mut self) {
        self.window.clear_timeout_with_handle(self.id);
        self.controller.abort();
    }
}
#[cfg(target_arch = "wasm32")]
pub(crate) async fn fetch_bytes(url: &str, limit: usize) -> Result<Vec<u8>, String> {
    use wasm_bindgen::{JsCast, closure::Closure};
    use wasm_bindgen_futures::JsFuture;
    let window = web_sys::window().ok_or("Browser window unavailable")?;
    let controller =
        web_sys::AbortController::new().map_err(|e| format!("Unable to start download: {e:?}"))?;
    let abort = controller.clone();
    let callback = Closure::once(move || abort.abort());
    let id = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            15_000,
        )
        .map_err(|e| format!("Unable to start download: {e:?}"))?;
    let _timer = FetchTimer {
        window: window.clone(),
        controller: controller.clone(),
        id,
        _callback: callback,
    };
    let options = web_sys::RequestInit::new();
    options.set_signal(Some(&controller.signal()));
    options.set_cache(web_sys::RequestCache::NoStore);
    let response = JsFuture::from(window.fetch_with_str_and_init(url, &options))
        .await
        .map_err(|e| format!("Fetch failed: {e:?}"))?
        .dyn_into::<web_sys::Response>()
        .map_err(|_| "Invalid fetch response")?;
    if !response.ok() {
        return Err(format!("HTTP {} while loading {url}", response.status()));
    }
    if response
        .headers()
        .get("Content-Length")
        .ok()
        .flatten()
        .and_then(|v| v.parse::<usize>().ok())
        .is_some_and(|len| len > limit)
    {
        controller.abort();
        return Err("Download exceeds the document/asset size limit".into());
    }
    let reader = response
        .body()
        .ok_or("Empty download response")?
        .get_reader()
        .dyn_into::<web_sys::ReadableStreamDefaultReader>()
        .map_err(|_| "Unable to read download stream")?;
    let mut bytes: Vec<u8> = Vec::new();
    loop {
        let item = JsFuture::from(reader.read())
            .await
            .map_err(|e| format!("Unable to read download: {e:?}"))?;
        if js_sys::Reflect::get(&item, &"done".into())
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            break;
        }
        let chunk = js_sys::Reflect::get(&item, &"value".into())
            .map_err(|_| "Invalid download chunk")?
            .dyn_into::<js_sys::Uint8Array>()
            .map_err(|_| "Invalid download chunk")?;
        let Some(next) = bytes
            .len()
            .checked_add(chunk.length() as usize)
            .filter(|len| *len <= limit)
        else {
            controller.abort();
            let _ = JsFuture::from(reader.cancel()).await;
            return Err("Download exceeds the document/asset size limit".into());
        };
        if next > bytes.capacity() {
            let capacity = bytes.capacity().saturating_mul(2).max(next).min(limit);
            bytes
                .try_reserve_exact(capacity - bytes.len())
                .map_err(|_| "Not enough memory for the download")?;
        }
        let old = bytes.len();
        bytes.resize(next, 0);
        chunk.copy_to(&mut bytes[old..]);
    }
    Ok(bytes)
}
