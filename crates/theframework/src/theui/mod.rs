pub mod thecanvas;
pub mod thecodehighlighter;
pub mod thecollection;
pub mod thecontextmenu;
pub mod thedrop;
pub mod theflattenedmap;
pub mod theid;
pub mod thelayout;
pub mod thenodeui;
pub mod thergbbuffer;
pub mod thesdf;
pub mod thesizelimiter;
pub mod thestyle;
pub mod thetheme;
pub mod thetilemask;
pub mod thetimeline;
pub mod theuicontext;
pub mod theuiglobals;
pub mod theundo;
pub mod thevalue;
pub mod thevent;
pub mod thewidget;

use ::serde::de::{self, Deserializer};
use ::serde::ser::{self, Serializer};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, Sender};

fn compress<S>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).map_err(ser::Error::custom)?;
    let compressed_data = encoder.finish().map_err(ser::Error::custom)?;

    serializer.serialize_bytes(&compressed_data)
}

fn decompress<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let data = Vec::<u8>::deserialize(deserializer)?;
    let mut decoder = ZlibDecoder::new(&data[..]);
    let mut decompressed_data = Vec::new();
    decoder
        .read_to_end(&mut decompressed_data)
        .map_err(de::Error::custom)?;

    Ok(decompressed_data)
}

pub use crate::prelude::*;

pub type RGBA = [u8; 4];
pub const TRANSPARENT: RGBA = [0, 0, 0, 0];
pub const BLACK: RGBA = [0, 0, 0, 255];
pub const WHITE: RGBA = [255, 255, 255, 255];

pub mod prelude {
    pub use serde::{Deserialize, Serialize};

    pub use crate::theui::RGBA;

    pub use crate::theui::BLACK;
    pub use crate::theui::WHITE;

    pub use std::rc::Rc;

    pub use crate::theui::theid::TheId;

    pub use crate::theui::thecanvas::*;
    pub use crate::theui::thecodehighlighter::{TheCodeHighlighter, TheCodeHighlighterTrait};

    pub use crate::theui::thergbbuffer::TheRGBBuffer;
    pub use crate::theui::thesizelimiter::TheSizeLimiter;
    pub use crate::theui::theuicontext::*;
    pub use crate::theui::TheUI;

    pub use crate::theui::thevalue::{TheValue, TheValueAssignment, TheValueComparison};
    pub use crate::theui::thevent::TheEvent;

    pub use crate::theui::thewidget::prelude::*;
    pub use crate::theui::thewidget::thecolorbutton::*;

    pub use crate::theui::thestyle::prelude::*;
    pub use crate::theui::thestyle::TheStyle;

    pub use crate::theui::thetheme::prelude::*;
    pub use crate::theui::thetheme::{TheTheme, TheThemeColors, TheThemeColors::*};

    pub use crate::theui::thelayout::prelude::*;
    pub use crate::theui::thesdf::thepattern::ThePattern;
    pub use crate::theui::thesdf::thesdfcanvas::TheSDFCanvas;
    pub use crate::theui::thesdf::*;
    pub use crate::theui::thewidget::TheWidget;

    pub use crate::theui::thecontextmenu::*;
    pub use crate::theui::thedrop::*;
    pub use crate::theui::theuiglobals::*;
    pub use crate::theui::theundo::*;

    pub use crate::str;
    pub use crate::theui::thecollection::TheCollection;
    pub use crate::theui::theflattenedmap::{TheFlattenedMap, TheFlattenedMap3D};
    pub use crate::theui::thetilemask::TheTileMask;
    pub use crate::theui::thetimeline::{TheInterpolation, TheTimeline};
    pub use crate::theui::TheAccelerator;
    pub use crate::theui::TheAcceleratorKey;
    pub use crate::theui::TheDialogButtonRole;

    pub use crate::theui::thenodeui::*;
}

// Define a macro named `str!`.
#[macro_export]
macro_rules! str {
    ($x:expr) => {
        $x.to_string()
    };
}

bitflags::bitflags! {
    pub struct TheAcceleratorKey: u32 {
        /// Shift Key
        const SHIFT = 0b00000001;
        /// Ctrl Key / Control on Mac
        const CTRL = 0b00000010;
        /// Alt Key / Option on Mac
        const ALT = 0b00000100;
        /// Cmd on Mac
        const CMD = 0b00001000;

        /// CtrlAndCmd
        const CTRLCMD = Self::CTRL.bits() | Self::CMD.bits();

        /// The combination of `A`, `B`, and `C`.
        const ALL = Self::SHIFT.bits() | Self::CTRL.bits() | Self::ALT.bits() | Self::CMD.bits();
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
/// An accelerator for context menus and similar.
pub struct TheAccelerator {
    pub accel: TheAcceleratorKey,
    pub key: char,
}

impl TheAccelerator {
    pub fn new(accel: TheAcceleratorKey, key: char) -> Self {
        Self { accel, key }
    }

    /// Converts the accelerator to a string.
    pub fn description(&self) -> String {
        let mut str = "".to_string();

        if self.accel.contains(TheAcceleratorKey::SHIFT) {
            str += "Shift + ";
        }

        if self.accel.contains(TheAcceleratorKey::CTRLCMD) {
            if cfg!(target_os = "macos") {
                str += "Cmd + ";
            } else {
                str += "Ctrl + ";
            }
        }

        if self.accel.contains(TheAcceleratorKey::ALT) {
            if cfg!(target_os = "macos") {
                str += "Option + ";
            } else {
                str += "Alt + ";
            }
        }

        let mut s = str.to_string();
        s += &self.key.to_string().to_uppercase();

        s
    }

    /// Test if we match the given modifiers and key.
    pub fn matches(&self, shift: bool, ctrl: bool, alt: bool, logo: bool, key: char) -> bool {
        #[allow(clippy::if_same_then_else)]
        // We assume that accelerators are always case-insensitive.
        if self.key == key.to_ascii_lowercase() {
            if shift || ctrl || alt || logo {
                let mut ok = true;

                if (shift && !self.accel.contains(TheAcceleratorKey::SHIFT))
                    || (!shift && self.accel.contains(TheAcceleratorKey::SHIFT))
                {
                    ok = false;
                }
                // Check Ctrl: Allow for cases where either Ctrl or Cmd is part of CtrlCmd
                if (ctrl
                    && !(self.accel.contains(TheAcceleratorKey::CTRL)
                        || self.accel.contains(TheAcceleratorKey::CMD)))
                    || (!ctrl
                        && self.accel.contains(TheAcceleratorKey::CTRL)
                        && !self.accel.contains(TheAcceleratorKey::CMD))
                {
                    ok = false;
                }
                if alt && !self.accel.contains(TheAcceleratorKey::ALT) {
                    ok = false;
                }
                // Check Cmd (Logo): Allow for cases where either Ctrl or Cmd is part of CtrlCmd
                if (logo
                    && !(self.accel.contains(TheAcceleratorKey::CMD)
                        || self.accel.contains(TheAcceleratorKey::CTRL)))
                    || (!logo
                        && self.accel.contains(TheAcceleratorKey::CMD)
                        && !self.accel.contains(TheAcceleratorKey::CTRL))
                {
                    ok = false;
                }

                ok
            } else {
                false
            }
        } else {
            false
        }
    }
}

/// The roles for dialog buttons.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TheDialogButtonRole {
    Accept,
    Reject,
    Delete,
    Rename,
}

impl TheDialogButtonRole {
    pub fn to_string(self) -> &'static str {
        match self {
            Self::Accept => "Accept",
            Self::Reject => "Cancel",
            Self::Delete => "Delete",
            Self::Rename => "Delete",
        }
    }
    pub fn to_id(self) -> &'static str {
        match self {
            Self::Accept => "TheDialogButtonRole::Accept",
            Self::Reject => "TheDialogButtonRole::Reject",
            Self::Delete => "TheDialogButtonRole::Delete",
            Self::Rename => "TheDialogButtonRole::Rename",
        }
    }
    pub fn iterator() -> impl Iterator<Item = TheDialogButtonRole> {
        [Self::Accept, Self::Reject, Self::Delete, Self::Rename]
            .iter()
            .copied()
    }
}

