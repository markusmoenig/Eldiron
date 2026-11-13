//! A browser text service for the shared host. The canvas is not editable and
//! Winit's web IME methods are no-ops. An invisible, caret-positioned textarea
//! receives browser composition; the existing controller still owns all text,
//! selection, undo and painting. No DOM text or pixels become document storage.
use super::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{
    CompositionEvent, Event, HtmlCanvasElement, HtmlTextAreaElement, InputEvent as DomInput,
    KeyboardEvent,
};

#[derive(Default)]
struct State {
    input: Option<TextInputState>,
    composing: Option<TextInputTarget>,
}
pub(super) struct BrowserText {
    textarea: HtmlTextAreaElement,
    canvas: HtmlCanvasElement,
    state: Rc<RefCell<State>>,
    listeners: Vec<(&'static str, Closure<dyn FnMut(Event)>)>,
    proxy: EventLoopProxy<HostEvent>,
}
impl BrowserText {
    pub fn new(canvas: HtmlCanvasElement, proxy: EventLoopProxy<HostEvent>) -> Result<Self> {
        let document = canvas
            .owner_document()
            .ok_or_else(|| RenderError("canvas document unavailable".into()))?;
        let textarea = document
            .create_element("textarea")
            .map_err(dom_error)?
            .dyn_into::<HtmlTextAreaElement>()
            .map_err(|error| dom_error(error.into()))?;
        textarea
            .set_attribute("aria-label", "Eldiron text input")
            .map_err(dom_error)?;
        textarea
            .set_attribute("autocomplete", "off")
            .map_err(dom_error)?;
        textarea
            .set_attribute("autocorrect", "off")
            .map_err(dom_error)?;
        textarea
            .set_attribute("autocapitalize", "off")
            .map_err(dom_error)?;
        textarea.set_spellcheck(false);
        textarea.set_tab_index(-1);
        textarea.set_disabled(true);
        textarea
            .set_attribute("style", "position:fixed;left:0;top:0;width:1px;height:1px;opacity:0;pointer-events:none;z-index:-1")
            .map_err(dom_error)?;
        document
            .body()
            .ok_or_else(|| RenderError("canvas body unavailable".into()))?
            .append_child(&textarea)
            .map_err(dom_error)?;
        let mut bridge = Self {
            textarea,
            canvas,
            state: Rc::default(),
            listeners: Vec::new(),
            proxy,
        };
        bridge.install()?;
        Ok(bridge)
    }
    pub fn active(&self) -> bool {
        self.state.borrow().input.is_some()
    }
    pub fn owns_focus(&self) -> bool {
        self.textarea
            .owner_document()
            .and_then(|d| d.active_element())
            .is_some_and(|active| active.is_same_node(Some(self.textarea.as_ref())))
    }
    /// Opt-in CPU source inspection for platform validation. Normal pages do
    /// not request or publish text contents; enabling this does not edit them.
    pub fn inspection_requested(&self) -> bool {
        self.textarea
            .owner_document()
            .and_then(|d| d.document_element())
            .is_some_and(|root| root.has_attribute("data-eldiron-inspect-text"))
    }
    pub fn inspect(&self, source: Option<&crate::text_edit::NativeTextSnapshot>) {
        if let Some(source) = source {
            let state = serde_json::json!({
                "target": source.target.raw(), "revision": source.revision,
                "text": source.text.as_ref(), "selection": source.selection,
                "composition": source.composition.as_ref().map(|r| [r.start, r.end]),
            });
            let _ = self
                .textarea
                .set_attribute("data-eldiron-text-source", &state.to_string());
        } else {
            let _ = self.textarea.remove_attribute("data-eldiron-text-source");
        }
    }
    pub fn sync(&mut self, input: Option<TextInputState>) {
        let (old, cancelled) = {
            let mut state = self.state.borrow_mut();
            let old = state.input;
            let cancelled = (old.map(|v| v.target) != input.map(|v| v.target))
                .then(|| state.composing.take())
                .flatten();
            state.input = input;
            (old, cancelled)
        };
        if let Some(target) = cancelled {
            send(
                &self.proxy,
                InputEvent::CompositionFor {
                    target,
                    text: String::new(),
                    cursor: None,
                },
            );
        }
        if input.is_none() && self.owns_focus() {
            let _ = self.canvas.focus();
        }
        if old != input {
            if old.map(|v| v.target) != input.map(|v| v.target) {
                self.textarea.set_value("");
            }
            self.textarea.set_disabled(input.is_none());
            if let Some(input) = input {
                let area = input.area;
                // Candidate positioning uses logical CSS coordinates, just as
                // the paint/input contract. The browser handles device density.
                let _ = self.textarea.set_attribute("style", &format!(
                    "position:fixed;left:{}px;top:{}px;width:1px;height:{}px;opacity:0;border:0;padding:0;margin:0;resize:none;overflow:hidden;pointer-events:none;z-index:-1",
                    area.x, area.y, area.height.max(1.)
                ));
            }
        }
        if input.is_some() {
            // A canvas pointer gesture can return DOM focus to the canvas while
            // the same field remains active. Restore the input service then.
            if !self.owns_focus() {
                let _ = self.textarea.focus();
            }
        } else if self.owns_focus() {
            let _ = self.canvas.focus();
        }
    }
    fn listen(&mut self, name: &'static str, listener: impl FnMut(Event) + 'static) -> Result<()> {
        let listener = Closure::wrap(Box::new(listener) as Box<dyn FnMut(Event)>);
        self.textarea
            .add_event_listener_with_callback(name, listener.as_ref().unchecked_ref())
            .map_err(dom_error)?;
        self.listeners.push((name, listener));
        Ok(())
    }
    fn install(&mut self) -> Result<()> {
        for name in ["keydown", "keyup"] {
            let (state, proxy) = (self.state.clone(), self.proxy.clone());
            self.listen(name, move |event| {
                let Ok(key) = event.dyn_into::<KeyboardEvent>() else {
                    return;
                };
                if state.borrow().input.is_none()
                    || key.is_composing()
                    || key.key_code() == 229
                    || state.borrow().composing.is_some()
                {
                    return;
                }
                // A web key can include its modifier transition. Queue the
                // neutral modifier snapshot before the key, independently of
                // Winit's key-before-modifiers browser event ordering.
                send(
                    &proxy,
                    InputEvent::Modifiers(Modifiers {
                        shift: key.shift_key(),
                        control: key.ctrl_key(),
                        alt: key.alt_key(),
                        super_key: key.meta_key(),
                    }),
                );
                send(
                    &proxy,
                    InputEvent::Key {
                        physical: physical_key(&key.code()),
                        logical: logical_key(&key.key()),
                        pressed: name == "keydown",
                        repeat: key.repeat(),
                    },
                );
                // Controller edits/navigation and clipboard/undo shortcuts
                // must not also mutate the textarea or trigger browser actions.
                if matches!(
                    key.key().as_str(),
                    "Backspace"
                        | "Delete"
                        | "Enter"
                        | "Tab"
                        | "Escape"
                        | "ArrowLeft"
                        | "ArrowRight"
                        | "ArrowUp"
                        | "ArrowDown"
                        | "Home"
                        | "End"
                        | "PageUp"
                        | "PageDown"
                ) || key.meta_key()
                    || (key.ctrl_key() && !key.alt_key())
                {
                    key.prevent_default();
                }
            })?;
        }
        let state = self.state.clone();
        self.listen("compositionstart", move |_| {
            let mut state = state.borrow_mut();
            state.composing = state.input.map(|v| v.target);
        })?;
        let (state, proxy) = (self.state.clone(), self.proxy.clone());
        self.listen("compositionupdate", move |event| {
            let Ok(event) = event.dyn_into::<CompositionEvent>() else {
                return;
            };
            let target = state.borrow().composing;
            if let Some(target) = target {
                let text = event.data().unwrap_or_default();
                send(
                    &proxy,
                    InputEvent::CompositionFor {
                        target,
                        text,
                        // compositionupdate precedes the DOM selection update.
                        // The following input event supplies its actual range.
                        cursor: None,
                    },
                );
            }
        })?;
        let (state, proxy, textarea) = (
            self.state.clone(),
            self.proxy.clone(),
            self.textarea.clone(),
        );
        self.listen("compositionend", move |event| {
            let Ok(event) = event.dyn_into::<CompositionEvent>() else {
                return;
            };
            let target = state.borrow_mut().composing.take();
            if let Some(target) = target {
                let text = event.data().unwrap_or_default();
                if text.is_empty() {
                    send(
                        &proxy,
                        InputEvent::CompositionFor {
                            target,
                            text,
                            cursor: None,
                        },
                    );
                } else {
                    // Committed text replaces the controller's marked range as
                    // one undo transaction. Clearing first would cancel it.
                    send(&proxy, InputEvent::TextFor { target, text });
                }
            }
            textarea.set_value("");
        })?;
        let (state, proxy, textarea) = (
            self.state.clone(),
            self.proxy.clone(),
            self.textarea.clone(),
        );
        self.listen("input", move |event| {
            let Ok(event) = event.dyn_into::<DomInput>() else {
                return;
            };
            let (input, composing) = {
                let state = state.borrow();
                (state.input, state.composing)
            };
            if let Some(target) = composing {
                let text = event.data().unwrap_or_else(|| textarea.value());
                let cursor = textarea
                    .selection_start()
                    .ok()
                    .flatten()
                    .zip(textarea.selection_end().ok().flatten())
                    .and_then(|(start, end)| {
                        Some((
                            crate::text_edit::utf16_byte_offset(&text, start as usize)?,
                            crate::text_edit::utf16_byte_offset(&text, end as usize)?,
                        ))
                    });
                send(
                    &proxy,
                    InputEvent::CompositionFor {
                        target,
                        text,
                        cursor,
                    },
                );
                return;
            }
            if event.is_composing()
                || matches!(
                    event.input_type().as_str(),
                    "insertCompositionText" | "insertFromComposition"
                )
            {
                return;
            }
            if let Some(input) = input {
                let text = event.data().unwrap_or_else(|| textarea.value());
                if !text.is_empty() {
                    send(
                        &proxy,
                        InputEvent::TextFor {
                            target: input.target,
                            text,
                        },
                    );
                }
            }
            textarea.set_value("");
        })?;
        let (state, proxy) = (self.state.clone(), self.proxy.clone());
        self.listen("beforeinput", move |event| {
            let Ok(event) = event.dyn_into::<DomInput>() else {
                return;
            };
            if state.borrow().input.is_none() || event.is_composing() {
                return;
            }
            // Virtual keyboards need not emit physical Backspace/Delete keys.
            let key = match event.input_type().as_str() {
                "deleteContentBackward" => Some(PhysicalKey::Backspace),
                "deleteContentForward" => Some(PhysicalKey::Delete),
                _ => None,
            };
            if let Some(physical) = key {
                event.prevent_default();
                for pressed in [true, false] {
                    send(
                        &proxy,
                        InputEvent::Key {
                            physical,
                            logical: LogicalKey::Other,
                            pressed,
                            repeat: false,
                        },
                    );
                }
            }
        })?;
        let (canvas, proxy) = (self.canvas.clone(), self.proxy.clone());
        self.listen("blur", move |event| {
            let on_canvas = event
                .dyn_ref::<web_sys::FocusEvent>()
                .and_then(|e| e.related_target())
                .is_some_and(|target| target == canvas.clone().into());
            if !on_canvas {
                send(&proxy, InputEvent::Focus(false));
            }
        })?;
        Ok(())
    }
}
impl Drop for BrowserText {
    fn drop(&mut self) {
        for (name, listener) in &self.listeners {
            let _ = self
                .textarea
                .remove_event_listener_with_callback(name, listener.as_ref().unchecked_ref());
        }
        self.textarea.remove();
    }
}
fn send(proxy: &EventLoopProxy<HostEvent>, event: InputEvent) {
    let _ = proxy.send_event(HostEvent::BrowserInput(event));
}
fn dom_error(error: wasm_bindgen::JsValue) -> RenderError {
    RenderError(format!("browser text service: {error:?}"))
}

fn physical_key(code: &str) -> PhysicalKey {
    use PhysicalKey as P;
    if let Some(letter) = code.strip_prefix("Key").filter(|v| v.len() == 1) {
        return P::Letter(letter.as_bytes()[0]);
    }
    if let Some(digit) = code
        .strip_prefix("Digit")
        .and_then(|v| v.parse::<u8>().ok())
        .filter(|v| *v < 10)
    {
        return P::Digit(digit);
    }
    if let Some(function) = code
        .strip_prefix('F')
        .and_then(|v| v.parse::<u8>().ok())
        .filter(|v| (1..=35).contains(v))
    {
        return P::Function(function);
    }
    match code {
        "Escape" => P::Escape,
        "Enter" | "NumpadEnter" => P::Enter,
        "Space" => P::Space,
        "Tab" => P::Tab,
        "Backspace" => P::Backspace,
        "Delete" => P::Delete,
        "ArrowLeft" => P::Left,
        "ArrowRight" => P::Right,
        "ArrowUp" => P::Up,
        "ArrowDown" => P::Down,
        "Home" => P::Home,
        "End" => P::End,
        "PageUp" => P::PageUp,
        "PageDown" => P::PageDown,
        "ShiftLeft" => P::ShiftLeft,
        "ShiftRight" => P::ShiftRight,
        "ControlLeft" => P::ControlLeft,
        "ControlRight" => P::ControlRight,
        "AltLeft" => P::AltLeft,
        "AltRight" => P::AltRight,
        "MetaLeft" => P::SuperLeft,
        "MetaRight" => P::SuperRight,
        _ => P::Other(0),
    }
}
fn logical_key(key: &str) -> LogicalKey {
    use crate::TheKeyCode as K;
    let named = match key {
        "Escape" => Some(K::Escape),
        "Enter" => Some(K::Return),
        "Backspace" => Some(K::Delete),
        "Home" => Some(K::Home),
        "End" => Some(K::End),
        "ArrowLeft" => Some(K::Left),
        "ArrowRight" => Some(K::Right),
        "ArrowUp" => Some(K::Up),
        "ArrowDown" => Some(K::Down),
        "Tab" => Some(K::Tab),
        _ => None,
    };
    if let Some(key) = named {
        LogicalKey::Named(key)
    } else if key.chars().count() == 1 {
        LogicalKey::Character(key.into())
    } else {
        LogicalKey::Other
    }
}
