
use crate::atom::AtomData;
use crate::widget::*;
use server::asset::Asset;

use crate::widget::atom:: { AtomWidget, AtomWidgetType };
use crate::widget::context::ScreenContext;


#[derive(PartialEq, Eq, Hash)]
enum ControlWidgets {
    _Undo,
    _Redo,
    _ProjectSwitch,
    _Help,
    Play,
    Debug,
}

pub struct ControlBar {
    rect                    : (usize, usize, usize, usize),
    pub widgets             : Vec<AtomWidget>,
    pub show_help           : bool,
}

impl Widget for ControlBar {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &mut ScreenContext) -> Self where Self: Sized {

        let mut widgets : Vec<AtomWidget> = vec![];

        let mut undo_button = AtomWidget::new(vec!["Undo".to_string()], AtomWidgetType::ToolBarButton,
            AtomData::new_as_int("Undo".to_string(), 0));
        undo_button.no_border = true;
        undo_button.state = WidgetState::Disabled;
        undo_button.set_rect((rect.0 + 10, rect.1, 80, rect.3), asset, context);
        widgets.push(undo_button);

        let mut redo_button = AtomWidget::new(vec!["Redo".to_string()], AtomWidgetType::ToolBarButton,
            AtomData::new_as_int("Redo".to_string(), 0));
        redo_button.no_border = true;
        redo_button.state = WidgetState::Disabled;
        redo_button.set_rect((rect.0 + 100, rect.1, 80, rect.3), asset, context);
        widgets.push(redo_button);


        let mut projects_button = AtomWidget::new(context.get_project_list(), AtomWidgetType::ToolBarSliderButton,
            AtomData::new_as_int("Projects".to_string(), 0));
        projects_button.no_border = true;
        projects_button.state = WidgetState::Disabled;
        projects_button.set_rect((rect.0 + 220, rect.1, 300, rect.3), asset, context);
        widgets.push(projects_button);

        let mut help_button = AtomWidget::new(vec!["Help".to_string()], AtomWidgetType::ToolBarButton,
            AtomData::new_as_int("Help".to_string(), 0));
        help_button.no_border = true;
        help_button.set_rect((rect.2 - 100 - 200, rect.1, 80, rect.3), asset, context);
        widgets.push(help_button);

        let mut play_button = AtomWidget::new(vec!["Play".to_string()], AtomWidgetType::ToolBarButton,
            AtomData::new_as_int("Play".to_string(), 0));
        play_button.no_border = true;
        play_button.set_rect((rect.2 - 100 - 100, rect.1, 80, rect.3), asset, context);
        widgets.push(play_button);

        let mut debug_button = AtomWidget::new(vec!["Debug".to_string()], AtomWidgetType::ToolBarButton,
            AtomData::new_as_int("Debug".to_string(), 0));
        debug_button.no_border = true;
        debug_button.set_rect((rect.2 - 110, rect.1, 100, rect.3), asset, context);
        widgets.push(debug_button);

        Self {
            rect,
            widgets             : widgets,
            show_help           : false,
        }
    }

    fn resize(&mut self, width: usize, _height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {
        context.draw2d.draw_rect(frame, &self.rect, context.width, &context.color_black);

        for atom in &mut self.widgets {
            atom.draw(frame, context.width, anim_counter, asset, context);
        }
    }

    fn draw_overlay(&mut self, frame: &mut [u8], rect: &(usize, usize, usize, usize), anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {
        for atom in &mut self.widgets {
            atom.draw_overlay(frame, rect, anim_counter, asset, context);
        }
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        for atom_widget in &mut self.widgets {
            if atom_widget.mouse_down(pos, asset, context) {

                if atom_widget.atom_data.id == "Help" {
                    self.show_help = true;
                } else
                if atom_widget.atom_data.id == "Debug" {
                    if context.is_running == false {
                        context.data.runs_in_editor = true;
                        // context.data.create_behavior_instances();
                        // context.data.create_player_instance(context.player_id);
                        // context.data.activate_region_instances(context.data.regions_ids[context.curr_region_index]);
                        context.data.startup_client();
                        context.is_running = true;
                        context.is_debugging = true;
                        atom_widget.text[0] = "Stop".to_string();
                        context.data.messages = vec![];

                        for index in 0..self.widgets.len() {
                            if index != ControlWidgets::Debug as usize {
                                self.widgets[index].state = WidgetState::Disabled;
                                self.widgets[index].dirty = true;
                            }
                        }
                    } else {
                        // context.data.clear_instances();
                        context.data.shutdown_client();
                        context.is_running = false;
                        context.is_debugging = false;
                        atom_widget.text[0] = "Debug".to_string();
                        context.just_stopped_running = true;

                        for index in 0..self.widgets.len() {
                            if index != ControlWidgets::Debug as usize {
                                self.widgets[index].state = WidgetState::Normal;
                            }
                            self.widgets[index].dirty = true;
                        }
                    }
                } else
                if atom_widget.atom_data.id == "Play" {
                    if context.is_running == false {
                        context.data.runs_in_editor = false;
                        // context.data.create_behavior_instances();
                        // context.data.create_player_instance(context.player_id);
                        // context.data.activate_region_instances(context.data.regions_ids[context.curr_region_index]);
                        context.data.startup_client();
                        context.is_running = true;
                        context.is_debugging = false;
                        atom_widget.text[0] = "Stop".to_string();
                        context.data.messages = vec![];

                        for index in 0..self.widgets.len() {
                            if index != ControlWidgets::Play as usize {
                                self.widgets[index].state = WidgetState::Disabled;
                                self.widgets[index].dirty = true;
                            }
                        }
                    } else {
                        //context.data.clear_instances();
                        context.data.shutdown_client();
                        context.is_running = false;
                        atom_widget.text[0] = "Play".to_string();
                        context.just_stopped_running = true;

                        for index in 0..self.widgets.len() {
                            if index != ControlWidgets::Play as usize {
                                self.widgets[index].state = WidgetState::Normal;
                            }
                            self.widgets[index].dirty = true;
                        }
                    }
                }
                return true;
            }
        }
        false
    }

    fn stop_debugging(&mut self, context: &mut ScreenContext) {
        context.data.clear_instances();
        context.is_running = false;
        context.is_debugging = false;
        self.widgets[ControlWidgets::Debug as usize].text[0] = "Debug".to_string();
        context.just_stopped_running = true;

        for index in 0..self.widgets.len() {
            if index != ControlWidgets::Debug as usize {
                self.widgets[index].state = WidgetState::Normal;
            }
            self.widgets[index].dirty = true;
        }
    }

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        let mut consumed = false;

        for atom in &mut self.widgets {
            if atom.mouse_up(pos, asset, context) {
                consumed = true;
            }
        }
        consumed
    }

    fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_dragged(pos, asset, context) {
                return true;
            }
        }
        false
    }

    fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_hover(pos, asset, context) {
                return true;
            }
        }
        false
    }

    fn get_rect(&self) -> &(usize, usize, usize, usize) {
        return &self.rect;
    }
}