pub struct TheUI {
    pub canvas: TheCanvas,

    pub dialog_text: String,
    pub dialog: Option<TheCanvas>,

    pub style: Box<dyn TheStyle>,

    state_events_receiver: Option<Receiver<TheEvent>>,

    app_state_events: FxHashMap<String, Sender<TheEvent>>,

    statusbar_name: Option<String>,

    pub context_menu: Option<TheContextMenu>,
    pub menu_widget_id: Option<TheId>,

    pub is_dirty: bool,

    // Modifiers
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub logo: bool,

    // Mouse pos
    pub mouse_coord: Vec2<i32>,
}

impl Default for TheUI {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(unused)]
impl TheUI {
    pub fn new() -> Self {
        Self {
            canvas: TheCanvas::new(),

            style: Box::new(TheClassicStyle::new()),

            state_events_receiver: None,
            app_state_events: FxHashMap::default(),

            dialog_text: "".to_string(),
            dialog: None,

            statusbar_name: None,

            context_menu: None,
            menu_widget_id: None,
            is_dirty: false,

            shift: false,
            ctrl: false,
            alt: false,
            logo: false,

            mouse_coord: Vec2::zero(),
        }
    }

    pub fn init(&mut self, ctx: &mut TheContext) {
        let (tx, rx) = mpsc::channel();

        self.state_events_receiver = Some(rx);
        ctx.ui.state_events_sender = Some(tx);
    }

    /// Adds a widget state listener of the given name. Returns the Receiver<TheEvent> which the app can user to react to widget state changes. An app can add several listeners.
    pub fn add_state_listener(&mut self, name: String) -> Receiver<TheEvent> {
        let (tx, rx) = mpsc::channel();
        self.app_state_events.insert(name, tx);
        rx
    }

    pub fn set_statusbar_name(&mut self, name: String) {
        self.statusbar_name = Some(name);
    }

    pub fn relayout(&mut self, ctx: &mut TheContext) {
        let width = self.canvas.buffer().dim().width;
        let height = self.canvas.buffer().dim().height;
        self.canvas.layout(width, height, ctx);
        ctx.ui.relayout = false;
    }

    /// Returns true if the current focus widget supports text input.
    pub fn focus_widget_supports_text_input(&mut self, ctx: &mut TheContext) -> bool {
        let mut supports = false;
        if let Some(id) = &ctx.ui.focus {
            if let Some(widget) = self.get_widget_abs(None, Some(&id.uuid)) {
                supports = widget.supports_text_input();
            }
        }
        supports
    }

    /// Returns true if the current focus widget supports clipboard operations.
    pub fn focus_widget_supports_clipboard(&mut self, ctx: &mut TheContext) -> bool {
        let mut supports = false;
        if let Some(id) = &ctx.ui.focus {
            if let Some(widget) = self.get_widget_abs(None, Some(&id.uuid)) {
                supports = widget.supports_clipboard();
            }
        }
        supports
    }

    /// Returns true if the current focus widget supports internal undo / redo operations.
    pub fn focus_widget_supports_undo_redo(&mut self, ctx: &mut TheContext) -> bool {
        let mut supports = false;
        if let Some(id) = &ctx.ui.focus {
            if let Some(widget) = self.get_widget_abs(None, Some(&id.uuid)) {
                supports = widget.supports_undo_redo();
            }
        }
        supports
    }

    /// Initiate a cut operation on the current focus widget.
    pub fn cut(&mut self, ctx: &mut TheContext) {
        if let Some(id) = &ctx.ui.focus {
            if let Some(widget) = self.get_widget_abs(None, Some(&id.uuid)) {
                let event = TheEvent::Cut;
                self.is_dirty = widget.on_event(&event, ctx);
                self.process_events(ctx);
            }
        }
    }

    /// Initiate a copy operation on the current focus widget.
    pub fn copy(&mut self, ctx: &mut TheContext) {
        if let Some(id) = &ctx.ui.focus {
            if let Some(widget) = self.get_widget_abs(None, Some(&id.uuid)) {
                let event = TheEvent::Copy;
                self.is_dirty = widget.on_event(&event, ctx);
                self.process_events(ctx);
            }
        }
    }

    /// Initiate a paste operation on the current focus widget.
    pub fn paste(&mut self, ctx: &mut TheContext) {
        if let Some(id) = &ctx.ui.focus {
            if let Some(widget) = self.get_widget_abs(None, Some(&id.uuid)) {
                let event = if let Some(value) = &ctx.ui.clipboard {
                    TheEvent::Paste(value.clone(), ctx.ui.clipboard_app_type.clone())
                } else {
                    TheEvent::Paste(TheValue::Empty, ctx.ui.clipboard_app_type.clone())
                };
                self.is_dirty = widget.on_event(&event, ctx);
                self.process_events(ctx);
            }
        }
    }

    /// Initiate a widget based undo.
    pub fn undo(&mut self, ctx: &mut TheContext) {
        if let Some(id) = &ctx.ui.focus {
            if let Some(widget) = self.get_widget_abs(None, Some(&id.uuid)) {
                let event = TheEvent::Undo;
                self.is_dirty = widget.on_event(&event, ctx);
                self.process_events(ctx);
            }
        }
    }

    /// Initiate a widget based redo.
    pub fn redo(&mut self, ctx: &mut TheContext) {
        if let Some(id) = &ctx.ui.focus {
            if let Some(widget) = self.get_widget_abs(None, Some(&id.uuid)) {
                let event = TheEvent::Redo;
                self.is_dirty = widget.on_event(&event, ctx);
                self.process_events(ctx);
            }
        }
    }

