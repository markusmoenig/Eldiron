use crate::widget::*;

use crate::atom::AtomData;
use crate::widget::context::ScreenDragContext;
use server::asset::Asset;

use crate::widget::atom::AtomWidget;
use crate::widget::atom::AtomWidgetType;
use crate::widget::context::ScreenContext;

use crate::editor::{ EditorOptions, EditorContent };


#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum ScreenEditorMode {
    Widgets,
    Tiles,
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum ScreenEditorAction {
    Add,
    //Select,
}

pub struct ScreenEditorOptions {
    rect                    : (usize, usize, usize, usize),
    pub widgets             : Vec<AtomWidget>,

    pub tile_widgets        : Vec<AtomWidget>,
    pub widget_widgets      : Vec<AtomWidget>,

    pub drag_context        : Option<ScreenDragContext>,
}

impl EditorOptions for ScreenEditorOptions {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self {

        let mut widgets : Vec<AtomWidget> = vec![];

        let mut mode_list = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new_as_int("Mode".to_string(), 0));
        mode_list.drag_enabled = true;
        mode_list.centered_text = true;

        mode_list.add_group_list([50, 50, 50, 255], [80, 80, 80, 255], vec!["Draw Widgets".to_string(), "Draw UI Tiles".to_string()]);
        mode_list.set_rect((rect.0, rect.1 + 10, rect.2, 200), asset, context);
        widgets.push(mode_list);

        // Widget Widgets
        let mut widget_widgets : Vec<AtomWidget> = vec![];

        let mut widgets_button = AtomWidget::new(vec![], AtomWidgetType::MenuButton,
        AtomData::new_as_int("Area".to_string(), 0));
        widgets_button.atom_data.text = "Area".to_string();
        widgets_button.set_rect((rect.0 + 10, rect.1 + 130, rect.2 - 20, 40), asset, context);
        widgets_button.state = WidgetState::Disabled;
        widget_widgets.push(widgets_button);

        let mut add_widget_button = AtomWidget::new(vec!["Add Widget".to_string()], AtomWidgetType::Button,
            AtomData::new_as_int("Add Widget".to_string(), 0));
        //add_area_button.state = WidgetState::Disabled;
        add_widget_button.set_rect((rect.0 + 10, rect.1 + 180, rect.2 - 20, 40), asset, context);

        let mut del_widget_button = AtomWidget::new(vec!["Delete".to_string()], AtomWidgetType::Button,
            AtomData::new_as_int("Delete".to_string(), 0));
        del_widget_button.state = WidgetState::Disabled;
        del_widget_button.set_rect((rect.0 + 10, rect.1 + 175 + 40, rect.2 - 20, 40), asset, context);

        let mut rename_widget_button = AtomWidget::new(vec!["Rename".to_string()], AtomWidgetType::Button,
            AtomData::new_as_int("Rename".to_string(), 0));
        rename_widget_button.state = WidgetState::Disabled;
        rename_widget_button.set_rect((rect.0 + 10, rect.1 + 175 + 35 + 40, rect.2 - 20, 40), asset, context);

        let mut widget_editing_mode = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new_as_int("EditingMode".to_string(), 0));
        widget_editing_mode.drag_enabled = true;

        widget_editing_mode.add_group_list(context.color_blue, context.color_light_blue, vec!["Add Mode".to_string(), "Remove".to_string()]);
        widget_editing_mode.set_rect((rect.0 + 10, rect.1 + 310, rect.2 - 20, 200), asset, context);

        widget_widgets.push(add_widget_button);
        widget_widgets.push(del_widget_button);
        widget_widgets.push(rename_widget_button);
        widget_widgets.push(widget_editing_mode);

        /*
        let mut widget_list = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new_as_int("WidgetList".to_string(), 0));
        widget_list.add_group_list(context.color_blue, context.color_light_blue, vec!["Game Widget".to_string(), "Status Widget".to_string(), "Custom Widget".to_string()]);

        widget_list.set_rect((rect.0 + 10, rect.1 + 210, rect.2 - 20, 200), asset, context);
        widget_widgets.push(widget_list);*/

        // Tile Widgets
        let tile_widgets : Vec<AtomWidget> = vec![];

        Self {
            rect,
            widgets,

            widget_widgets,
            tile_widgets,

            drag_context            : None
        }
    }

    fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) {
        context.draw2d.draw_rect(frame, &self.rect, context.width, &context.color_black);

        for atom in &mut self.widgets {
           atom.draw(frame, context.width, anim_counter, asset, context);
        }

        let mode = self.get_screen_editor_mode();

        if mode.0 == ScreenEditorMode::Widgets {
            for atom in &mut self.widget_widgets {
                atom.draw(frame, context.width, anim_counter, asset, context);
            }
        } else
        if mode.0 == ScreenEditorMode::Tiles {
            for atom in &mut self.tile_widgets {
                atom.draw(frame, context.width, anim_counter, asset, context);
            }
        }

        if mode.0 == ScreenEditorMode::Widgets {

            if let Some(_content) = content {
                /*
                if let Some(tile) = content.get_selected_tile() {
                    context.draw2d.draw_animated_tile(frame, &((self.rect.2 - 100) / 2, self.rect.1 + self.rect.3 - 120), asset.get_map_of_id(tile.0), context.width, &(tile.1, tile.2), anim_counter, 100);

                    //context.draw2d.draw_text_rect(frame, &(0, self.rect.1 + self.rect.3 - 22, self.rect.2, 20), context.width, &asset.get_editor_font("OpenSans"), 15.0, &format!("{}, {})", /*tile.0,*/ tile.1, tile.2), &context.color_white, &[0,0,0,255], crate::draw2d::TextAlignment::Center);
                }*/
                for atom in &mut self.widget_widgets {
                    atom.draw_overlay(frame, &self.rect, anim_counter, asset, context);
                }
            }
        } else
        if mode.0 == ScreenEditorMode::Tiles {
            for atom in &mut self.tile_widgets {
                atom.draw_overlay(frame, &self.rect, anim_counter, asset, context);
            }
        }

    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_down(pos, asset, context) {
                return true;
            }
        }

        let mode = self.get_screen_editor_mode();

        if mode.0 == ScreenEditorMode::Widgets {
            for atom in &mut self.tile_widgets {
                if atom.mouse_down(pos, asset, context) {
                    if let Some(_content) = content {
                        /*
                        if atom.atom_data.id == "UsageList" {
                            if let Some(tile_selector) = content.get_tile_selector() {
                                tile_selector.set_tile_type(vec![self.get_tile_usage()], self.get_tilemap_index(), self.get_tags(), &asset);
                            }
                        } else
                        if atom.atom_data.id == "Layer" {
                            self.curr_layer = atom.curr_index + 1;
                        } else
                        if atom.atom_data.id == "remap" {
                            if let Some(region) = context.data.regions.get_mut(&content.get_region_id()) {
                                region.remap(asset);
                            }
                        }*/
                    }
                    return true;
                }
            }
        } else
        if mode.0 == ScreenEditorMode::Tiles {
            for atom in &mut self.tile_widgets {
                if atom.mouse_down(pos, asset, context) {
                    return true;
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

    /// Returns the current editor mode
    fn get_screen_editor_mode(&self) -> (ScreenEditorMode, ScreenEditorAction) {
        let mode = self.widgets[0].curr_item_index;

        let mode = match mode {
            1 => ScreenEditorMode::Tiles,
            _ => ScreenEditorMode::Widgets
        };

        (mode, ScreenEditorAction::Add)
    }

}