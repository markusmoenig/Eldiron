use crate::prelude::*;

use std::time::{SystemTime, UNIX_EPOCH};

pub struct StatusBar {
    pub rect                    : (usize, usize, usize, usize),

    pub dirty                   : bool,
    pub buffer                  : Vec<u8>,

    pub size                    : (usize, usize),
    text                        : String,

    start_time                  : u128,

    message_to_add              : Option<String>
}

impl StatusBar {

    pub fn new() -> Self {

        Self {
            rect                : (0,0,0,0),

            dirty               : true,
            buffer              : vec![],

            size                : (500, 30),

            text                : "".to_string(),

            start_time          : 0,

            message_to_add      : None,
        }
    }

    /// Draw the node
    pub fn draw(&mut self, frame: &mut [u8], _anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        if let Some(message) = &self.message_to_add {
            self.text = message.clone();
            self.start_time = self.get_time();
            self.size.0 = context.draw2d.get_text_size(&asset.get_editor_font("OpenSans"), 18.0, &self.text.as_str()).0 + 20;
            self.dirty = true;
            self.message_to_add = None;
        }

        if self.text.is_empty() {
            return;
        }

        if self.get_time() - self.start_time > 4000 {
            self.text = "".to_string();
            return;
        }

        if self.buffer.len() != self.size.0 * self.size.1 * 4 {
            self.buffer = vec![0;self.size.0 * self.size.1 * 4];
        }

        let rect = (0, 0, self.size.0, self.size.1);

        if self.dirty {
            for i in &mut self.buffer[..] { *i = 0 }
            let buffer_frame = &mut self.buffer[..];
            let stride = self.size.0;

            context.draw2d.draw_rect(buffer_frame, &(0, 0, rect.2, rect.3), stride, &context.color_black);
            context.draw2d.draw_text(buffer_frame, &(10, 1), rect.2, &asset.get_editor_font("OpenSans"), 18.0, &self.text, &context.color_white, &context.color_black);
        }
        context.draw2d.copy_slice(frame, &self.buffer[..], &(self.rect.0, context.height - self.size.1, self.size.0, self.size.1), context.width);
        self.dirty = false;

    }

    // Clears the content
    pub fn _clear(&mut self) {
        self.text = "".to_string();
        self.start_time = 0;
    }

    /// Gets the current time in milliseconds
    fn get_time(&self) -> u128 {
        let stop = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
            stop.as_millis()
    }

    /// Add a new message
    pub fn add_message(&mut self, message: String) {
        self.message_to_add = Some(message);
    }

    /// Returns true if we are displaying a message
    pub fn _has_message(&self) -> bool {
        self.text.len() > 0
    }
}