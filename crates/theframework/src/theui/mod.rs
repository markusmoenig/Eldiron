pub mod thecanvas;
pub mod thecodehighlighter;
pub mod thecollection;
pub mod thecontextmenu;
pub mod thedrop;
pub mod thefeedback;
pub mod theflattenedmap;
pub mod theid;
pub mod thelayout;
pub mod thenodeui;
pub mod thepainter;
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
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use web_time::{Duration, Instant};

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

/// Rasterizes SVG path data into a transparent, anti-aliased icon buffer.
///
/// `view_box_size` is the width/height of the source coordinate system. Most
/// icon sets, including Phosphor, use a square view box.
pub fn rasterize_svg_path_icon(
    path: &str,
    size: u32,
    view_box_size: f32,
    color: RGBA,
) -> TheRGBABuffer {
    if size == 0 || view_box_size <= 0.0 {
        return TheRGBABuffer::empty();
    }

    let mut alpha = vec![0; (size * size) as usize];
    let scale = size as f32 / view_box_size;
    zeno::Mask::new(path)
        .transform(Some(zeno::Transform::scale(scale, scale)))
        .size(size, size)
        .render_into(&mut alpha, None);

    let mut pixels = Vec::with_capacity(alpha.len() * 4);
    for coverage in alpha {
        pixels.extend_from_slice(&[
            color[0],
            color[1],
            color[2],
            ((coverage as u16 * color[3] as u16) / 255) as u8,
        ]);
    }
    TheRGBABuffer::from(pixels, size, size)
}

pub mod prelude {
    pub use serde::{Deserialize, Serialize};

    pub use crate::theui::RGBA;

    pub use crate::theui::BLACK;
    pub use crate::theui::WHITE;

    pub use std::rc::Rc;

    pub use crate::theui::theid::TheId;
    pub use crate::theui::thepainter::*;

    pub use crate::theui::thecanvas::*;
    pub use crate::theui::thecodehighlighter::{TheCodeHighlighter, TheCodeHighlighterTrait};
    pub use crate::theui::thefeedback::*;

    pub use crate::theui::thergbbuffer::TheRGBBuffer;
    pub use crate::theui::thesizelimiter::TheSizeLimiter;
    pub use crate::theui::theuicontext::*;
    pub use crate::theui::{TheUI, rasterize_svg_path_icon};

    pub use crate::theui::thevalue::{TheValue, TheValueAssignment, TheValueComparison};
    pub use crate::theui::thevent::TheEvent;

    pub use crate::theui::thewidget::prelude::*;
    pub use crate::theui::thewidget::thecolorbutton::*;

    pub use crate::theui::thestyle::TheStyle;
    pub use crate::theui::thestyle::prelude::*;

