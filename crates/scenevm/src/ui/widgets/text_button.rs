use uuid::Uuid;
use vek::Vec4;

use crate::ui::{
    ButtonKind, ButtonStyle, HAlign, VAlign,
    drawable::Drawable,
    event::{UiAction, UiEvent, UiEventKind, UiEventOutcome},
    workspace::{UiView, ViewContext},
};

/// A button with integrated text label - combines Button + LabelRect for convenience.
#[derive(Debug, Clone)]
pub struct TextButton {
    pub id: String,
    pub style: ButtonStyle,
    pub kind: ButtonKind,
    pub text: String,
    pub text_size: f32,
    pub text_color: Vec4<f32>,
    pub h_align: HAlign,
    pub v_align: VAlign,

    // State
    toggled: bool,
    active_pointer: Option<u32>,
}

impl TextButton {
    pub fn new(style: ButtonStyle, text: impl Into<String>) -> Self {
        let text_color = style.text_color;
        Self {
            id: String::new(),
            style,
            kind: ButtonKind::Momentary,
            text: text.into(),
            text_size: 16.0,
            text_color,
            h_align: HAlign::Center,
            v_align: VAlign::Center,
            toggled: false,
            active_pointer: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_kind(mut self, kind: ButtonKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_text_size(mut self, size: f32) -> Self {
        self.text_size = size;
        self
    }

    pub fn with_text_color(mut self, color: Vec4<f32>) -> Self {
        self.text_color = color;
        self
    }

    pub fn with_alignment(mut self, h: HAlign, v: VAlign) -> Self {
        self.h_align = h;
        self.v_align = v;
        self
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub fn is_toggled(&self) -> bool {
        self.toggled
    }

    pub fn set_toggled(&mut self, toggled: bool) {
        self.toggled = toggled;
    }

    fn is_pressed(&self) -> bool {
        self.active_pointer.is_some()
    }

    fn hit(&self, pos: [f32; 2]) -> bool {
        let [x, y, w, h] = self.style.rect;
        pos[0] >= x && pos[0] <= x + w && pos[1] >= y && pos[1] <= y + h
    }

    fn current_fill_border(&self) -> (Vec4<f32>, Vec4<f32>) {
        let active = match self.kind {
            ButtonKind::Momentary => self.is_pressed(),
            ButtonKind::Toggle => self.toggled,
        };

        if active {
            (self.style.pressed_fill, self.style.pressed_border)
        } else {
            (self.style.fill, self.style.border)
        }
    }
}

impl UiView for TextButton {
    fn build(&mut self, ctx: &mut ViewContext) {
        let (fill, border) = self.current_fill_border();
        let [x, y, w, h] = self.style.rect;

        // Draw button background
        ctx.push(Drawable::Rect {
            id: Uuid::new_v4(),
            rect: self.style.rect,
            fill,
            border,
            radius_px: self.style.radius_px,
            border_px: self.style.border_px,
            layer: self.style.layer,
        });

        // Calculate text position based on alignment
        let approx_text_width = self.text.len() as f32 * self.text_size * 0.6;
        let approx_text_height = self.text_size;

        let text_x = match self.h_align {
            HAlign::Left => x + 8.0,
            HAlign::Center => x + (w - approx_text_width) * 0.5,
            HAlign::Right => x + w - approx_text_width - 8.0,
        };

        let text_y = match self.v_align {
            VAlign::Top => y + 4.0,
            VAlign::Center => y + (h - approx_text_height) * 0.5,
            VAlign::Bottom => y + h - approx_text_height - 4.0,
        };

        // Draw text
        ctx.push(Drawable::Text {
            id: Uuid::new_v4(),
            text: self.text.clone(),
            origin: [text_x, text_y],
            px_size: self.text_size,
            color: self.text_color,
            layer: self.style.layer + 1,
        });
    }

    fn handle_event(&mut self, evt: &UiEvent) -> UiEventOutcome {
        let inside = self.hit(evt.pos);
        match evt.kind {
            UiEventKind::PointerDown => {
                if inside {
                    self.active_pointer = Some(evt.pointer_id);
                    return UiEventOutcome::dirty();
                }
            }
            UiEventKind::PointerUp => {
                let was_active = self.active_pointer == Some(evt.pointer_id);
                self.active_pointer = None;
                if was_active {
                    match self.kind {
                        ButtonKind::Momentary => {
                            let mut outcome = UiEventOutcome::dirty();
                            if inside {
                                outcome.merge(UiEventOutcome::with_action(
                                    UiAction::ButtonPressed(self.id.clone()),
                                ));
                            }
                            return outcome;
                        }
                        ButtonKind::Toggle => {
                            if inside {
                                self.toggled = !self.toggled;
                                let mut outcome = UiEventOutcome::dirty();
                                outcome.merge(UiEventOutcome::with_action(
                                    UiAction::ButtonToggled(self.id.clone(), self.toggled),
                                ));
                                return outcome;
                            }
                            return UiEventOutcome::dirty();
                        }
                    }
                }
            }
            _ => {}
        }
        UiEventOutcome::none()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn view_id(&self) -> &str {
        &self.id
    }
}
