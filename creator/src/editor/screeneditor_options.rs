use crate::prelude::*;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum ScreenEditorMode {
    None,
    Script,
    Tiles,
}

pub struct ScreenEditorOptions {
    rect                    : (usize, usize, usize, usize),
    pub widgets             : Vec<AtomWidget>,

    pub drag_context        : Option<ScreenDragContext>,
}

impl EditorOptions for ScreenEditorOptions {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self {

        let mut widgets : Vec<AtomWidget> = vec![];

        let mut mode_list = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new_as_int("Mode".to_string(), 0));
        mode_list.drag_enabled = true;
        mode_list.centered_text = true;

        mode_list.add_group_list([50, 50, 50, 255], [80, 80, 80, 255], vec!["None".to_string(), "Script".to_string(),  "Tiles".to_string()]);
        mode_list.set_rect((rect.0, rect.1 + 10, rect.2, 200), asset, context);
        mode_list.curr_item_index = 1;
        widgets.push(mode_list);

        /*
        // Widget Widgets
        let mut widget_widgets : Vec<AtomWidget> = vec![];

        let mut widgets_button = AtomWidget::new(vec![], AtomWidgetType::SliderButton,
        AtomData::new_as_int("Widgets".to_string(), 0));
        widgets_button.atom_data.text = "Widgets".to_string();
        widgets_button.set_rect((rect.0 + 10, rect.1 + 10, rect.2 - 20, 40), asset, context);
        widgets_button.state = WidgetState::Disabled;
        widget_widgets.push(widgets_button);

        let mut rename_widget_button = AtomWidget::new(vec!["Rename".to_string()], AtomWidgetType::Button,
            AtomData::new_as_int("Rename".to_string(), 0));
        rename_widget_button.state = WidgetState::Disabled;
        rename_widget_button.set_rect((rect.0 + 10, rect.1 + 60, rect.2 - 20, 40), asset, context);

        let mut del_widget_button = AtomWidget::new(vec!["Delete".to_string()], AtomWidgetType::Button,
            AtomData::new_as_int("Delete".to_string(), 0));
        del_widget_button.state = WidgetState::Disabled;
        del_widget_button.set_rect((rect.0 + 10, rect.1 + 95, rect.2 - 20, 40), asset, context);

        let mut widget_type_button = AtomWidget::new(vec!["Game".to_string(), "Region".to_string(), "Status".to_string(), "Custom".to_string()], AtomWidgetType::SliderButton,
        AtomData::new_as_int("Widget Type".to_string(), 0));
        widget_type_button.atom_data.text = "Widget Type".to_string();
        widget_type_button.set_rect((rect.0 + 10, rect.1 + 160, rect.2 - 20, 40), asset, context);
        widget_type_button.state = WidgetState::Disabled;

        let mut widget_editing_mode = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new_as_int("EditingMode".to_string(), 0));
        widget_editing_mode.drag_enabled = true;

        widget_editing_mode.add_group_list([50, 50, 50, 255], [80, 80, 80, 255], vec!["Add Widget".to_string(), "Move".to_string(), "Resize".to_string(), "Draw UI Tiles".to_string()]);
        widget_editing_mode.set_rect((rect.0 + 10, rect.1 + 220, rect.2 - 20, 200), asset, context);

        widget_widgets.push(del_widget_button);
        widget_widgets.push(rename_widget_button);
        widget_widgets.push(widget_type_button);
        widget_widgets.push(widget_editing_mode);
        */


        /*
        let mut widget_list = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new_as_int("WidgetList".to_string(), 0));
        widget_list.add_group_list(context.color_blue, context.color_light_blue, vec!["Game Widget".to_string(), "Status Widget".to_string(), "Custom Widget".to_string()]);

        widget_list.set_rect((rect.0 + 10, rect.1 + 210, rect.2 - 20, 200), asset, context);
        widget_widgets.push(widget_list);*/

