use crate::prelude::*;
use crate::Embedded;
use fontdue::Font;

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self};

/// TheFileExtension is used to specify supported file extensions in the load and save file requesters.
pub struct TheFileExtension {
    pub name: String,
    pub extensions: Vec<String>,
}

impl TheFileExtension {
    pub fn new(name: String, extensions: Vec<String>) -> Self {
        Self { name, extensions }
    }
}

pub struct TheUIContext {
    pub font: Option<Font>,
    icons: FxHashMap<String, TheRGBABuffer>,

    pub focus: Option<TheId>,
    pub keyboard_focus: Option<TheId>,
    pub hover: Option<TheId>,
    pub overlay: Option<TheId>,
    pub context_menu: Option<TheContextMenu>,

    pub disabled_ids: FxHashSet<String>,

    pub state_events_sender: Option<Sender<TheEvent>>,

    pub redraw_all: bool,
    pub relayout: bool,

    pub undo_stack: TheUndoStack,

    pub drop: Option<TheDrop>,

    pub file_requester_receiver: Option<(TheId, Receiver<Vec<PathBuf>>)>,

    pub accelerators: FxHashMap<TheId, TheAccelerator>,

    pub clipboard: Option<TheValue>,
    pub clipboard_app_type: Option<String>,
}

impl Default for TheUIContext {
    fn default() -> Self {
        Self::new()
    }
}

impl TheUIContext {
    pub fn new() -> Self {
        let mut font: Option<Font> = None;
        let mut icons: FxHashMap<String, TheRGBABuffer> = FxHashMap::default();

        for file in Embedded::iter() {
            let name = file.as_ref();

            if name.starts_with("fonts/Roboto-Bold") {
                if let Some(font_bytes) = Embedded::get(name) {
                    if let Ok(f) =
                        Font::from_bytes(font_bytes.data, fontdue::FontSettings::default())
                    {
                        font = Some(f);
                    }
                }
            } else if name.starts_with("icons/") {
                if let Some(file) = Embedded::get(name) {
                    let data = std::io::Cursor::new(file.data);

                    let decoder = png::Decoder::new(data);
                    if let Ok(mut reader) = decoder.read_info() {
                        let mut buf = vec![0; reader.output_buffer_size()];
                        let info = reader.next_frame(&mut buf).unwrap();
                        let bytes = &buf[..info.buffer_size()];

                        // Ensure the image data has 4 channels (RGBA)
                        let rgba_bytes = if info.color_type.samples() == 3 {
                            // Image is RGB, expand to RGBA
                            let mut expanded_buf =
                                Vec::with_capacity(info.width as usize * info.height as usize * 4);
                            for chunk in bytes.chunks(3) {
                                expanded_buf.push(chunk[0]); // R
                                expanded_buf.push(chunk[1]); // G
                                expanded_buf.push(chunk[2]); // B
                                expanded_buf.push(255); // A (opaque)
                            }
                            expanded_buf
                        } else {
                            // Image is already RGBA
                            bytes.to_vec()
                        };

                        let mut cut_name = name.replace("icons/", "");
                        cut_name = cut_name.replace(".png", "");
                        icons.insert(
                            cut_name.to_string(),
                            TheRGBABuffer::from(rgba_bytes, info.width, info.height),
                        );
                    }
                }
            }
        }

        Self {
            focus: None,
            keyboard_focus: None,
            hover: None,
            overlay: None,
            context_menu: None,

            font,
            icons,

            disabled_ids: FxHashSet::default(),
            state_events_sender: None,

            redraw_all: false,
            relayout: false,

            undo_stack: TheUndoStack::default(),

            drop: None,

            file_requester_receiver: None,

            accelerators: FxHashMap::default(),

            clipboard: None,
            clipboard_app_type: None,
        }
    }

    /// Set the given id as disabled.
    pub fn set_disabled(&mut self, id: &str) {
        self.disabled_ids.insert(id.to_string());
        self.set_widget_state(id.to_string(), TheWidgetState::None);
    }

    /// Check if the given id is disabled.
    pub fn is_disabled(&self, id: &str) -> bool {
        self.disabled_ids.contains(id)
    }

    /// Remove the given id from the disabled list.
    pub fn set_enabled(&mut self, id: &str) {
        self.disabled_ids.remove(id);
        self.set_widget_state(id.to_string(), TheWidgetState::None);
    }

    /// Adds an icon to the library.
    pub fn add_icon(&mut self, name: String, icon: TheRGBABuffer) {
        self.icons.insert(name, icon);
    }

    /// Returns an icon of the given name from the embedded style icons
    pub fn icon(&self, name: &str) -> Option<&TheRGBABuffer> {
        if let Some(icon) = self.icons.get(name) {
            return Some(icon);
        }
        None
    }