    pub use crate::theui::thetheme::prelude::*;
    pub use crate::theui::thetheme::{
        TheTheme, TheThemeColors, TheThemeColors::*, TheThemeMetrics, TheThemeMetrics::*,
        TheThemePaints, TheThemePaints::*, TheThemePalettes, TheThemePalettes::*,
    };

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
    pub use crate::theui::TheAccelerator;
    pub use crate::theui::TheAcceleratorKey;
    pub use crate::theui::TheDialogButtonRole;
    pub use crate::theui::thecollection::TheCollection;
    pub use crate::theui::theflattenedmap::{TheFlattenedMap, TheFlattenedMap3D};
    pub use crate::theui::thetilemask::TheTileMask;
    pub use crate::theui::thetimeline::{TheInterpolation, TheTimeline};

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
        // We assume that accelerators are always case-insensitive.
        if self.key == key.to_ascii_lowercase() {
            if shift || ctrl || alt || logo {
                let mut ok = true;

                if (shift && !self.accel.contains(TheAcceleratorKey::SHIFT))
                    || (!shift && self.accel.contains(TheAcceleratorKey::SHIFT))
                {
                    ok = false;
                }
                if alt && !self.accel.contains(TheAcceleratorKey::ALT) {
                    ok = false;
                }

                let wants_ctrl = self.accel.contains(TheAcceleratorKey::CTRL);
                let wants_cmd = self.accel.contains(TheAcceleratorKey::CMD);
                if wants_ctrl || wants_cmd {
                    // For CTRLCMD we accept either Ctrl or Cmd, but at least one must be pressed.
                    if !(wants_ctrl && ctrl || wants_cmd && logo) {
                        ok = false;
                    }
                } else if ctrl || logo {
                    // No Ctrl/Cmd expected but one was pressed.
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

struct TheHoverHelp {
    id: TheId,
    text: String,
    started_at: Instant,
    anchor: Vec2<i32>,
    visible: bool,
}

const HOVER_HELP_HORIZONTAL_PADDING: usize = 18;
// Text rendering ellipsizes at an exact-width boundary, so reserve a small
// amount beyond the measured glyph width.
const HOVER_HELP_FIT_SLACK: usize = 2;

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

    hover_help: Option<TheHoverHelp>,
    hover_help_delay: Duration,

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
    pub right_mouse_down: bool,
    mouse_capture_id: Option<TheId>,
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

            hover_help: None,
            hover_help_delay: Duration::from_millis(1_250),

            context_menu: None,
            menu_widget_id: None,
            is_dirty: false,

            shift: false,
            ctrl: false,
            alt: false,
            logo: false,

            mouse_coord: Vec2::zero(),
            right_mouse_down: false,
            mouse_capture_id: None,
        }
    }

    /// Replaces the active theme and invalidates the full UI for repainting.
    pub fn set_theme(&mut self, theme: Box<dyn TheTheme>, ctx: &mut TheContext) {
        self.style.set_theme(theme);
        self.is_dirty = true;
        ctx.ui.redraw_all = true;
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

    /// Sets how long the pointer must remain over a control before hover help appears.
    pub fn set_hover_help_delay(&mut self, delay: Duration) {
        self.hover_help_delay = delay;
    }

    fn schedule_hover_help(&mut self, id: TheId, text: String, ctx: &mut TheContext) {
        if text.trim().is_empty() {
            self.clear_hover_help(ctx);
            return;
        }

        if self
            .hover_help
            .as_ref()
            .is_some_and(|help| help.id.uuid == id.uuid && help.text == text && !help.visible)
        {
            return;
        }

        let was_visible = self.hover_help.as_ref().is_some_and(|help| help.visible);
        self.hover_help = Some(TheHoverHelp {
            id,
            text,
            started_at: Instant::now(),
            anchor: self.mouse_coord,
            visible: false,
        });
        if was_visible {
            ctx.ui.redraw_all = true;
            self.is_dirty = true;
        }
    }

    fn clear_hover_help(&mut self, ctx: &mut TheContext) {
        if self.hover_help.take().is_some_and(|help| help.visible) {
            // Hover help is painted passively into the persistent canvas buffer.
            // Redraw the underlying UI to erase it.
            ctx.ui.redraw_all = true;
            self.is_dirty = true;
        }
    }

    pub fn relayout(&mut self, ctx: &mut TheContext) {
        let width = self.canvas.buffer().dim().width;
        let height = self.canvas.buffer().dim().height;
        self.canvas.layout(width, height, ctx);
        if let Some(dialog) = &mut self.dialog {
            let width = dialog.limiter.get_max_width();
            let height = dialog.limiter.get_max_height();
            dialog.layout(width, height, ctx);
        }
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
                #[cfg(not(target_arch = "wasm32"))]
                let system_text = arboard::Clipboard::new()
                    .ok()
                    .and_then(|mut clipboard| clipboard.get_text().ok());
                #[cfg(target_arch = "wasm32")]
                let system_text: Option<String> = None;

                let (value, app_type) = if let Some(text) = system_text {
                    (TheValue::Text(text), Some("text/plain".to_string()))
                } else {
                    (
                        ctx.ui.clipboard.clone().unwrap_or(TheValue::Empty),
                        ctx.ui.clipboard_app_type.clone(),
                    )
                };
                let event = TheEvent::Paste(value, app_type);
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
        self.draw_hover_help(ctx);
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

    fn hover_help_rect(
        anchor: Vec2<i32>,
        content_width: i32,
        content_height: i32,
        window_width: i32,
        window_height: i32,
    ) -> TheDim {
        let width = content_width.min((window_width - 8).max(1));
        let height = content_height.min((window_height - 8).max(1));
        let mut x = anchor.x + 12;
        let mut y = anchor.y + 18;
        if x + width + 4 > window_width {
            x = (window_width - width - 4).max(0);
        }
        if y + height + 4 > window_height {
            y = (anchor.y - height - 8).max(0);
        }
        x = x.clamp(0, (window_width - width).max(0));
        y = y.clamp(0, (window_height - height).max(0));
        TheDim::new(x, y, width, height)
    }

    fn wrap_hover_help_lines(
        text: &str,
        max_text_width: usize,
        draw: &TheDraw2D,
        font_settings: &TheFontSettings,
    ) -> Vec<String> {
        let mut lines = Vec::<String>::new();
        for paragraph in text.lines() {
            let mut line = String::new();
            for word in paragraph.split_whitespace() {
                let candidate = if line.is_empty() {
                    word.to_string()
                } else {
                    format!("{line} {word}")
                };
                if !line.is_empty()
                    && draw.get_text_size(&candidate, font_settings).0 > max_text_width
                {
                    lines.push(std::mem::take(&mut line));
                    line.push_str(word);
                } else {
                    line = candidate;
                }
            }
            if !line.is_empty() {
                lines.push(line);
            } else if paragraph.is_empty() {
                lines.push(String::new());
            }
        }
        lines
    }

    fn hover_help_content_width(longest_line: usize, max_box_width: usize) -> usize {
        (longest_line + HOVER_HELP_HORIZONTAL_PADDING + HOVER_HELP_FIT_SLACK)
            .clamp(120.min(max_box_width), max_box_width)
    }

    fn draw_hover_help(&mut self, ctx: &mut TheContext) {
        let Some(help) = self.hover_help.as_ref().filter(|help| help.visible) else {
            return;
        };

        let font_settings = TheFontSettings {
            size: 12.5,
            ..Default::default()
        };
        let max_box_width = ctx.width.saturating_sub(8).clamp(1, 360);
        let max_text_width = max_box_width
            .saturating_sub(HOVER_HELP_HORIZONTAL_PADDING + HOVER_HELP_FIT_SLACK)
            .max(1);
        let mut lines =
            Self::wrap_hover_help_lines(&help.text, max_text_width, &ctx.draw, &font_settings);
        if lines.is_empty() {
            return;
        }

        let available_height = (ctx.height as i32 - 8).max(1);
        let max_lines = ((available_height - 10) / 17).max(1) as usize;
        lines.truncate(max_lines);
        let longest = lines
            .iter()
            .map(|line| ctx.draw.get_text_size(line, &font_settings).0)
            .max()
            .unwrap_or(1);
        let content_width = Self::hover_help_content_width(longest, max_box_width) as i32;
        let content_height = lines.len() as i32 * 17 + 10;
        let dim = Self::hover_help_rect(
            help.anchor,
            content_width,
            content_height,
            ctx.width as i32,
            ctx.height as i32,
        );
        if !dim.is_valid() {
            return;
        }

        let mut tooltip = TheRGBABuffer::new(TheDim::new(0, 0, dim.width, dim.height));
        let stride = dim.width as usize;
        ctx.draw.rect(
            tooltip.pixels_mut(),
            &(0, 0, dim.width as usize, dim.height as usize),
            stride,
            self.style.theme().color(ContextMenuBackground),
        );
        ctx.draw.rect_outline(
            tooltip.pixels_mut(),
            &(0, 0, dim.width as usize, dim.height as usize),
            stride,
            self.style.theme().color(ContextMenuBorder),
        );
        let text_color = *self.style.theme().color(ContextMenuTextNormal);
        for (line_index, line) in lines.iter().enumerate() {
            ctx.draw.text_rect_blend(
                tooltip.pixels_mut(),
                &(
                    9,
                    5 + line_index * 17,
                    dim.width
                        .saturating_sub(HOVER_HELP_HORIZONTAL_PADDING as i32)
                        as usize,
                    17,
                ),
                stride,
                line,
                TheFontSettings {
                    size: font_settings.size,
                    ..Default::default()
                },
                &text_color,
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );
        }
        self.canvas.buffer.blend_into(dim.x, dim.y, &tooltip);
    }

    /// Processes widget state events, these are mostly send from TheUIContext based on state changes provided by the widgets.
    pub fn process_events(&mut self, ctx: &mut TheContext) {
        // Temporarily own the receiver so event handlers can freely mutate the UI
        // (for example when scheduling or clearing passive hover help).
        if let Some(receiver) = self.state_events_receiver.take() {
            while let Ok(event) = receiver.try_recv() {
                // Resend event to all app listeners
                for (name, sender) in &self.app_state_events {
                    sender.send(event.clone()).unwrap();
                }

                match event {
                    TheEvent::SetClipboard(value, app_type) => {
                        #[cfg(not(target_arch = "wasm32"))]
                        if let TheValue::Text(text) = &value
                            && let Ok(mut clipboard) = arboard::Clipboard::new()
                        {
                            let _ = clipboard.set_text(text.clone());
                        }

                        ctx.ui.clipboard = Some(value);
                        ctx.ui.clipboard_app_type = app_type;
                        ctx.ui.send(TheEvent::ClipboardChanged);
                    }
                    TheEvent::ShowMenu(id, coord, mut menu) => {
                        self.clear_hover_help(ctx);
                        menu.set_position(coord, ctx);
                        menu.id = id.clone();
                        self.context_menu = Some(menu);
                        self.menu_widget_id = Some(id.clone());
                    }
                    TheEvent::ShowContextMenu(id, coord, mut menu) => {
                        self.clear_hover_help(ctx);
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
                        let hover_text = self
                            .canvas
                            .get_widget(None, Some(&id.uuid))
                            .and_then(|widget| widget.status_text());
                        if let Some(text) = hover_text {
                            self.schedule_hover_help(id.clone(), text, ctx);
                        } else {
                            self.clear_hover_help(ctx);
                        }
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
                        if self
                            .hover_help
                            .as_ref()
                            .is_some_and(|help| help.id.uuid == id.uuid)
                        {
                            self.clear_hover_help(ctx);
                        }
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
                    TheEvent::SetStatusText(id, text) => {
                        if !id.name.is_empty()
                            && ctx
                                .ui
                                .hover
                                .as_ref()
                                .is_some_and(|hover| hover.uuid == id.uuid)
                        {
                            self.schedule_hover_help(id, text.clone(), ctx);
                        }
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
            self.state_events_receiver = Some(receiver);
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
        if let Some(help) = &mut self.hover_help
            && !help.visible
            && help.started_at.elapsed() >= self.hover_help_delay
        {
            help.visible = true;
            self.is_dirty = true;
        }
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

        self.clear_hover_help(ctx);

        ctx.ui.send(TheEvent::MouseDown(coord));

        //ctx.ui.clear_focus();

        if let Some(context) = &mut self.context_menu {
            if context.contains(coord) {
                let event = TheEvent::MouseDown(context.dim.to_local(coord));
                if context.on_event(&event, ctx) {
                    redraw = true;
                    if let Some((menu_id, menu_item_id)) = context.get_hovered_id() {
                        let event_menu_id = self.menu_widget_id.clone().unwrap_or(menu_id.clone());
                        ctx.ui.send(TheEvent::ContextMenuSelected(
                            event_menu_id,
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

        self.mouse_capture_id = None;

        if let Some(id) = &ctx.ui.overlay {
            let overlay_hit =
                if let Some(widget) = self.get_widget_abs(Some(&id.name), Some(&id.uuid)) {
                    Some((widget.id().clone(), widget.dim().to_local(coord)))
                } else {
                    None
                };
            if let Some((widget_id, local_coord)) = overlay_hit {
                self.mouse_capture_id = Some(widget_id.clone());
                if let Some(widget) = self.get_widget_abs(None, Some(&widget_id.uuid)) {
                    let event = TheEvent::MouseDown(local_coord);
                    redraw = widget.on_event(&event, ctx);
                }
                self.process_events(ctx);
                return redraw;
            }
        }

        let mut mouse_capture_id: Option<TheId> = None;
        if let Some(widget) = self.get_widget_at_coord(coord) {
            mouse_capture_id = Some(widget.id().clone());
            let event = TheEvent::MouseDown(widget.dim().to_local(coord));
            redraw = widget.on_event(&event, ctx);

            self.process_events(ctx);
        }
        self.mouse_capture_id = mouse_capture_id;
        redraw
    }

    pub fn touch_dragged(&mut self, x: f32, y: f32, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        let coord = Vec2::new(x as i32, y as i32);

        self.clear_hover_help(ctx);

        if let Some(context) = &mut self.context_menu {
            return redraw;
        }

        if let Some(id) = &ctx.ui.overlay {
            if let Some(widget) = self.get_widget_abs(None, Some(&id.uuid)) {
                let event = TheEvent::MouseDragged(widget.dim().to_local(coord));
                redraw = widget.on_event(&event, ctx);
                self.process_events(ctx);
            }
        } else if let Some(id) = self.mouse_capture_id.clone() {
            if let Some(widget) = self.get_widget_abs(None, Some(&id.uuid)) {
                let event = TheEvent::MouseDragged(widget.dim().to_local(coord));
                redraw = widget.on_event(&event, ctx);
                self.process_events(ctx);
            } else {
                self.mouse_capture_id = None;
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
        } else if let Some(id) = self.mouse_capture_id.clone() {
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
        self.mouse_capture_id = None;
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
        self.mouse_wheel_with_event((delta.0 as f32, delta.1 as f32), false, ctx)
    }

    /// Routes precise scrolling as a distinct event only when it lands on a
    /// render view. Scrollable UI layouts and all other widgets retain their
    /// established mouse-wheel behavior.
    pub fn precise_scroll(&mut self, delta: (f32, f32), ctx: &mut TheContext) -> bool {
        self.mouse_wheel_with_event(delta, true, ctx)
    }

    fn mouse_wheel_with_event(
        &mut self,
        delta: (f32, f32),
        precise: bool,
        ctx: &mut TheContext,
    ) -> bool {
        let mut redraw = false;

        self.clear_hover_help(ctx);

        let mut layout_id = None;
        if let Some(id) = self.get_layout_at_coord(self.mouse_coord) {
            layout_id = Some(id);
        }

        let mut processed = false;

        // We check first if the layout under the mouse supports manual scrolling, and if yes use that
        if let Some(layout_id) = layout_id {
            if let Some(layout) = self.get_layout(&layout_id.name) {
                if layout.supports_mouse_wheel() {
                    layout.mouse_wheel_scroll(Vec2::new(delta.0 as i32, delta.1 as i32));
                    processed = true;
                    redraw = true;
                }
            }
        }

        if !processed {
            // If not processed, call the widget directly.
            if let Some(id) = &ctx.ui.hover {
                if let Some(widget) = self.get_widget_abs(Some(&id.name), Some(&id.uuid)) {
                    let is_render_view = widget.as_render_view().is_some();
                    let event = if precise && is_render_view {
                        TheEvent::PreciseScroll(Vec2::new(delta.0, delta.1))
                    } else {
                        TheEvent::MouseWheel(Vec2::new(delta.0 as i32, delta.1 as i32))
                    };
                    redraw = widget.on_event(&event, ctx);
                    self.process_events(ctx);
                }
            }
        }
        redraw
    }

    /// Sends a pinch gesture only to the render view under the pointer.
    pub fn pinch(&mut self, delta: f32, ctx: &mut TheContext) -> bool {
        self.clear_hover_help(ctx);
        let Some(id) = &ctx.ui.hover else {
            return false;
        };
        let Some(widget) = self.get_widget_abs(Some(&id.name), Some(&id.uuid)) else {
            return false;
        };
        if widget.as_render_view().is_none() {
            return false;
        }
        let redraw = widget.on_event(&TheEvent::Pinch(delta), ctx);
        self.process_events(ctx);
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
        let mut suppress_focused_widget = false;

        if let Some(c) = char {
            if self.ctrl || self.alt || self.logo {
                // Local text editing shortcuts must override global accelerators.
                // In particular Cmd/Ctrl+A should select-all in focused editors
                // instead of triggering app-level actions like "Save As".
                let focused_text_input_wants_select_all = if (self.ctrl || self.logo)
                    && c.to_ascii_lowercase() == 'a'
                {
                    if let Some(id) = &ctx.ui.focus {
                        if let Some(widget) = self.get_widget_abs(Some(&id.name), Some(&id.uuid)) {
                            widget.supports_text_input()
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };
                let focused_text_input = if let Some(id) = &ctx.ui.focus {
                    if let Some(widget) = self.get_widget_abs(Some(&id.name), Some(&id.uuid)) {
                        widget.supports_text_input()
                    } else {
                        false
                    }
                } else {
                    false
                };
                let focused_text_input_wants_local_shortcut = focused_text_input
                    && (self.ctrl || self.logo)
                    && matches!(c.to_ascii_lowercase(), 'a' | '+' | '-' | '/');

                if focused_text_input_wants_select_all || focused_text_input_wants_local_shortcut {
                    consumed = false;
                } else {
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
                    if !consumed && focused_text_input {
                        suppress_focused_widget = true;
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
            if !suppress_focused_widget && let Some(id) = &ctx.ui.focus {
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

    pub fn get_layout_abs(
        &mut self,
        name: Option<&String>,
        uuid: Option<&Uuid>,
    ) -> Option<&mut Box<dyn TheLayout>> {
        if let Some(dialog) = &mut self.dialog {
            dialog.get_layout(name, uuid)
        } else {
            self.canvas.get_layout(name, uuid)
        }
    }

    /// Gets a given widget by name
    pub fn get_widget(&mut self, name: &str) -> Option<&mut Box<dyn TheWidget>> {
        self.get_widget_abs(Some(&name.to_string()), None)
    }

    /// Gets a given widget by id
    pub fn get_widget_id(&mut self, id: Uuid) -> Option<&mut Box<dyn TheWidget>> {
        self.get_widget_abs(None, Some(&id))
    }

    /// Gets a given text line edit by name
    pub fn get_text_line_edit(&mut self, name: &str) -> Option<&mut dyn TheTextLineEditTrait> {
        if let Some(text_line_edit) = self.get_widget_abs(Some(&name.to_string()), None) {
            return text_line_edit.as_text_line_edit();
        }
        None
    }

    /// Gets a given text area edit by name
    pub fn get_text_area_edit(&mut self, name: &str) -> Option<&mut dyn TheTextAreaEditTrait> {
        if let Some(text_area_edit) = self.get_widget_abs(Some(&name.to_string()), None) {
            return text_area_edit.as_text_area_edit();
        }
        None
    }

    /// Gets a given text view by name
    pub fn get_text_view(&mut self, name: &str) -> Option<&mut dyn TheTextViewTrait> {
        if let Some(text_view) = self.get_widget_abs(Some(&name.to_string()), None) {
            return text_view.as_text_view();
        }
        None
    }

    /// Gets a given icon view by name
    pub fn get_icon_view(&mut self, name: &str) -> Option<&mut dyn TheIconViewTrait> {
        if let Some(text_line_edit) = self.get_widget_abs(Some(&name.to_string()), None) {
            return text_line_edit.as_icon_view();
        }
        None
    }

    /// Gets a given icon grid view by name.
    pub fn get_icon_grid_view(&mut self, name: &str) -> Option<&mut dyn TheIconGridViewTrait> {
        if let Some(icon_grid) = self.get_widget_abs(Some(&name.to_string()), None) {
            return icon_grid.as_icon_grid_view();
        }
        None
    }

    /// Gets a given menu by name
    pub fn get_menu(&mut self, name: &str) -> Option<&mut dyn TheMenuTrait> {
        if let Some(menu) = self.get_widget_abs(Some(&name.to_string()), None) {
            return menu.as_menu();
        }
        None
    }

    /// Gets a given render view by name
    pub fn get_render_view(&mut self, name: &str) -> Option<&mut dyn TheRenderViewTrait> {
        if let Some(render_view) = self.get_widget_abs(Some(&name.to_string()), None) {
            return render_view.as_render_view();
        }
        None
    }

    /// Gets a given text by name
    pub fn get_text(&mut self, name: &str) -> Option<&mut dyn TheTextTrait> {
        if let Some(text) = self.get_widget_abs(Some(&name.to_string()), None) {
            return text.as_text();
        }
        None
    }

    /// Gets a given group button by name
    pub fn get_group_button(&mut self, name: &str) -> Option<&mut dyn TheGroupButtonTrait> {
        if let Some(text) = self.get_widget_abs(Some(&name.to_string()), None) {
            return text.as_group_button();
        }
        None
    }

    /// Gets a given statusbar by name
    pub fn get_statusbar(&mut self, name: &str) -> Option<&mut dyn TheStatusbarTrait> {
        if let Some(text) = self.get_widget_abs(Some(&name.to_string()), None) {
            return text.as_statusbar();
        }
        None
    }

    /// Gets a given drop down menu by name
    pub fn get_drop_down_menu(&mut self, name: &str) -> Option<&mut dyn TheDropdownMenuTrait> {
        if let Some(drop_down_menu) = self.get_widget_abs(Some(&name.to_string()), None) {
            return drop_down_menu.as_drop_down_menu();
        }
        None
    }

    /// Gets a given time slider by name
    pub fn get_time_slider(&mut self, name: &str) -> Option<&mut dyn TheTimeSliderTrait> {
        if let Some(text) = self.get_widget_abs(Some(&name.to_string()), None) {
            return text.as_time_slider();
        }
        None
    }

    /// Gets a given palette picker by name
    pub fn get_palette_picker(&mut self, name: &str) -> Option<&mut dyn ThePalettePickerTrait> {
        if let Some(text) = self.get_widget_abs(Some(&name.to_string()), None) {
            return text.as_palette_picker();
        }
        None
    }

    /// Gets a given layout by name
    pub fn get_layout(&mut self, name: &str) -> Option<&mut Box<dyn TheLayout>> {
        self.get_layout_abs(Some(&name.to_string()), None)
    }

    /// Relayouts the given layout.
    pub fn relayout_layout(&mut self, name: &str, ctx: &mut TheContext) {
        if let Some(l) = self.get_layout_abs(Some(&name.to_string()), None) {
            l.relayout(ctx);
        }
    }

    /// Gets a given TheListLayout by name
    pub fn get_list_layout(&mut self, name: &str) -> Option<&mut dyn TheListLayoutTrait> {
        if let Some(text_line_edit) = self.get_layout_abs(Some(&name.to_string()), None) {
            return text_line_edit.as_list_layout();
        }
        None
    }

    /// Gets a given TheTreeLayout by name
    pub fn get_tree_layout(&mut self, name: &str) -> Option<&mut dyn TheTreeLayoutTrait> {
        if let Some(layout) = self.get_layout_abs(Some(&name.to_string()), None) {
            return layout.as_tree_layout();
        }
        None
    }

    /// Gets a given TheRowListLayout by name
    pub fn get_rowlist_layout(&mut self, name: &str) -> Option<&mut dyn TheRowListLayoutTrait> {
        if let Some(text_line_edit) = self.get_layout_abs(Some(&name.to_string()), None) {
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

    pub fn set_node_overlay_tiled(&mut self, name: &str, tiled: bool) {
        if let Some(view) = self.canvas.get_widget(Some(&name.to_string()), None) {
            if let Some(nodes) = view.as_node_canvas_view() {
                nodes.set_overlay_tiled(tiled);
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

#[cfg(test)]
mod hover_help_tests {
    use super::*;

    #[test]
    fn vector_icon_rasterization_keeps_the_canvas_transparent() {
        let icon =
            rasterize_svg_path_icon("M32,32H224V224H32Z M64,64V192H192V64Z", 18, 256.0, WHITE);
        assert_eq!((icon.dim().width, icon.dim().height), (18, 18));
        assert_eq!(
            icon.pixels()[3],
            0,
            "top-left canvas pixel must be transparent"
        );
        assert!(
            icon.pixels().chunks_exact(4).any(|pixel| pixel[3] > 0),
            "the path must produce covered pixels"
        );
    }

    #[test]
    fn hover_help_is_clamped_to_normal_and_tiny_windows() {
        for (window_width, window_height) in [(220, 120), (3, 2)] {
            let rect = TheUI::hover_help_rect(
                Vec2::new(window_width - 1, window_height - 1),
                360,
                80,
                window_width,
                window_height,
            );
            assert!(rect.x >= 0);
            assert!(rect.y >= 0);
            assert!(rect.x + rect.width <= window_width);
            assert!(rect.y + rect.height <= window_height);
        }
    }

    #[test]
    fn repeated_dynamic_help_does_not_restart_its_delay() {
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(320, 200, 1.0);
        let id = TheId::named("Group");
        ui.schedule_hover_help(id.clone(), "Camera".into(), &mut ctx);
        let started_at = ui.hover_help.as_ref().unwrap().started_at;
        ui.schedule_hover_help(id, "Camera".into(), &mut ctx);
        assert_eq!(ui.hover_help.as_ref().unwrap().started_at, started_at);
    }

    #[test]
    fn hover_help_wraps_to_measured_pixel_width() {
        let draw = TheDraw2D::new();
        let settings = TheFontSettings {
            size: 12.5,
            ..Default::default()
        };
        let max_width = 150;
        let lines = TheUI::wrap_hover_help_lines(
            "Runtime diagnostics use the available tooltip width without an estimated character gutter.",
            max_width,
            &draw,
            &settings,
        );

        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| {
            draw.get_text_size(line, &settings).0 <= max_width
                || line.split_whitespace().count() == 1
        }));

        let longest = lines
            .iter()
            .map(|line| draw.get_text_size(line, &settings).0)
            .max()
            .unwrap();
        let content_width = TheUI::hover_help_content_width(longest, 360);
        let text_rect_width = content_width - HOVER_HELP_HORIZONTAL_PADDING;
        assert!(longest < text_rect_width);
    }
}
