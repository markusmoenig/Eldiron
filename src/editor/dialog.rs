use server::asset::Asset;

use crate::atom::{ AtomWidget, AtomWidgetType, AtomData };

use crate::context::ScreenContext;

#[derive(PartialEq, Debug)]
pub enum DialogState {
    Closed,
    Open,
    Opening,
    Closing
}

pub struct DialogWidget {
    pub rect                    : (usize, usize, usize, usize),
    pub text                    : String,

    pub widgets                 : Vec<AtomWidget>,

    dirty                       : bool,
    buffer                      : Vec<u8>,

    clicked_id                  : String,
}

impl DialogWidget {

    pub fn new() -> Self {

        let mut widgets : Vec<AtomWidget> = vec![];

        let cancel_button = AtomWidget::new(vec!["Cancel".to_string()], AtomWidgetType::ToolBarButton,
        AtomData::new_as_int("Cancel".to_string(), 0));
        widgets.push(cancel_button);

        let ok_button = AtomWidget::new(vec!["Accept".to_string()], AtomWidgetType::ToolBarButton,
        AtomData::new_as_int("Accept".to_string(), 0));
        widgets.push(ok_button);

        Self {
            rect                : (0, 0, 600, 200),
            text                : "".to_string(),

            widgets             : widgets,

            dirty               : true,
            buffer              : vec![0],

            clicked_id          : "".to_string(),
        }
    }

    pub fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        let mut rect = (0_usize, 0_usize, self.rect.2, self.rect.3);

        if context.dialog_state == DialogState::Opening {
            context.dialog_height += 20;
            rect.3 = context.dialog_height;
            if context.dialog_height >= self.rect.3 {
                context.dialog_state = DialogState::Open;
                context.target_fps = context.default_fps;
            }
            self.dirty = true;
        } else
        if context.dialog_state == DialogState::Closing {
            context.dialog_height -= 20;
            rect.3 = context.dialog_height;
            if context.dialog_height <= 20 {
                context.dialog_state = DialogState::Closed;
                context.target_fps = context.default_fps;
                return;
            }
            self.dirty = true;
        }

        if self.buffer.len() != rect.2 * rect.3 * 4 {
            self.buffer = vec![0;rect.2 * rect.3 * 4];
        }
        let buffer_frame = &mut self.buffer[..];

        if self.dirty {

            buffer_frame.iter_mut().map(|x| *x = 0).count();

            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &(rect.2 as f64 - 1.0, rect.3 as f64 - 1.0), &context.color_black, &(20.0, 0.0, 20.0, 0.0), &context.color_light_gray, 1.5);

            if context.dialog_state == DialogState::Open {
                self.widgets[0].set_rect((rect.2 - 280, rect.3 - 60, 120, 40), asset, context);
                self.widgets[1].set_rect((rect.2 - 140, rect.3 - 60, 120, 40), asset, context);

                for atom in &mut self.widgets {
                    atom.draw(buffer_frame, rect.2, anim_counter, asset, context);
                }
            }
        }
        self.dirty = false;
        context.draw2d.blend_slice(frame, buffer_frame, &(self.rect.0, self.rect.1, rect.2, rect.3), context.width);
    }

    pub fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        self.clicked_id = "".to_string();

        if pos.0 < self.rect.0 || pos.1 < self.rect.1 { return false; }
        let local = (pos.0 - self.rect.0, pos.1 - self.rect.1);
        for atom in &mut self.widgets {
            if atom.mouse_down(local, asset, context) {
                self.dirty = true;
                self.clicked_id = atom.atom_data.id.clone();
                return true;
            }
        }
        false
    }

    pub fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {

        if pos.0 < self.rect.0 || pos.1 < self.rect.1 { return false; }
        let local = (pos.0 - self.rect.0, pos.1 - self.rect.1);
        for atom in &mut self.widgets {
            if atom.mouse_up(local, asset, context) {
                self.dirty = true;

                if self.clicked_id == "Cancel" {
                    context.dialog_state = DialogState::Closing;
                    context.target_fps = 60;
                }

                return true;
            }
        }

        false
    }

    pub fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        if pos.0 < self.rect.0 || pos.1 < self.rect.1 { return false; }
        let local = (pos.0 - self.rect.0, pos.1 - self.rect.1);
        for atom in &mut self.widgets {
            if atom.mouse_dragged(local, asset, context) {
                self.dirty = true;
                return true;
            }
        }
        false
    }

    pub fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        if pos.0 < self.rect.0 || pos.1 < self.rect.1 { return false; }
        let local = (pos.0 - self.rect.0, pos.1 - self.rect.1);
        for atom in &mut self.widgets {
            if atom.mouse_hover(local, asset, context) {
                self.dirty = true;
                return true;
            }
        }
        false
    }
}