    /// Sets the focus to the given widget
    pub fn set_focus(&mut self, id: &TheId) {
        if !id.equals(&self.focus) {
            if let Some(focus) = &self.focus {
                self.send(TheEvent::LostFocus(focus.clone()));
            }
            self.send(TheEvent::GainedFocus(id.clone()));
            self.focus = Some(id.clone());
        }
    }

    /// Clears the focus state.
    pub fn clear_focus(&mut self) {
        self.focus = None;
    }

    /// Checks if the given id has focus
    pub fn has_focus(&self, id: &TheId) -> bool {
        id.equals(&self.focus)
    }

    /// Sets the hover to the given widget
    pub fn set_hover(&mut self, id: &TheId) {
        if !id.equals(&self.hover) {
            if let Some(hover) = &self.hover {
                self.send(TheEvent::LostHover(hover.clone()));
            }
            self.send(TheEvent::GainedHover(id.clone()));
            self.hover = Some(id.clone());
        }
    }

    /// Clears the hover state.
    pub fn clear_hover(&mut self) {
        self.hover = None;
    }

    /// Sets the overlay to the given widget. This will call the draw_overlay method of the widget after all other draw calls (for menus etc).
    pub fn set_overlay(&mut self, id: &TheId) {
        self.overlay = Some(id.clone());
    }

    /// Clears
    pub fn clear_overlay(&mut self) {
        self.overlay = None;
        self.redraw_all = true;
    }

    /// Sets the drop to the given value.
    pub fn set_drop(&mut self, drop: TheDrop) {
        self.drop = Some(drop);
    }

    /// Clears the drop state.
    pub fn clear_drop(&mut self) {
        self.drop = None;
    }

    /// Checks if there is currently a drop operation.
    pub fn has_drop(&self) -> bool {
        self.drop.is_some()
    }

    /// Indicates that the state of the given widget changed
    pub fn send_widget_state_changed(&mut self, id: &TheId, state: TheWidgetState) {
        self.send(TheEvent::StateChanged(id.clone(), state));
    }

    pub fn set_widget_state(&mut self, name: String, state: TheWidgetState) {
        self.send(TheEvent::SetState(name, state));
    }

    pub fn set_widget_state_id(&mut self, id: Uuid, state: TheWidgetState) {
        self.send(TheEvent::SetStateId(id, state));
    }

    /// Sends the given state event
    pub fn send(&mut self, event: TheEvent) {
        if let Some(sender) = &mut self.state_events_sender {
            sender.send(event).unwrap();
        }
    }

    /// Indicates that the state of the given widget changed
    pub fn send_widget_value_changed(&mut self, id: &TheId, value: TheValue) {
        self.send(TheEvent::ValueChanged(id.clone(), value));
    }

    /// Opens a file requester with the given title and extensions. Upon completion a TheEvent::FileRequesterResult event will be send.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_file_requester(&mut self, id: TheId, title: String, extension: TheFileExtension) {
        let (tx, rx): (Sender<Vec<PathBuf>>, Receiver<Vec<PathBuf>>) = mpsc::channel();

        let task = rfd::AsyncFileDialog::new()
            .add_filter(extension.name, &extension.extensions)
            .set_title(title)
            .pick_files();

        std::thread::spawn(move || {
            let files = futures::executor::block_on(task);

            if let Some(files) = files {
                let mut ff = vec![];
                for f in files {
                    ff.push(f.path().to_path_buf());
                }
                tx.send(ff).unwrap();
            } else {
                tx.send(vec![]).unwrap();
            }
        });

        self.file_requester_receiver = Some((id, rx));
    }

    /// Opens a save file requester with the given title and extensions. Upon completion a TheEvent::FileRequesterResult event will be send.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_file_requester(&mut self, id: TheId, title: String, extension: TheFileExtension) {
        let (tx, rx): (Sender<Vec<PathBuf>>, Receiver<Vec<PathBuf>>) = mpsc::channel();

        let task = rfd::AsyncFileDialog::new()
            .add_filter(extension.name, &extension.extensions)
            .set_title(title)
            .save_file();

        std::thread::spawn(move || {
            let file = futures::executor::block_on(task);

            if let Some(file) = file {
                let ff = vec![file.path().to_path_buf()];
                tx.send(ff).unwrap();
            } else {
                tx.send(vec![]).unwrap();
            }
        });

        self.file_requester_receiver = Some((id, rx));
    }

    /// Decode image
    pub fn decode_image(&mut self, id: TheId, path: PathBuf) {
        match image::open(&path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (width, height) = rgba.dimensions();
                let buffer = TheRGBABuffer::from(rgba.into_vec(), width, height);
                let name = path.file_stem().and_then(|f| f.to_str()).unwrap_or("");

                self.send(TheEvent::ImageDecodeResult(id, name.to_string(), buffer));
            }
            Err(err) => {
                eprintln!("Failed to decode image {}: {}", path.to_string_lossy(), err);
            }
        }
    }
}
