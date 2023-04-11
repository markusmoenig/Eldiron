use crate::prelude::*;

pub struct ItemsOverviewOptions {
    rect                    : (usize, usize, usize, usize),
    pub widgets             : Vec<AtomWidget>,

    pub drag_context        : Option<ScreenDragContext>,
}

impl EditorOptions for ItemsOverviewOptions {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self {

        let mut widgets : Vec<AtomWidget> = vec![];

        let mut node_list = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new("NodeList", Value::Empty()));
        node_list.drag_enabled = true;

        node_list.add_group_list(context.color_blue, context.color_light_blue, vec!["Items".to_string(), "Spells".to_string()]);

        node_list.set_rect(rect, asset, context);
        widgets.push(node_list);

        Self {
            rect,
            widgets,
            drag_context            : None
        }
    }

    fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) {
        context.draw2d.draw_rect(frame, &self.rect, context.width, &context.color_black);

        for atom in &mut self.widgets {
           atom.draw(frame, context.width, anim_counter, asset, context);
        }
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>, toolbar: &mut Option<&mut ToolBar>) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_down(pos, asset, context) {
                if atom.clicked {
                    if atom.atom_data.id == "NodeList" {
                        if let Some(el_content) = content {
                            if atom.curr_item_index == 0 {
                                el_content.set_sub_node_type(NodeSubType::Item, context);
                            } else
                            if atom.curr_item_index == 1 {
                                el_content.set_sub_node_type(NodeSubType::Spell, context);
                            }
                            if let Some(toolbar) = toolbar {

                                let mut items = vec![];
                                let indices = el_content.get_active_indices();

                                if let Some(nodes) = el_content.get_nodes() {
                                    for i in &indices {
                                        items.push(nodes[*i].name.clone());
                                    }
                                }

                                toolbar.widgets[0].text = items;
                                toolbar.widgets[0].dirty = true;
                                if indices.len() > 0 {
                                    toolbar.widgets[0].curr_index = 0;
                                }
                            }
                        }
                        return true;
                    }
                }
            }
        }
        false
    }

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) -> bool {
        let mut consumed = false;
        for atom in &mut self.widgets {
            if atom.mouse_up(pos, asset, context) {
                consumed = true;
            }
        }
        consumed
    }

    fn mouse_dragged(&mut self, _pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) -> bool {
        if let Some(drag_context) = &self.widgets[0].drag_context {
            if context.drag_context == None {

                let mut buffer = [0; 180 * 32 * 4];

                context.draw2d.draw_rect(&mut buffer[..], &(0, 0, 180, 32), 180, &drag_context.color.clone());
                context.draw2d.draw_text_rect(&mut buffer[..], &(0, 0, 180, 32), 180, &asset.get_editor_font("OpenSans"), context.toolbar_button_text_size, drag_context.text.as_str(), &context.color_white, &drag_context.color.clone(), draw2d::TextAlignment::Center);

                context.drag_context = Some(ScreenDragContext {
                    text    : drag_context.text.clone(),
                    color   : drag_context.color.clone(),
                    offset  : drag_context.offset.clone(),
                    buffer  : Some(buffer)
                });
                context.target_fps = 60;
            }
            self.widgets[0].drag_context = None;
        }
        false
    }
}