        // Tile Widgets

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

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) {
        context.draw2d.draw_rect(frame, &self.rect, context.width, &context.color_black);

        self.update_ui(context, content);

        for atom in &mut self.widgets {
           atom.draw(frame, context.width, anim_counter, asset, context);
        }

        if let Some(content) = content {

            if let Some(rect) = content.get_hover_rect() {
                context.draw2d.draw_text_rect(frame, &(0, self.rect.1 + 140, self.rect.2, 20), context.width, &asset.get_editor_font("OpenSans"), 15.0, &format!("Tile ({}, {})", rect.0 / rect.2, rect.1 / rect.3), &context.color_white, &[0,0,0,255], crate::draw2d::TextAlignment::Center);

                context.draw2d.draw_text_rect(frame, &(0, self.rect.1 + 165, self.rect.2, 20), context.width, &asset.get_editor_font("OpenSans"), 15.0, &format!("Pixel ({}, {})", rect.0, rect.1), &context.color_white, &[0,0,0,255], crate::draw2d::TextAlignment::Center);
            }

            if let Some(tile) = content.get_selected_tile() {
                if let Some(map) = asset.get_map_of_id(tile.tilemap) {
                    context.draw2d.draw_animated_tile(frame, &((self.rect.2 - 100) / 2, self.rect.1 + 250), map, context.width, &(tile.grid_x as usize, tile.grid_y as usize), anim_counter, 100);
                }

                // if let Some(map) = asset.tileset.maps.get(&tile.tilemap) {
                //     context.draw2d.draw_text_rect(frame, &(0, self.rect.1 + 370, self.rect.2, 20), context.width, &asset.get_editor_font("OpenSans"), 15.0, &format!("\"{}\"", map.get_name()), &context.color_white, &[0,0,0,255], crate::draw2d::TextAlignment::Center);
                // }

                context.draw2d.draw_text_rect(frame, &(0, self.rect.1 + 390, self.rect.2, 20), context.width, &asset.get_editor_font("OpenSans"), 15.0, &format!("({}, {})", tile.grid_x, tile.grid_y), &context.color_white, &[0,0,0,255], crate::draw2d::TextAlignment::Center);
            }
        }
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_down(pos, asset, context) {

                if atom.atom_data.id == "Mode" {
                    if atom.curr_item_index == 0 {
                        context.code_editor_is_active = false;
                    } else
                    if atom.curr_item_index == 1 {
                        context.code_editor_is_active = true;
                    } else
                    if atom.curr_item_index == 2 {
                        context.code_editor_is_active = false;
                    }
                }
                return true;
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

        /*
        let mode = self.get_screen_editor_mode();

        if mode.0 == ScreenEditorMode::Widgets {
            for atom in &mut self.widget_widgets {
                if atom.mouse_up(pos, asset, context) {
                    if let Some(el_content) = content {

                        if atom.atom_data.id == "Widgets" {
                            if let Some(game_screen) = el_content.get_game_screen() {
                                game_screen.curr_widget_index = atom.curr_index;
                                consumed = true;
                                atom.dirty = true;
                            }
                        } else
                        if atom.atom_data.id == "Widget Type" {
                            if let Some(game_screen) = el_content.get_game_screen() {
                                game_screen.widgets[game_screen.curr_widget_index].widget_type = match atom.curr_index {
                                    1 => GameScreenWidgetType::Region,
                                    2 => GameScreenWidgetType::Status,
                                    3 => GameScreenWidgetType::Custom,
                                    _ => GameScreenWidgetType::Game,
                                };
                            }
                            consumed = true;
                            atom.dirty = true;
                        }
                    }
                }
            }
        }*/

        consumed
    }

    fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) -> bool {
        let mut consumed = false;
        for atom in &mut self.widgets {
            if atom.mouse_hover(pos, asset, context) {
                consumed = true;
            }
        }

        consumed
    }

    fn mouse_dragged(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) -> bool {
        false
    }

    fn update_ui(&mut self, _context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) {

     }

    fn opening(&mut self, _asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) {
        self.update_ui(context, content);
    }

    fn closing(&mut self, _asset: &mut Asset, _context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) { }

    /// Returns the current editor mode
    fn get_screen_editor_mode(&self) -> ScreenEditorMode {

        let mode = self.widgets[0].curr_item_index;

        let mode = match mode {
            0 => ScreenEditorMode::None,
            2 => ScreenEditorMode::Tiles,
            _ => ScreenEditorMode::Script
        };

        mode
    }

}