use crate::prelude::*;

pub struct HLayout {

    pub rect                : (usize, usize, usize, usize),
    pub widgets             : Vec<AtomWidget>,

    pub spacing             : usize,
    pub local_spacing       : Vec<usize>,

    pub margin              : (usize, usize, usize, usize),
}

impl HLayout {

    pub fn new(rect: (usize, usize, usize, usize)) -> Self {
        Self {
            rect,
            widgets         : vec![],
            spacing         : 0,
            local_spacing   : vec![],
            margin          : (10, 10, 10, 10),
        }
    }

    /// Set the layout rect
    pub fn set_rect(&mut self, rect: (usize, usize, usize, usize)) {
        self.rect = rect;
        self.layout();
    }

    /// Add a widget to the layout
    pub fn add(&mut self, widget: AtomWidget,  local_spacing: usize) {
        self.widgets.push(widget);
        self.local_spacing.push(local_spacing);
    }

    /// Layout the widgets
    pub fn layout(&mut self) {
        let mut x = self.rect.0 + self.margin.0;

        for index in 0..self.widgets.len() {
            self.widgets[index].rect.0 = x + self.local_spacing[index];
            self.widgets[index].rect.1 = self.rect.1 + self.margin.1;
            self.widgets[index].rect.3 = self.rect.3 - self.margin.1 - self.margin.3;
            x += self.widgets[index].rect.2 + self.spacing + self.local_spacing[index];
        }
    }

    /// Draw the widgets
    pub fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {
        for widget in self.widgets.iter_mut() {
            widget.draw(frame, context.width, anim_counter, asset, context);
        }
    }

    /// Mouse down event
    pub fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> Option<(usize, String)> {
        for index in 0..self.widgets.len() {
            if self.widgets[index].mouse_down(pos, asset, context) {
                return Some((index, self.widgets[index].atom_data.id.clone()));
            }
        }
        None
    }

    /// Mouse up event
    pub fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> Option<(usize, String)> {
        for index in 0..self.widgets.len() {
            if self.widgets[index].mouse_up(pos, asset, context) {
                return Some((index, self.widgets[index].atom_data.id.clone()));
            }
        }
        None
    }

        /// Mouse dragged event
    pub fn _mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> Option<(usize, String)> {
        for index in 0..self.widgets.len() {
            if self.widgets[index].mouse_dragged(pos, asset, context) {
                return Some((index, self.widgets[index].atom_data.id.clone()));
            }
        }
        None
    }

    /// Mouse hover event
    pub fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> Option<(usize, String)> {
        for index in 0..self.widgets.len() {
            if self.widgets[index].mouse_hover(pos, asset, context) {
                return Some((index, self.widgets[index].atom_data.id.clone()));
            }
        }
        None
    }
}