use crate::prelude::*;

pub struct ItemsOptions {
    rect: (usize, usize, usize, usize),
    pub widgets: Vec<AtomWidget>,

    pub drag_context: Option<ScreenDragContext>,
}

impl EditorOptions for ItemsOptions {
    fn new(
        _text: Vec<String>,
        rect: (usize, usize, usize, usize),
        _asset: &Asset,
        context: &ScreenContext,
    ) -> Self {
        let mut widgets: Vec<AtomWidget> = vec![];

        let mut node_list = AtomWidget::new(
            vec![],
            AtomWidgetType::GroupedList,
            AtomData::new("NodeList", Value::Empty()),
        );
        node_list.drag_enabled = true;

        node_list.add_group_list(
            context.color_green,
            context.color_light_green,
            vec![
                "Behavior Tree".to_string(),
                "Expression".to_string(),
                "Script".to_string(),
            ],
        );

        node_list.add_group_list(
            context.color_orange,
            context.color_light_orange,
            vec!["Skill Tree".to_string(), "Skill Level".to_string()],
        );

        node_list.add_group_list(
            context.color_blue,
            context.color_light_blue,
            vec![
                "Audio".to_string(),
                "Effect".to_string(),
                "Light".to_string(),
                "Magic Target".to_string(),
                "Message".to_string(),
                "Set Tile".to_string(),
            ],
        );

        node_list.set_rect(rect);
        widgets.push(node_list);

        Self {
            rect,
            widgets,
            drag_context: None,
        }
    }

    fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
        self.widgets[0].set_rect(self.rect);
    }

    fn draw(
        &mut self,
        frame: &mut [u8],
        anim_counter: usize,
        asset: &mut Asset,
        context: &mut ScreenContext,
        _content: &mut Option<Box<dyn EditorContent>>,
    ) {
        context
            .draw2d
            .draw_rect(frame, &self.rect, context.width, &context.color_black);

        for atom in &mut self.widgets {
            atom.draw(frame, context.width, anim_counter, asset, context);
        }
    }

    fn mouse_down(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        _content: &mut Option<Box<dyn EditorContent>>,
        _toolbar: &mut Option<&mut ToolBar>,
    ) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_down(pos, asset, context) {
                if atom.clicked {
                    if atom.atom_data.id == "NodeList" {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn mouse_up(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        _content: &mut Option<Box<dyn EditorContent>>,
    ) -> bool {
        let mut consumed = false;
        for atom in &mut self.widgets {
            if atom.mouse_up(pos, asset, context) {
                consumed = true;
            }
        }
        consumed
    }

    fn mouse_dragged(
        &mut self,
        _pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        _content: &mut Option<Box<dyn EditorContent>>,
    ) -> bool {
        if let Some(drag_context) = &self.widgets[0].drag_context {
            if context.drag_context == None {
                let mut buffer = [0; 180 * 32 * 4];

                context.draw2d.draw_rect(
                    &mut buffer[..],
                    &(0, 0, 180, 32),
                    180,
                    &drag_context.color.clone(),
                );
                context.draw2d.draw_text_rect(
                    &mut buffer[..],
                    &(0, 0, 180, 32),
                    180,
                    &asset.get_editor_font("OpenSans"),
                    context.toolbar_button_text_size,
                    drag_context.text.as_str(),
                    &context.color_white,
                    &drag_context.color.clone(),
                    draw2d::TextAlignment::Center,
                );

                context.drag_context = Some(ScreenDragContext {
                    text: drag_context.text.clone(),
                    color: drag_context.color.clone(),
                    offset: drag_context.offset.clone(),
                    buffer: Some(buffer),
                });
                context.target_fps = 60;
            }
            self.widgets[0].drag_context = None;
        }
        false
    }
}