    pub fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        if ctx.width == 0 || ctx.height == 0 {
            return;
        };
        if self.canvas.resize(ctx.width as i32, ctx.height as i32, ctx) {
            ctx.ui.send(TheEvent::Resize);
            ctx.ui.relayout = false;
        }
        if ctx.ui.relayout {
            self.relayout(ctx);
        }
        self.canvas.draw(&mut self.style, ctx);
        if self.dialog.is_some() {
            self.draw_dialog(ctx);
        }
        self.canvas.draw_overlay(&mut self.style, ctx);
        if let Some(drop) = &ctx.ui.drop {
            if let Some(position) = &drop.position {
                self.canvas.buffer.blend_into(
                    position.x - drop.offset.x,
                    position.y - drop.offset.y,
                    &drop.image,
                )
            }
        }
        if let Some(menu) = &mut self.context_menu {
            menu.draw(self.canvas.buffer.pixels_mut(), &mut self.style, ctx);
        }
        ctx.ui.redraw_all = false;

        pixels.copy_from_slice(self.canvas.buffer().pixels());
        self.is_dirty = false;
    }

    /// Processes widget state events, these are mostly send from TheUIContext based on state changes provided by the widgets.
    pub fn process_events(&mut self, ctx: &mut TheContext) {
        if let Some(receiver) = &mut self.state_events_receiver {
            while let Ok(event) = receiver.try_recv() {
                // Resend event to all app listeners
                for (name, sender) in &self.app_state_events {
                    sender.send(event.clone()).unwrap();
                }

                match event {
                    TheEvent::SetClipboard(value, app_type) => {
                        ctx.ui.clipboard = Some(value);
                        ctx.ui.clipboard_app_type = app_type;
                        ctx.ui.send(TheEvent::ClipboardChanged);
                    }
                    TheEvent::ShowMenu(id, coord, mut menu) => {
                        menu.set_position(coord, ctx);
                        menu.id = id.clone();
                        self.context_menu = Some(menu);
                        self.menu_widget_id = Some(id.clone());
                    }
                    TheEvent::ShowContextMenu(id, coord, mut menu) => {
                        menu.set_position(coord, ctx);
                        menu.id = id;
                        self.context_menu = Some(menu);
                        self.menu_widget_id = None;
                    }
                    TheEvent::RedirectWidgetValueToLayout(layout_id, widget_id, value) => {
                        if let Some(layout) = self.canvas.get_layout(None, Some(&layout_id.uuid)) {
                            layout.redirected_widget_value(&widget_id, &value, ctx);
                        }
                    }
                    TheEvent::DragStartedWithNoImage(drop) => {
                        let mut drop = drop.clone();
                        self.style.create_drop_image(&mut drop, ctx);
                        ctx.ui.drop = Some(drop);
                    }
                    TheEvent::NewListItemSelected(id, layout_id) => {
                        if let Some(layout) = self.canvas.get_layout(None, Some(&layout_id.uuid)) {
                            if let Some(list) = layout.as_list_layout() {
                                list.new_item_selected(id);
                                self.is_dirty = true;
                            } else if let Some(list) = layout.as_rowlist_layout() {
                                list.new_item_selected(id);
                                self.is_dirty = true;
                            } else if let Some(tree) = layout.as_tree_layout() {
                                tree.new_item_selected(id.clone());
                                self.is_dirty = true;
                                ctx.ui.redraw_all = true;
                            }
                        }
                    }
                    TheEvent::SnapperStateChanged(id, layout_id, open) => {
                        if let Some(layout) = self.canvas.get_layout(None, Some(&layout_id.uuid)) {
                            if let Some(tree) = layout.as_tree_layout() {
                                tree.tree_node_state_changed(id.clone(), open);
                                ctx.ui.relayout = true;
                                tree.set_dim(tree.dim().clone(), ctx);
                                ctx.ui.relayout = false;
                            }
                        }
                    }
                    TheEvent::ScrollLayout(layout_id, delta) => {
                        if let Some(layout) = self.canvas.get_layout(None, Some(&layout_id.uuid)) {
                            if let Some(list) = layout.as_list_layout() {
                                list.scroll_by(delta);
                                self.is_dirty = true;
                            } else if let Some(list) = layout.as_rowlist_layout() {
                                list.scroll_by(delta);
                                self.is_dirty = true;
                            } else if let Some(list) = layout.as_tree_layout() {
                                list.scroll_by(delta);
                                self.is_dirty = true;
                            }
                        }
                    }
                    TheEvent::SetStackIndex(id, index) => {
                        if let Some(layout) = self.canvas.get_layout(None, Some(&id.uuid)) {
                            if let Some(stack) = layout.as_stack_layout() {
                                if stack.index() != index {
                                    stack.set_index(index);
                                    self.is_dirty = true;
                                    ctx.ui.redraw_all = true;
                                    ctx.ui.relayout = true;
                                }
                            }
                        } else if let Some(layout) = self.canvas.get_layout(Some(&id.name), None) {
                            if let Some(stack) = layout.as_stack_layout() {
                                if stack.index() != index {
                                    stack.set_index(index);
                                    self.is_dirty = true;
                                    ctx.ui.redraw_all = true;
                                    ctx.ui.relayout = true;
                                }
                            }
                        }
                    }
                    TheEvent::StateChanged(id, state) => {
                        //println!("Widget State changed {:?}: {:?}", id, state);

                        if let Some(dialog) = &mut self.dialog {
                            // If a dialog, close it if one of the dialog buttons was clicked.
                            if state == TheWidgetState::Clicked
                                && id.name.starts_with("TheDialogButtonRole")
                            {
                                for button in TheDialogButtonRole::iterator() {
                                    if id.name == button.to_id() {
                                        if let Some(widget) = dialog
                                            .get_widget(Some(&"Dialog Value".to_string()), None)
                                        {
                                            let value = widget.value();
                                            ctx.ui.send(TheEvent::DialogValueOnClose(
                                                button,
                                                self.dialog_text.clone(),
                                                widget.id().uuid,
                                                value,
                                            ));
                                        }
                                    }
                                }
                                self.dialog = None;
                            }
                        }
                    }
                    TheEvent::SetState(name, state) => {
                        if let Some(widget) = self.canvas.get_widget(Some(&name), None) {
                            widget.set_state(state);
                        }
                        self.is_dirty = true;
                    }
                    TheEvent::SetStateId(id, state) => {
                        if let Some(widget) = self.canvas.get_widget(None, Some(&id)) {
                            widget.set_state(state);
                        }
                        self.is_dirty = true;
                    }
                    TheEvent::ScrollBy(id, delta) => {
                        //println!("Set State {:?}: {:?}", name, state);
                        if let Some(widget) = self.canvas.get_widget(None, Some(&id.uuid)) {
                            widget.on_event(&TheEvent::ScrollBy(id.clone(), delta), ctx);
                        }
                        self.is_dirty = true;
                    }
                    TheEvent::GainedFocus(id) => {
                        //println!("Gained focus {:?}", id);
                        if let Some(widget) = self.canvas.get_widget(None, Some(&id.uuid)) {
                            widget.on_event(&TheEvent::GainedFocus(widget.id().clone()), ctx);
                            widget.set_needs_redraw(true);

                            // Update cursor when widget gains focus
                            if let Some(cursor_icon) = widget.cursor_icon() {
                                ctx.set_cursor_icon(cursor_icon);
                            }
                        }
                    }
                    TheEvent::LostFocus(id) => {
                        //println!("Lost focus {:?}", id);
                        if let Some(widget) = self.canvas.get_widget(None, Some(&id.uuid)) {
                            widget.on_event(&TheEvent::LostFocus(widget.id().clone()), ctx);
                            widget.set_needs_redraw(true);

                            // Reset cursor to default when widget loses focus
                            ctx.set_cursor_icon(TheCursorIcon::Default);
                        }
                    }
                    TheEvent::GainedHover(id) => {
                        //println!("Gained hover {:?}", id);
                        if let Some(statusbar_name) = &self.statusbar_name {
                            let mut status_text: Option<String> = None;
                            if let Some(widget) = self.canvas.get_widget(None, Some(&id.uuid)) {
                                status_text = widget.status_text();
                            }

                            if let Some(widget) = self.canvas.get_widget(Some(statusbar_name), None)
                            {
                                if let Some(widget) = widget.as_statusbar() {
                                    if let Some(status_text) = status_text {
                                        widget.set_text(status_text);
                                    } else {
                                        widget.set_text("".to_string());
                                    }
                                }
                            }
                        }
                    }
                    TheEvent::LostHover(id) => {
                        //println!("Lost hover {:?}", id);
                        if let Some(widget) = self.canvas.get_widget(None, Some(&id.uuid)) {
                            widget.on_event(&TheEvent::LostHover(widget.id().clone()), ctx);
                            widget.set_needs_redraw(true);
                        }
                        if let Some(statusbar_name) = &self.statusbar_name {
                            let mut status_text: Option<String> = None;

                            if let Some(widget) = self.canvas.get_widget(Some(statusbar_name), None)
                            {
                                if let Some(widget) = widget.as_statusbar() {
                                    if let Some(status_text) = status_text {
                                        widget.set_text(status_text);
                                    } else {
                                        widget.set_text("".to_string());
                                    }
                                }
                            }
                        }
                    }
                    TheEvent::SetStatusText(_id, text) => {
                        if let Some(statusbar_name) = &self.statusbar_name {
                            if let Some(widget) = self.canvas.get_widget(Some(statusbar_name), None)
                            {
                                if let Some(widget) = widget.as_statusbar() {
                                    widget.set_text(text);
                                }
                            }
                        }
                    }
                    TheEvent::ValueChanged(id, value) => {
                        //println!("Widget Value changed {:?}: {:?}", id, value);
                    }
                    TheEvent::SetValue(uuid, value) => {
                        //println!("Set Value {:?}: {:?}", name, value);
                        if let Some(widget) = self.canvas.get_widget(None, Some(&uuid)) {
                            widget.set_value(value.clone());
                            ctx.ui.send_widget_value_changed(widget.id(), value);
                        }
                        self.is_dirty = true;
                    }
                    _ => {}
                }
            }
        }
    }

    /// Set the given id as disabled.
    pub fn set_disabled(&mut self, id: &str, ctx: &mut TheContext) {
        ctx.ui.set_disabled(id);
        if let Some(widget) = self.get_widget(id) {
            widget.set_needs_redraw(true);
        }
    }

    /// Remove the given id from the disabled list.
    pub fn set_enabled(&mut self, id: &str, ctx: &mut TheContext) {
        ctx.ui.set_enabled(id);
        if let Some(widget) = self.get_widget(id) {
            widget.set_needs_redraw(true);
        }
    }

    pub fn update(&mut self, ctx: &mut TheContext) -> bool {
        // Check if the result of an FileRequester is available, and if yes, send the result
        if let Some(rx) = &ctx.ui.file_requester_receiver {
            let rc = rx.1.try_recv();
            if let Ok(paths) = rc {
                ctx.ui
                    .send(TheEvent::FileRequesterResult(rx.0.clone(), paths));
                ctx.ui.file_requester_receiver = None;
            }
        }

        self.process_events(ctx);
        self.is_dirty
    }

    pub fn context(&mut self, x: f32, y: f32, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        let coord = Vec2::new(x as i32, y as i32);
        if let Some(widget) = self.get_widget_at_coord(coord) {
            let event = TheEvent::Context(coord);
            redraw = widget.on_event(&event, ctx);

            self.process_events(ctx);
        }
        redraw
    }

    pub fn touch_down(&mut self, x: f32, y: f32, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        let coord = Vec2::new(x as i32, y as i32);

        ctx.ui.send(TheEvent::MouseDown(coord));

        //ctx.ui.clear_focus();

        if let Some(context) = &mut self.context_menu {
            if context.contains(coord) {
                let event = TheEvent::MouseDown(context.dim.to_local(coord));
                if context.on_event(&event, ctx) {
                    redraw = true;
                    if let Some((menu_id, menu_item_id)) = context.get_hovered_id() {
                        ctx.ui.send(TheEvent::ContextMenuSelected(
                            menu_id.clone(),
                            menu_item_id.clone(),
                        ));
                        ctx.ui.send(TheEvent::StateChanged(
                            menu_item_id.clone(),
                            TheWidgetState::Clicked,
                        ));
                    }
                    self.context_menu = None;
                    let menu_widget_id = self.menu_widget_id.clone();
                    if let Some(menu_widget_id) = menu_widget_id {
                        if let Some(widget) = self.get_widget_abs(None, Some(&menu_widget_id.uuid))
                        {
                            widget.on_event(&TheEvent::ContextMenuClosed(menu_widget_id), ctx);
                        }
                    }
                    self.menu_widget_id = None;
                    ctx.ui.clear_hover();
                }
            } else {
                self.context_menu = None;
                let menu_widget_id = self.menu_widget_id.clone();
                if let Some(menu_widget_id) = menu_widget_id {
                    if let Some(widget) = self.get_widget_abs(None, Some(&menu_widget_id.uuid)) {
                        widget.on_event(&TheEvent::ContextMenuClosed(menu_widget_id), ctx);
                    }
                }
                self.menu_widget_id = None;
                ctx.ui.clear_hover();
                redraw = true;
            }
            return redraw;
        }

        if let Some(widget) = self.get_widget_at_coord(coord) {
            let event = TheEvent::MouseDown(widget.dim().to_local(coord));
            redraw = widget.on_event(&event, ctx);

            self.process_events(ctx);
        }
        redraw
    }

    pub fn touch_dragged(&mut self, x: f32, y: f32, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        let coord = Vec2::new(x as i32, y as i32);

        if let Some(context) = &mut self.context_menu {
            return redraw;
        }

        if let Some(id) = &ctx.ui.overlay {
            if let Some(widget) = self.get_widget_abs(None, Some(&id.uuid)) {
                let event = TheEvent::MouseDragged(widget.dim().to_local(coord));
                redraw = widget.on_event(&event, ctx);
                self.process_events(ctx);
            }
        } else if let Some(id) = &ctx.ui.focus {
            if let Some(widget) = self.get_widget_abs(None, Some(&id.uuid)) {
                let event = TheEvent::MouseDragged(widget.dim().to_local(coord));
                redraw = widget.on_event(&event, ctx);
                self.process_events(ctx);
            }
        } else if let Some(widget) = self.canvas.get_widget_at_coord(coord) {
            let event = TheEvent::MouseDragged(widget.dim().to_local(coord));
            redraw = widget.on_event(&event, ctx);
            self.process_events(ctx);
        }

        if let Some(drop) = &mut ctx.ui.drop {
            drop.set_position(coord);
            if let Some(widget) = self.canvas.get_widget_at_coord(coord) {
                let event = TheEvent::DropPreview(widget.dim().to_local(coord), drop.clone());
                redraw = widget.on_event(&event, ctx);
                self.process_events(ctx);
            }
            redraw = true;
        }

        redraw
    }

    pub fn touch_up(&mut self, x: f32, y: f32, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        let coord = Vec2::new(x as i32, y as i32);

        ctx.ui.send(TheEvent::MouseUp(coord));

        if let Some(context) = &mut self.context_menu {
            return redraw;
        }

        if let Some(id) = &ctx.ui.overlay {
            if let Some(widget) = self.get_widget_abs(Some(&id.name), Some(&id.uuid)) {
                let event = TheEvent::MouseUp(widget.dim().to_local(coord));
                redraw = widget.on_event(&event, ctx);
                self.process_events(ctx);
            }
        } else if let Some(id) = &ctx.ui.focus {
            if let Some(widget) = self.get_widget_abs(Some(&id.name), Some(&id.uuid)) {
                let event = TheEvent::MouseUp(widget.dim().to_local(coord));
                redraw = widget.on_event(&event, ctx);
                self.process_events(ctx);
            }
        } else if let Some(widget) = self.canvas.get_widget_at_coord(coord) {
            let event = TheEvent::MouseUp(widget.dim().to_local(coord));
            redraw = widget.on_event(&event, ctx);
            self.process_events(ctx);
        }

        if let Some(drop) = &ctx.ui.drop {
            if let Some(widget) = self.canvas.get_widget_at_coord(coord) {
                let mut drop_copy = drop.clone();
                drop_copy.target_id = widget.id().clone();
                let event = TheEvent::Drop(widget.dim().to_local(coord), drop.clone());
                redraw = widget.on_event(&event, ctx);
                ctx.ui.send(event);
                self.process_events(ctx);
            }
            redraw = true;
        }

        ctx.ui.clear_drop();
        redraw
    }

    pub fn hover(&mut self, x: f32, y: f32, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        let coord = Vec2::new(x as i32, y as i32);
        self.mouse_coord = coord;

        if let Some(context) = &mut self.context_menu {
            if context.contains(coord) {
                let event = TheEvent::Hover(context.dim.to_local(coord));
                redraw = context.on_event(&event, ctx);
            }
            let menu_widget_id = self.menu_widget_id.clone();
            if self.menu_widget_id.is_some() {
                if let Some(widget) = self.get_widget_at_coord(coord) {
                    if Some(widget.id().clone()) == menu_widget_id {
                        let event = TheEvent::Hover(widget.dim().to_local(coord));
                        redraw = widget.on_event(&event, ctx);
                    }
                }
            }
            return redraw;
        }

        if let Some(widget) = self.get_widget_at_coord(coord) {
            let event = TheEvent::Hover(widget.dim().to_local(coord));
            redraw = widget.on_event(&event, ctx);

            // Check if the widget has a cursor icon and set it
            if let Some(cursor_icon) = widget.cursor_icon() {
                ctx.set_cursor_icon(cursor_icon);
            } else {
                // Reset to default if widget doesn't specify a cursor
                ctx.set_cursor_icon(TheCursorIcon::Default);
            }

            // If the new hover widget does not support a hover state, make sure to unhover the current widget if any
            if !widget.supports_hover() {
                if let Some(hover) = &ctx.ui.hover {
                    ctx.ui.send(TheEvent::LostHover(hover.clone()));
                    redraw = true;
                    ctx.ui.hover = None;
                }
            }

            self.process_events(ctx);
        } else if let Some(hover) = &ctx.ui.hover {
            ctx.ui.send(TheEvent::LostHover(hover.clone()));
            redraw = true;
            ctx.ui.hover = None;

            // Reset cursor to default when no widget is hovered
            ctx.set_cursor_icon(TheCursorIcon::Default);

            self.process_events(ctx);
        }
        redraw
    }

    pub fn mouse_wheel(&mut self, delta: (i32, i32), ctx: &mut TheContext) -> bool {
        let mut redraw = false;

        let mut layout_id = None;
        if let Some(id) = self.get_layout_at_coord(self.mouse_coord) {
            layout_id = Some(id);
        }

        let mut processed = false;

        // We check first if the layout under the mouse supports manual scrolling, and if yes use that
        if let Some(layout_id) = layout_id {
            if let Some(layout) = self.get_layout(&layout_id.name) {
                if layout.supports_mouse_wheel() {
                    layout.mouse_wheel_scroll(Vec2::new(delta.0, delta.1));
                    processed = true;
                }
            }
        }

        if !processed {
            // If not processed, call the widget directly.
            if let Some(id) = &ctx.ui.hover {
                if let Some(widget) = self.get_widget_abs(Some(&id.name), Some(&id.uuid)) {
                    redraw =
                        widget.on_event(&TheEvent::MouseWheel(Vec2::new(delta.0, delta.1)), ctx);
                    self.process_events(ctx);
                }
            }
        }
        redraw
    }

    pub fn key_down(
        &mut self,
        char: Option<char>,
        key: Option<TheKeyCode>,
        ctx: &mut TheContext,
    ) -> bool {
        let mut redraw = false;
        let mut consumed = false;

        if let Some(c) = char {
            if self.ctrl || self.shift || self.alt || self.logo {
                // Check for accelerators in context menus.
                for (id, accel) in &ctx.ui.accelerators.clone() {
                    if accel.matches(self.shift, self.ctrl, self.alt, self.logo, c) {
                        consumed = true;
                        ctx.ui
                            .send(TheEvent::ContextMenuSelected(id.clone(), id.clone()));
                        ctx.ui
                            .send(TheEvent::StateChanged(id.clone(), TheWidgetState::Selected));
                        break;
                    }
                }
            }
        }

        if !consumed {
            let event = if let Some(c) = char {
                TheEvent::KeyDown(TheValue::Char(c))
            } else {
                if key.clone().unwrap().clone() == TheKeyCode::Escape && self.context_menu.is_some()
                {
                    self.context_menu = None;
                    let menu_widget_id = self.menu_widget_id.clone();
                    if let Some(menu_widget_id) = menu_widget_id {
                        if let Some(widget) = self.get_widget_abs(None, Some(&menu_widget_id.uuid))
                        {
                            widget.on_event(&TheEvent::ContextMenuClosed(menu_widget_id), ctx);
                        }
                    }
                    self.menu_widget_id = None;
                    ctx.ui.clear_hover();
                    return true;
                }

                TheEvent::KeyCodeDown(TheValue::KeyCode(key.unwrap()))
            };
            ctx.ui.send(event.clone());
            if let Some(id) = &ctx.ui.focus {
                if let Some(widget) = self.get_widget_abs(Some(&id.name), Some(&id.uuid)) {
                    redraw = widget.on_event(&event, ctx);
                    self.process_events(ctx);
                }
            }
        }
        redraw
    }

    pub fn key_up(
        &mut self,
        char: Option<char>,
        key: Option<TheKeyCode>,
        ctx: &mut TheContext,
    ) -> bool {
        let mut redraw = false;

        let event = if let Some(c) = char {
            TheEvent::KeyUp(TheValue::Char(c))
        } else {
            TheEvent::KeyCodeUp(TheValue::KeyCode(key.unwrap()))
        };
        ctx.ui.send(event.clone());

        false
    }

    pub fn modifier_changed(
        &mut self,
        shift: bool,
        ctrl: bool,
        alt: bool,
        logo: bool,
        ctx: &mut TheContext,
    ) -> bool {
        let mut redraw = false;

        self.shift = shift;
        self.ctrl = ctrl;
        self.alt = alt;
        self.logo = logo;

        if let Some(id) = &ctx.ui.focus {
            if let Some(widget) = self.get_widget_abs(Some(&id.name), Some(&id.uuid)) {
                let event = TheEvent::ModifierChanged(shift, ctrl, alt, logo);
                redraw = widget.on_event(&event, ctx);
                self.process_events(ctx);
            }
        }
        if ctx.ui.focus != ctx.ui.hover {
            if let Some(id) = &ctx.ui.hover {
                if let Some(widget) = self.get_widget_abs(Some(&id.name), Some(&id.uuid)) {
                    let event = TheEvent::ModifierChanged(shift, ctrl, alt, logo);
                    redraw = widget.on_event(&event, ctx);
                    self.process_events(ctx);
                }
            }
        }
        redraw
    }

    /// Returns the layout at the given position.
    pub fn get_layout_at_coord(&mut self, coord: Vec2<i32>) -> Option<TheId> {
        if let Some(dialog) = &mut self.dialog {
            if let Some(layout) = dialog.get_layout_at_coord(coord) {
                return Some(layout);
            }
        } else if let Some(layout) = self.canvas.get_layout_at_coord(coord) {
            return Some(layout);
        }
        None
    }

    /// Returns the absolute widget at the given position.
    pub fn get_widget_at_coord(&mut self, coord: Vec2<i32>) -> Option<&mut Box<dyn TheWidget>> {
        if let Some(dialog) = &mut self.dialog {
            if let Some(widget) = dialog.get_widget_at_coord(coord) {
                return Some(widget);
            }
        } else if let Some(widget) = self.canvas.get_widget_at_coord(coord) {
            return Some(widget);
        }
        None
    }

    pub fn get_widget_abs(
        &mut self,
        name: Option<&String>,
        uuid: Option<&Uuid>,
    ) -> Option<&mut Box<dyn TheWidget>> {
        if let Some(dialog) = &mut self.dialog {
            dialog.get_widget(name, uuid)
        } else {
            self.canvas.get_widget(name, uuid)
        }
    }

    /// Gets a given widget by name
    pub fn get_widget(&mut self, name: &str) -> Option<&mut Box<dyn TheWidget>> {
        self.canvas.get_widget(Some(&name.to_string()), None)
    }

    /// Gets a given widget by id
    pub fn get_widget_id(&mut self, id: Uuid) -> Option<&mut Box<dyn TheWidget>> {
        self.canvas.get_widget(None, Some(&id))
    }

    /// Gets a given text line edit by name
    pub fn get_text_line_edit(&mut self, name: &str) -> Option<&mut dyn TheTextLineEditTrait> {
        if let Some(text_line_edit) = self.canvas.get_widget(Some(&name.to_string()), None) {
            return text_line_edit.as_text_line_edit();
        }
        None
    }

    /// Gets a given text area edit by name
    pub fn get_text_area_edit(&mut self, name: &str) -> Option<&mut dyn TheTextAreaEditTrait> {
        if let Some(text_area_edit) = self.canvas.get_widget(Some(&name.to_string()), None) {
            return text_area_edit.as_text_area_edit();
        }
        None
    }

    /// Gets a given icon view by name
    pub fn get_icon_view(&mut self, name: &str) -> Option<&mut dyn TheIconViewTrait> {
        if let Some(text_line_edit) = self.canvas.get_widget(Some(&name.to_string()), None) {
            return text_line_edit.as_icon_view();
        }
        None
    }

    /// Gets a given menu by name
    pub fn get_menu(&mut self, name: &str) -> Option<&mut dyn TheMenuTrait> {
        if let Some(menu) = self.canvas.get_widget(Some(&name.to_string()), None) {
            return menu.as_menu();
        }
        None
    }

    /// Gets a given render view by name
    pub fn get_render_view(&mut self, name: &str) -> Option<&mut dyn TheRenderViewTrait> {
        if let Some(render_view) = self.canvas.get_widget(Some(&name.to_string()), None) {
            return render_view.as_render_view();
        }
        None
    }

    /// Gets a given text by name
    pub fn get_text(&mut self, name: &str) -> Option<&mut dyn TheTextTrait> {
        if let Some(text) = self.canvas.get_widget(Some(&name.to_string()), None) {
            return text.as_text();
        }
        None
    }

    /// Gets a given group button by name
    pub fn get_group_button(&mut self, name: &str) -> Option<&mut dyn TheGroupButtonTrait> {
        if let Some(text) = self.canvas.get_widget(Some(&name.to_string()), None) {
            return text.as_group_button();
        }
        None
    }

    /// Gets a given statusbar by name
    pub fn get_statusbar(&mut self, name: &str) -> Option<&mut dyn TheStatusbarTrait> {
        if let Some(text) = self.canvas.get_widget(Some(&name.to_string()), None) {
            return text.as_statusbar();
        }
        None
    }

    /// Gets a given drop down menu by name
    pub fn get_drop_down_menu(&mut self, name: &str) -> Option<&mut dyn TheDropdownMenuTrait> {
        if let Some(drop_down_menu) = self.canvas.get_widget(Some(&name.to_string()), None) {
            return drop_down_menu.as_drop_down_menu();
        }
        None
    }

    /// Gets a given time slider by name
    pub fn get_time_slider(&mut self, name: &str) -> Option<&mut dyn TheTimeSliderTrait> {
        if let Some(text) = self.canvas.get_widget(Some(&name.to_string()), None) {
            return text.as_time_slider();
        }
        None
    }

    /// Gets a given palette picker by name
    pub fn get_palette_picker(&mut self, name: &str) -> Option<&mut dyn ThePalettePickerTrait> {
        if let Some(text) = self.canvas.get_widget(Some(&name.to_string()), None) {
            return text.as_palette_picker();
        }
        None
    }

    /// Gets a given layout by name
    pub fn get_layout(&mut self, name: &str) -> Option<&mut Box<dyn TheLayout>> {
        self.canvas.get_layout(Some(&name.to_string()), None)
    }

    /// Relayouts the given layout.
    pub fn relayout_layout(&mut self, name: &str, ctx: &mut TheContext) {
        if let Some(l) = self.canvas.get_layout(Some(&name.to_string()), None) {
            l.relayout(ctx);
        }
    }

    /// Gets a given TheListLayout by name
    pub fn get_list_layout(&mut self, name: &str) -> Option<&mut dyn TheListLayoutTrait> {
        if let Some(text_line_edit) = self.canvas.get_layout(Some(&name.to_string()), None) {
            return text_line_edit.as_list_layout();
        }
        None
    }

    /// Gets a given TheTreeLayout by name
    pub fn get_tree_layout(&mut self, name: &str) -> Option<&mut dyn TheTreeLayoutTrait> {
        if let Some(layout) = self.canvas.get_layout(Some(&name.to_string()), None) {
            return layout.as_tree_layout();
        }
        None
    }

    /// Gets a given TheRowListLayout by name
    pub fn get_rowlist_layout(&mut self, name: &str) -> Option<&mut dyn TheRowListLayoutTrait> {
        if let Some(text_line_edit) = self.canvas.get_layout(Some(&name.to_string()), None) {
            return text_line_edit.as_rowlist_layout();
        }
        None
    }

    /// Gets a given TheStackLayout by name
    pub fn get_stack_layout(&mut self, name: &str) -> Option<&mut dyn TheStackLayoutTrait> {
        if let Some(text_line_edit) = self.canvas.get_layout(Some(&name.to_string()), None) {
            return text_line_edit.as_stack_layout();
        }
        None
    }

    /// Selects the first item of a list layout.
    pub fn select_first_list_item(&mut self, name: &str, ctx: &mut TheContext) {
        if let Some(layout) = self.get_list_layout(name) {
            layout.select_first_item(ctx);
        }
    }

    pub fn select_list_item_at(&mut self, name: &str, index: i32, ctx: &mut TheContext) {
        if let Some(layout) = self.get_list_layout(name) {
            layout.select_item_at(index, ctx, true);
        }
    }

    /// Gets a given TheRGBALayout by name
    pub fn get_rgba_layout(&mut self, name: &str) -> Option<&mut dyn TheRGBALayoutTrait> {
        if let Some(layout) = self.canvas.get_layout(Some(&name.to_string()), None) {
            return layout.as_rgba_layout();
        }
        None
    }

    /// Gets a given TheSharedHLayout by name
    pub fn get_sharedhlayout(&mut self, name: &str) -> Option<&mut dyn TheSharedHLayoutTrait> {
        if let Some(layout) = self.canvas.get_layout(Some(&name.to_string()), None) {
            return layout.as_sharedhlayout();
        }
        None
    }

    /// Gets a given TheSharedVLayout by name
    pub fn get_sharedvlayout(&mut self, name: &str) -> Option<&mut dyn TheSharedVLayoutTrait> {
        if let Some(layout) = self.canvas.get_layout(Some(&name.to_string()), None) {
            return layout.as_sharedvlayout();
        }
        None
    }

    /// Gets a given TheHLayout by name
    pub fn get_hlayout(&mut self, name: &str) -> Option<&mut dyn TheHLayoutTrait> {
        if let Some(layout) = self.canvas.get_layout(Some(&name.to_string()), None) {
            return layout.as_hlayout();
        }
        None
    }

    /// Gets a given TheVLayout by name
    pub fn get_vlayout(&mut self, name: &str) -> Option<&mut dyn TheVLayoutTrait> {
        if let Some(layout) = self.canvas.get_layout(Some(&name.to_string()), None) {
            return layout.as_vlayout();
        }
        None
    }

    /// Gets a given TheTextLayout by name
    pub fn get_text_layout(&mut self, name: &str) -> Option<&mut dyn TheTextLayoutTrait> {
        if let Some(layout) = self.canvas.get_layout(Some(&name.to_string()), None) {
            return layout.as_text_layout();
        }
        None
    }

    /// Sets the nodes for a node canvas.
    pub fn set_node_canvas(&mut self, name: &str, canvas: TheNodeCanvas) {
        if let Some(view) = self.canvas.get_widget(Some(&name.to_string()), None) {
            if let Some(nodes) = view.as_node_canvas_view() {
                nodes.set_canvas(canvas);
            }
        }
    }

    /// Sets the overlay for the node canvas.
    pub fn set_node_overlay(&mut self, name: &str, overlay: Option<TheRGBABuffer>) {
        if let Some(view) = self.canvas.get_widget(Some(&name.to_string()), None) {
            if let Some(nodes) = view.as_node_canvas_view() {
                nodes.set_overlay(overlay);
            }
        }
    }

    /// Sets the preview for a node in a node canvas.
    pub fn set_node_preview(&mut self, name: &str, index: usize, buffer: TheRGBABuffer) {
        if let Some(view) = self.canvas.get_widget(Some(&name.to_string()), None) {
            if let Some(nodes) = view.as_node_canvas_view() {
                nodes.set_node_preview(index, buffer);
            }
        }
    }

    /// Gets a given TheNodeCanvasView by name
    pub fn get_node_canvas_view(&mut self, name: &str) -> Option<&mut dyn TheNodeCanvasViewTrait> {
        if let Some(view) = self.canvas.get_widget(Some(&name.to_string()), None) {
            return view.as_node_canvas_view();
        }
        None
    }

    /// Set the disabled state of the given widget.
    pub fn set_widget_disabled_state(&mut self, name: &str, ctx: &mut TheContext, disabled: bool) {
        if let Some(widget) = self.canvas.get_widget(Some(&name.to_string()), None) {
            widget.set_disabled(disabled);
            if disabled && widget.id().equals(&ctx.ui.hover) {
                ctx.ui.clear_hover();
            }
            if disabled && widget.id().equals(&ctx.ui.focus) {
                ctx.ui.clear_focus();
            }
        }
    }

    /// Sets the context menu for the widget.
    pub fn set_widget_context_menu(&mut self, name: &str, menu: Option<TheContextMenu>) {
        if let Some(widget) = self.canvas.get_widget(Some(&name.to_string()), None) {
            widget.set_context_menu(menu);
        }
    }

    /// Get the value of the given widget.
    pub fn get_widget_value(&mut self, name: &str) -> Option<TheValue> {
        self.canvas
            .get_widget(Some(&name.to_string()), None)
            .map(|widget| widget.value())
    }

    /// Set the value of the given widget.
    pub fn set_widget_value(&mut self, name: &str, ctx: &mut TheContext, value: TheValue) -> bool {
        if let Some(widget) = self.canvas.get_widget(Some(&name.to_string()), None) {
            widget.set_value(value);
            true
        } else {
            false
        }
    }

    #[cfg(feature = "ui")]
    /// Opens a dialog which will have the canvas as context and the given text as title.
    pub fn show_dialog(
        &mut self,
        text: &str,
        mut canvas: TheCanvas,
        buttons: Vec<TheDialogButtonRole>,
        ctx: &mut TheContext,
    ) {
        self.dialog_text = text.to_string();

        let width = canvas.limiter.get_max_width();
        let mut height = canvas.limiter.get_max_height();

        if !buttons.is_empty() {
            let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
            toolbar_hlayout.set_background_color(Some(TheThemeColors::ListLayoutBackground));
            toolbar_hlayout.limiter_mut().set_max_width(width);
            toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));

            for b in &buttons {
                let mut button = TheTraybarButton::new(TheId::named(b.to_id()));
                button.set_text(b.to_string().to_string());
                toolbar_hlayout.add_widget(Box::new(button));
            }

            toolbar_hlayout.set_reverse_index(Some(buttons.len() as i32));

            let mut toolbar_canvas = TheCanvas::default();
            // toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
            toolbar_hlayout.limiter_mut().set_max_height(30);
            toolbar_canvas.set_layout(toolbar_hlayout);
            canvas.set_bottom(toolbar_canvas);
        }

        let off_x = (ctx.width as i32 - width) / 2;
        let off_y = (ctx.height as i32 - height) / 2;

        let mut dim = TheDim::new(off_x, off_y, width, height);
        dim.buffer_x = off_x;
        dim.buffer_y = off_y;

        canvas.set_dim(dim, ctx);

        ctx.ui.clear_focus();
        ctx.ui.clear_hover();

        self.dialog = Some(canvas);
    }

    #[cfg(feature = "ui")]
    /// Clears / closes the dialog.
    pub fn clear_dialog(&mut self) {
        self.dialog = None;
    }

    #[cfg(feature = "ui")]
    /// Draws the current dialog.
    pub fn draw_dialog(&mut self, ctx: &mut TheContext) {
        if let Some(dialog_canvas) = &mut self.dialog {
            dialog_canvas.draw(&mut self.style, ctx);

            let width = dialog_canvas.limiter.get_max_width();
            let height = dialog_canvas.limiter.get_max_height();

            // ctx.draw.rect(
            //     self.canvas.buffer.pixels_mut(),
            //     &(
            //         dialog_canvas.dim.buffer_x as usize,
            //         dialog_canvas.dim.buffer_y as usize,
            //         width as usize,
            //         height as usize,
            //     ),
            //     ctx.width,
            //     &BLACK,
            // );

            let mut tuple = dialog_canvas.dim.to_buffer_utuple();

            let window_margin = Vec4::new(3, 29, 3, 3);

            let mut border_shrinker = TheDimShrinker::zero();
            let mut border_dim = TheDim::new(
                tuple.0 as i32 - window_margin.x,
                tuple.1 as i32 - window_margin.y,
                tuple.2 as i32 + window_margin.x + window_margin.z,
                tuple.3 as i32 + window_margin.y + window_margin.w,
            );
            border_dim.buffer_x = border_dim.x;
            border_dim.buffer_y = border_dim.y;

            tuple = border_dim.to_buffer_utuple();

            ctx.draw.rect_outline(
                self.canvas.buffer.pixels_mut(),
                &tuple,
                ctx.width,
                self.style.theme().color(WindowBorderOuter),
            );

            border_shrinker.shrink(1);
            tuple = border_dim.to_buffer_shrunk_utuple(&border_shrinker);
            ctx.draw.rect_outline(
                self.canvas.buffer.pixels_mut(),
                &tuple,
                ctx.width,
                self.style.theme().color(WindowBorderInner),
            );

            border_shrinker.shrink(1);
            tuple = border_dim.to_buffer_shrunk_utuple(&border_shrinker);
            ctx.draw.rect_outline(
                self.canvas.buffer.pixels_mut(),
                &tuple,
                ctx.width,
                self.style.theme().color(WindowBorderInner),
            );

            // Header

            border_shrinker.shrink(1);
            tuple = border_dim.to_buffer_shrunk_utuple(&border_shrinker);
            ctx.draw.rect(
                self.canvas.buffer.pixels_mut(),
                &(tuple.0, tuple.1, tuple.2, 23),
                ctx.width,
                self.style.theme().color(WindowHeaderBackground),
            );

            ctx.draw.rect(
                self.canvas.buffer.pixels_mut(),
                &(tuple.0, tuple.1 + 23, tuple.2, 1),
                ctx.width,
                self.style.theme().color(WindowHeaderBorder1),
            );

            ctx.draw.rect(
                self.canvas.buffer.pixels_mut(),
                &(tuple.0, tuple.1 + 24, tuple.2, 1),
                ctx.width,
                self.style.theme().color(WindowBorderInner),
            );

            ctx.draw.rect(
                self.canvas.buffer.pixels_mut(),
                &(tuple.0, tuple.1 + 25, tuple.2, 1),
                ctx.width,
                self.style.theme().color(WindowHeaderBorder2),
            );

            ctx.draw.text_rect_blend(
                self.canvas.buffer.pixels_mut(),
                &(tuple.0 + 13, tuple.1, tuple.2 - 13, 23),
                ctx.width,
                &self.dialog_text,
                TheFontSettings {
                    size: 15.0,
                    ..Default::default()
                },
                &WHITE,
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );

            self.canvas.buffer.copy_into(
                dialog_canvas.dim.buffer_x,
                dialog_canvas.dim.buffer_y,
                &dialog_canvas.buffer,
            );
        }
    }
}
