use crate::prelude::*;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum ScreenEditorMode {
    None,
    Scripts,
    Tiles,
}

pub struct ScreenEditorOptions {
    rect: (usize, usize, usize, usize),
    pub widgets: Vec<AtomWidget>,

    pub drag_context: Option<ScreenDragContext>,
}

impl EditorOptions for ScreenEditorOptions {
    fn new(
        _text: Vec<String>,
        rect: (usize, usize, usize, usize),
        _asset: &Asset,
        context: &ScreenContext,
    ) -> Self {
        let mut widgets: Vec<AtomWidget> = vec![];

        let mut mode_list = AtomWidget::new(
            vec![],
            AtomWidgetType::GroupedList,
            AtomData::new("Mode", Value::Empty()),
        );
        mode_list.centered_text = true;

        mode_list.add_group_list(
            [50, 50, 50, 255],
            [80, 80, 80, 255],
            vec![
                "None".to_string(),
                "Scripts".to_string(),
                "Tiles".to_string(),
            ],
        );
        mode_list.set_rect((rect.0, rect.1 + 10, rect.2, 120));
        mode_list.curr_item_index = 1;
        widgets.push(mode_list);

        let mut scripts_list = AtomWidget::new(
            vec![],
            AtomWidgetType::GroupedList,
            AtomData::new("Scripts", Value::Empty()),
        );
        scripts_list.add_group_list(context.color_green, context.color_light_green, vec![]);
        scripts_list.set_rect((rect.0, rect.1 + 340, rect.2, rect.3 - 340));
        scripts_list.curr_item_index = 1;
        widgets.push(scripts_list);

        Self {
            rect,
            widgets,

            drag_context: None,
        }
    }

    fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
    }

    fn draw(
        &mut self,
        frame: &mut [u8],
        anim_counter: usize,
        asset: &mut Asset,
        context: &mut ScreenContext,
        content: &mut Option<Box<dyn EditorContent>>,
    ) {
        context
            .draw2d
            .draw_rect(frame, &self.rect, context.width, &context.color_black);

        self.update_ui(context, content);

        self.widgets[0].draw(frame, context.width, anim_counter, asset, context);

        if self.get_screen_editor_mode() == ScreenEditorMode::Scripts {
            self.widgets[1].draw(frame, context.width, anim_counter, asset, context);
        }

        if let Some(content) = content {
            if let Some(rect) = content.get_hover_rect() {
                context.draw2d.draw_text_rect(
                    frame,
                    &(0, self.rect.1 + 140, self.rect.2, 20),
                    context.width,
                    &asset.get_editor_font("OpenSans"),
                    15.0,
                    &format!("Tile ({}, {})", rect.0 / rect.2, rect.1 / rect.3),
                    &context.color_white,
                    &[0, 0, 0, 255],
                    crate::draw2d::TextAlignment::Center,
                );

                context.draw2d.draw_text_rect(
                    frame,
                    &(0, self.rect.1 + 165, self.rect.2, 20),
                    context.width,
                    &asset.get_editor_font("OpenSans"),
                    15.0,
                    &format!("Pixel ({}, {})", rect.0, rect.1),
                    &context.color_white,
                    &[0, 0, 0, 255],
                    crate::draw2d::TextAlignment::Center,
                );
            }

            if let Some(tile) = content.get_selected_tile() {
                if let Some(map) = asset.get_map_of_id(tile.tilemap) {
                    context.draw2d.draw_animated_tile(
                        frame,
                        &((self.rect.2 - 100) / 2, self.rect.1 + 210),
                        map,
                        context.width,
                        &(tile.x_off as usize, tile.y_off as usize),
                        anim_counter,
                        100,
                    );
                }

                // if let Some(map) = asset.tileset.maps.get(&tile.tilemap) {
                //     context.draw2d.draw_text_rect(frame, &(0, self.rect.1 + 370, self.rect.2, 20), context.width, &asset.get_editor_font("OpenSans"), 15.0, &format!("\"{}\"", map.get_name()), &context.color_white, &[0,0,0,255], crate::draw2d::TextAlignment::Center);
                // }

                context.draw2d.draw_text_rect(
                    frame,
                    &(0, self.rect.1 + 310, self.rect.2, 20),
                    context.width,
                    &asset.get_editor_font("OpenSans"),
                    15.0,
                    &format!("({}, {})", tile.x_off, tile.y_off),
                    &context.color_white,
                    &[0, 0, 0, 255],
                    crate::draw2d::TextAlignment::Center,
                );
            }
        }
    }

    fn mouse_down(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        content: &mut Option<Box<dyn EditorContent>>,
        _toolbar: &mut Option<&mut ToolBar>,
    ) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_down(pos, asset, context) {
                if atom.atom_data.id == "Mode" {
                    if atom.curr_item_index == 0 {
                        context.code_editor_is_active = false;
                    } else if atom.curr_item_index == 1 {
                        context.code_editor_is_active = true;
                    } else if atom.curr_item_index == 2 {
                        context.code_editor_is_active = false;
                    }
                }

                //let mode = self.get_screen_editor_mode();
                //if mode == ScreenEditorMode::Scripts {
                if atom.atom_data.id == "Scripts" {
                    let index = atom.curr_item_index;
                    if let Some(content) = content {
                        content.set_current_script(
                            self.widgets[1].groups[0].items[index].text.clone(),
                            context,
                        );
                    }
                }
                //}

                return true;
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

    fn mouse_hover(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        _content: &mut Option<Box<dyn EditorContent>>,
    ) -> bool {
        let mut consumed = false;
        for atom in &mut self.widgets {
            if atom.mouse_hover(pos, asset, context) {
                consumed = true;
            }
        }

        consumed
    }

    fn mouse_dragged(
        &mut self,
        _pos: (usize, usize),
        _asset: &mut Asset,
        _context: &mut ScreenContext,
        _content: &mut Option<Box<dyn EditorContent>>,
    ) -> bool {
        false
    }

    fn update_ui(
        &mut self,
        _context: &mut ScreenContext,
        _content: &mut Option<Box<dyn EditorContent>>,
    ) {
    }

    fn opening(
        &mut self,
        _asset: &mut Asset,
        context: &mut ScreenContext,
        content: &mut Option<Box<dyn EditorContent>>,
    ) {
        self.update_ui(context, content);
    }

    fn closing(
        &mut self,
        _asset: &mut Asset,
        _context: &mut ScreenContext,
        _content: &mut Option<Box<dyn EditorContent>>,
    ) {
    }

    /// Returns the current editor mode
    fn get_screen_editor_mode(&self) -> ScreenEditorMode {
        let mode = self.widgets[0].curr_item_index;

        let mode = match mode {
            0 => ScreenEditorMode::None,
            2 => ScreenEditorMode::Tiles,
            _ => ScreenEditorMode::Scripts,
        };

        mode
    }

    fn set_script_names(&mut self, scripts: Vec<&String>, index: usize) {
        let mut items = vec![];
        for s in scripts {
            items.push(GroupItem {
                rect: (0, 0, 0, 0),
                text: s.replace(".rhai", ""),
            })
        }

        self.widgets[1].groups[0].items = items;
        self.widgets[1].curr_item_index = index;
        self.widgets[1].dirty = true;
    }
}
