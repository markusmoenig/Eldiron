//! Browser file services own DOM/JavaScript objects and return addressed CPU
//! results. A download fallback cannot confirm a disk write and never cleans it.
use super::*;
use std::sync::mpsc::{self, Receiver, Sender};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use theframework_core::Uuid;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;
type Application = UiApplicationAdapter<NativeCreatorApplication>;
enum Completed {
    Open(Uuid, std::result::Result<(Vec<u8>, String), String>),
    Save(
        Uuid,
        u64,
        bool,
        std::result::Result<Option<SavedFile>, String>,
    ),
}
struct SavedFile {
    name: String,
    handle: JsValue,
}
pub(super) struct Files {
    input: web_sys::HtmlInputElement,
    dirty: Rc<Cell<bool>>,
    before_unload: Closure<dyn FnMut(web_sys::BeforeUnloadEvent)>,
    change: Closure<dyn FnMut(web_sys::Event)>,
    cancel: Closure<dyn FnMut(web_sys::Event)>,
    pending_open: Rc<RefCell<Option<Uuid>>>,
    done: Receiver<Completed>,
    send: Sender<Completed>,
    handles: std::collections::HashMap<Uuid, JsValue>,
    pending_save: Option<u64>,
}
impl Files {
    pub fn new() -> std::result::Result<Self, JsValue> {
        let document = web_sys::window().unwrap().document().unwrap();
        let input = document
            .create_element("input")?
            .dyn_into::<web_sys::HtmlInputElement>()?;
        input.set_id("eldiron-project-file");
        input.set_type("file");
        input.set_accept(".eldiron");
        input.set_attribute("hidden", "")?;
        document.body().unwrap().append_child(&input)?;
        let dirty = Rc::new(Cell::new(false));
        let state = dirty.clone();
        let before_unload = Closure::wrap(Box::new(move |event: web_sys::BeforeUnloadEvent| {
            if state.get() {
                event.prevent_default();
                event.set_return_value("");
            }
        }) as Box<dyn FnMut(web_sys::BeforeUnloadEvent)>);
        web_sys::window()
            .unwrap()
            .add_event_listener_with_callback(
                "beforeunload",
                before_unload.as_ref().unchecked_ref(),
            )?;
        let (send, done) = mpsc::channel();
        let pending_open = Rc::new(RefCell::new(None));
        let pending = pending_open.clone();
        let picker = input.clone();
        let tx = send.clone();
        let change = Closure::wrap(Box::new(move |_: web_sys::Event| {
            let Some(document) = pending.borrow_mut().take() else {
                return;
            };
            let Some(file) = picker.files().and_then(|files| files.get(0)) else {
                return;
            };
            let tx = tx.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let result = async {
                    if file.size() > (1024 * 1024 * 1024) as f64 {
                        return Err("Project exceeds the size limit".into());
                    }
                    let buffer = JsFuture::from(file.array_buffer())
                        .await
                        .map_err(js_error)?;
                    Ok((js_sys::Uint8Array::new(&buffer).to_vec(), file.name()))
                }
                .await;
                let _ = tx.send(Completed::Open(document, result));
            });
        }) as Box<dyn FnMut(web_sys::Event)>);
        input.add_event_listener_with_callback("change", change.as_ref().unchecked_ref())?;
        let pending = pending_open.clone();
        let cancel = Closure::wrap(Box::new(move |_: web_sys::Event| {
            pending.borrow_mut().take();
        }) as Box<dyn FnMut(web_sys::Event)>);
        input.add_event_listener_with_callback("cancel", cancel.as_ref().unchecked_ref())?;
        Ok(Self {
            input,
            dirty,
            before_unload,
            change,
            cancel,
            pending_open,
            done,
            send,
            handles: Default::default(),
            pending_save: None,
        })
    }
    pub fn set_dirty(&self, dirty: bool) {
        self.dirty.set(dirty);
    }
    pub fn request(
        &mut self,
        kind: &str,
        document: Uuid,
        app: &mut Application,
    ) -> std::result::Result<(), String> {
        if app
            .application
            .host_document_info()
            .is_none_or(|info| info.document != document)
        {
            return Err("The document changed before the file request".into());
        }
        if kind == "open_document" {
            if self.pending_open.borrow().is_some() {
                return Err("A project file picker is already open".into());
            }
            *self.pending_open.borrow_mut() = Some(document);
            self.input.set_value("");
            self.input.click();
            return Ok(());
        }
        if self.pending_save.is_some() {
            return Err("A project save is already pending".into());
        }
        let public_export = kind == "export_public_client";
        let save = if public_export {
            app.application
                .host_prepare_public_client_export(&mut app.session, document)?
        } else {
            app.application
                .host_prepare_document_save(&mut app.session, document)?
        };
        let bytes = app
            .application
            .host_document_save_bytes(save.ticket)
            .ok_or("Save snapshot was retired")?
            .to_vec();
        let mut name = app
            .application
            .host_document_info()
            .and_then(|info| info.path)
            .and_then(|path| path.file_name().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "Untitled.eldiron".into());
        if public_export {
            name = format!(
                "{}.eldiron-client",
                std::path::Path::new(&name)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
            );
        }
        let send = self.send.clone();
        let handle = (!public_export && kind == "save_document")
            .then(|| self.handles.get(&document).cloned())
            .flatten();
        self.pending_save = Some(save.ticket);
        wasm_bindgen_futures::spawn_local(async move {
            let result = save_file(&bytes, &name, handle).await;
            let _ = send.send(Completed::Save(
                document,
                save.ticket,
                public_export,
                result,
            ));
        });
        Ok(())
    }
    pub fn apply(&mut self, app: &mut Application) {
        self.handles
            .retain(|document, _| app.application.host_has_document(*document));
        while let Ok(result) = self.done.try_recv() {
            let result = match result {
                Completed::Open(document, result) => result.and_then(|(bytes, name)| {
                    app.application
                        .host_load_document(
                            &mut app.session,
                            document,
                            &bytes,
                            Some(name.into()),
                            false,
                        )
                        .map(|_| ())
                }),
                Completed::Save(document, ticket, public_export, result) => {
                    if self.pending_save == Some(ticket) {
                        self.pending_save = None;
                    }
                    match result {
                        Ok(saved) => {
                            let confirmed = saved.is_some();
                            let completed = app.application.host_complete_document_save(
                                &mut app.session,
                                ticket,
                                confirmed,
                                saved.as_ref().map(|file| file.name.clone().into()),
                            );
                            if !public_export
                                && completed.as_ref().is_ok_and(|result| result.saved)
                                && let Some(saved) = saved
                            {
                                self.handles.insert(document, saved.handle);
                            }
                            if !confirmed {
                                app.session.context.ui.send(TheEvent::SetStatusText(TheId::empty(), "Download requested. This browser cannot confirm the file write; the document keeps its saved state.".into()));
                            }
                            completed.map(|_| ())
                        }
                        Err(error) => {
                            let _ = app.application.host_complete_document_save(
                                &mut app.session,
                                ticket,
                                false,
                                None,
                            );
                            Err(error)
                        }
                    }
                }
            };
            if let Err(error) = result {
                app.session
                    .context
                    .ui
                    .send(TheEvent::SetStatusText(TheId::empty(), error));
            }
            app.session.context.ui.invalidate_paint();
        }
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        if let Some(window) = web_sys::window() {
            let _ = window.remove_event_listener_with_callback(
                "beforeunload",
                self.before_unload.as_ref().unchecked_ref(),
            );
        }
        let _ = self
            .input
            .remove_event_listener_with_callback("change", self.change.as_ref().unchecked_ref());
        let _ = self
            .input
            .remove_event_listener_with_callback("cancel", self.cancel.as_ref().unchecked_ref());
        self.input.remove();
        self.pending_open.borrow_mut().take();
    }
}
fn js_error(error: impl Into<JsValue>) -> String {
    let error = error.into();
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}
async fn call(
    object: &JsValue,
    method: &str,
    argument: Option<&JsValue>,
) -> std::result::Result<JsValue, String> {
    let function = js_sys::Reflect::get(object, &method.into())
        .map_err(js_error)?
        .dyn_into::<js_sys::Function>()
        .map_err(js_error)?;
    let result = match argument {
        Some(argument) => function.call1(object, argument),
        None => function.call0(object),
    }
    .map_err(js_error)?;
    JsFuture::from(js_sys::Promise::resolve(&result))
        .await
        .map_err(js_error)
}
async fn save_file(
    bytes: &[u8],
    name: &str,
    existing: Option<JsValue>,
) -> std::result::Result<Option<SavedFile>, String> {
    let window = web_sys::window().ok_or("Browser window unavailable")?;
    let picker =
        js_sys::Reflect::get(window.as_ref(), &"showSaveFilePicker".into()).map_err(js_error)?;
    let bytes = js_sys::Uint8Array::from(bytes);
    if existing.is_some() || picker.is_function() {
        let options = js_sys::Object::new();
        js_sys::Reflect::set(&options, &"suggestedName".into(), &name.into()).map_err(js_error)?;
        let types = js_sys::Array::new();
        let file_type = js_sys::Object::new();
        let accept = js_sys::Object::new();
        let extensions = js_sys::Array::new();
        extensions.push(
            &if name.ends_with(".eldiron-client") {
                ".eldiron-client"
            } else {
                ".eldiron"
            }
            .into(),
        );
        js_sys::Reflect::set(&accept, &"application/x-eldiron".into(), &extensions)
            .map_err(js_error)?;
        js_sys::Reflect::set(&file_type, &"description".into(), &"Eldiron project".into())
            .map_err(js_error)?;
        js_sys::Reflect::set(&file_type, &"accept".into(), &accept).map_err(js_error)?;
        types.push(&file_type);
        js_sys::Reflect::set(&options, &"types".into(), &types).map_err(js_error)?;
        let handle = if let Some(handle) = existing {
            handle
        } else {
            call(
                window.as_ref(),
                "showSaveFilePicker",
                Some(options.as_ref()),
            )
            .await?
        };
        let selected_name = js_sys::Reflect::get(&handle, &"name".into())
            .map_err(js_error)?
            .as_string()
            .ok_or("Missing output filename")?;
        if name.ends_with(".eldiron-client") && !selected_name.ends_with(".eldiron-client") {
            return Err(
                "Public clients must use .eldiron-client; authored projects cannot be overwritten"
                    .into(),
            );
        }
        let writable = call(&handle, "createWritable", None).await?;
        if let Err(error) = call(&writable, "write", Some(bytes.as_ref())).await {
            let _ = call(&writable, "abort", None).await;
            return Err(error);
        }
        if let Err(error) = call(&writable, "close", None).await {
            let _ = call(&writable, "abort", None).await;
            return Err(error);
        }
        return Ok(Some(SavedFile {
            name: js_sys::Reflect::get(&handle, &"name".into())
                .map_err(js_error)?
                .as_string()
                .unwrap_or_else(|| name.into()),
            handle,
        }));
    }
    let parts = js_sys::Array::new();
    parts.push(bytes.as_ref());
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("application/x-eldiron");
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options)
        .map_err(js_error)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(js_error)?;
    let document = window.document().ok_or("Browser document unavailable")?;
    let link = document
        .create_element("a")
        .map_err(js_error)?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(js_error)?;
    link.set_href(&url);
    link.set_download(name);
    link.set_hidden(true);
    document
        .body()
        .ok_or("Browser document body unavailable")?
        .append_child(&link)
        .map_err(js_error)?;
    link.click();
    link.remove();
    let retire = Closure::once_into_js(move || {
        let _ = web_sys::Url::revoke_object_url(&url);
    });
    let _ =
        window.set_timeout_with_callback_and_timeout_and_arguments_0(retire.unchecked_ref(), 1000);
    Ok(None)
}
