use crate::prelude::*;

/// The cursor icon to display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TheCursorIcon {
    Default,
    Crosshair,
    Hand,
    Arrow,
    Text,
    Wait,
    Help,
    Progress,
    NotAllowed,
    ContextMenu,
    Cell,
    VerticalText,
    Alias,
    Copy,
    NoDrop,
    Grab,
    Grabbing,
    AllScroll,
    ZoomIn,
    ZoomOut,
    EResize,
    NResize,
    NEResize,
    NWResize,
    SResize,
    SEResize,
    SWResize,
    WResize,
    EWResize,
    NSResize,
    NESWResize,
    NWSEResize,
    ColResize,
    RowResize,
}

pub struct TheContext {
    pub width: usize,
    pub height: usize,
    pub scale_factor: f32,

    pub draw: TheDraw2D,
    #[cfg(feature = "ui")]
    pub ui: TheUIContext,

    pub cursor_icon: TheCursorIcon,
    pub cursor_changed: bool,
    pub cursor_visible: bool,
    pub cursor_visible_changed: bool,
}

impl TheContext {
    pub fn new(width: usize, height: usize, scale_factor: f32) -> Self {
        Self {
            width,
            height,
            scale_factor,
            draw: TheDraw2D::new(),
            #[cfg(feature = "ui")]
            ui: TheUIContext::new(),
            cursor_icon: TheCursorIcon::Default,
            cursor_changed: false,
            cursor_visible: true,
            cursor_visible_changed: false,
        }
    }

    /// Set the cursor icon
    pub fn set_cursor_icon(&mut self, icon: TheCursorIcon) {
        if self.cursor_icon != icon {
            self.cursor_icon = icon;
            self.cursor_changed = true;
        }
    }

    /// Get the current cursor icon
    pub fn cursor_icon(&self) -> TheCursorIcon {
        self.cursor_icon
    }

    /// Check if the cursor has changed
    pub fn cursor_changed(&self) -> bool {
        self.cursor_changed
    }

    /// Reset the cursor changed flag
    pub fn reset_cursor_changed(&mut self) {
        self.cursor_changed = false;
    }

    /// Set the cursor visibility
    pub fn set_cursor_visible(&mut self, visible: bool) {
        if self.cursor_visible != visible {
            self.cursor_visible = visible;
            self.cursor_visible_changed = true;
        }
    }

    /// Get the current cursor visibility
    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// Check if the cursor visibility has changed
    pub fn cursor_visible_changed(&self) -> bool {
        self.cursor_visible_changed
    }

    /// Reset the cursor visibility changed flag
    pub fn reset_cursor_visible_changed(&mut self) {
        self.cursor_visible_changed = false;
    }

    /// Gets the current time in milliseconds.
    pub fn get_time(&self) -> u128 {
        let time;
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            let t = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            time = t.as_millis();
        }
        #[cfg(target_arch = "wasm32")]
        {
            time = web_sys::window().unwrap().performance().unwrap().now() as u128;
        }
        time
    }

    #[cfg(feature = "i18n")]
    pub fn load_system_fonts(&mut self, fonts: Vec<TheFontScript>) {
        use std::fs::read;

        use font_kit::{
            family_name::FamilyName, handle::Handle, properties::Properties, source::SystemSource,
        };

        if fonts.is_empty() {
            return;
        }

        let source = SystemSource::new();

        for font in fonts {
            for font_name in font.fonts() {
                if let Ok(handle) = source.select_best_match(
                    &[FamilyName::Title(font_name.to_string())],
                    &Properties::new(),
                ) {
                    let buf = match handle {
                        Handle::Memory { bytes, .. } => Some(bytes.to_vec()),
                        Handle::Path { path, .. } => read(path).ok(),
                    };

                    if let Some(buf) = buf {
                        self.draw.add_font_data(buf);

                        break;
                    }
                }
            }
        }
    }
}
