//! One owned, cancellable verified content download. Dropping the owner cancels
//! transport work, including browser fetches after a reload/disconnect.
use crate::*;
use std::sync::mpsc::{self, Receiver, TryRecvError};
pub(crate) struct ContentTask {
    receiver: Receiver<Result<PhysicsContent, String>>,
    #[cfg(not(target_arch = "wasm32"))]
    task: tokio::task::JoinHandle<()>,
    #[cfg(target_arch = "wasm32")]
    _abort: FetchAbort,
}
impl ContentTask {
    pub(crate) fn start(endpoint: &str, hint: &ContentHint) -> Result<Self, String> {
        let (sender, receiver) = mpsc::sync_channel(1);
        let endpoint = endpoint.to_owned();
        let hint = hint.clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let runtime = tokio::runtime::Handle::try_current()
                .map_err(|_| "Content downloads require an async runtime")?;
            let task = runtime.spawn(async move {
                let result = crate::native::fetch_content(&endpoint, &hint).await;
                let _ = sender.try_send(result);
            });
            Ok(Self { receiver, task })
        }
        #[cfg(target_arch = "wasm32")]
        {
            let abort = FetchAbort::new().map_err(js_error)?;
            let signal = abort.signal();
            wasm_bindgen_futures::spawn_local(async move {
                let result = crate::browser::fetch_content_with_signal(&endpoint, &hint, &signal)
                    .await
                    .map_err(js_error);
                let _ = sender.try_send(result);
            });
            Ok(Self {
                receiver,
                _abort: abort,
            })
        }
    }
    pub(crate) fn poll(&mut self) -> Option<Result<PhysicsContent, String>> {
        match self.receiver.try_recv() {
            Ok(value) => Some(value),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err("Content download stopped".into())),
        }
    }
}
impl Drop for ContentTask {
    fn drop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.task.abort();
    }
}
#[cfg(target_arch = "wasm32")]
fn js_error(error: wasm_bindgen::JsValue) -> String {
    error
        .as_string()
        .unwrap_or_else(|| "Content download failed or timed out".into())
}
#[cfg(target_arch = "wasm32")]
pub(crate) struct FetchAbort {
    controller: web_sys::AbortController,
    timer: i32,
    _timeout: wasm_bindgen::closure::Closure<dyn FnMut()>,
}
#[cfg(target_arch = "wasm32")]
impl FetchAbort {
    pub(crate) fn new() -> Result<Self, wasm_bindgen::JsValue> {
        use wasm_bindgen::JsCast;
        let controller = web_sys::AbortController::new()?;
        let abort = controller.clone();
        let timeout = wasm_bindgen::closure::Closure::wrap(
            Box::new(move || abort.abort()) as Box<dyn FnMut()>
        );
        let timer = web_sys::window()
            .ok_or_else(|| wasm_bindgen::JsValue::from_str("Browser window required"))?
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                timeout.as_ref().unchecked_ref(),
                10_000,
            )?;
        Ok(Self {
            controller,
            timer,
            _timeout: timeout,
        })
    }
    pub(crate) fn signal(&self) -> web_sys::AbortSignal {
        self.controller.signal()
    }
}
#[cfg(target_arch = "wasm32")]
impl Drop for FetchAbort {
    fn drop(&mut self) {
        if let Some(window) = web_sys::window() {
            window.clear_timeout_with_handle(self.timer);
        }
        self.controller.abort();
    }
}
