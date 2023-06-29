use crate::prelude::*;

#[derive(PartialEq, Eq, Hash)]
enum ControlWidgets {
    _Undo,
    _Redo,
    Demo,
    Game1,
    Game2,
    Game3,
    Game4,
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

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), _asset: &Asset, _context: &mut ScreenContext) -> Self where Self: Sized {

        let mut widgets : Vec<AtomWidget> = vec![];

        let mut undo_button = AtomWidget::new(vec!["Undo".to_string()], AtomWidgetType::ToolBarButton,
            AtomData::new("Undo", Value::Empty()));
        undo_button.no_border = true;
        undo_button.state = WidgetState::Disabled;
        undo_button.set_rect((rect.0 + 10, rect.1, 80, rect.3));
        widgets.push(undo_button);

        let mut redo_button = AtomWidget::new(vec!["Redo".to_string()], AtomWidgetType::ToolBarButton,
            AtomData::new("Redo", Value::Empty()));
        redo_button.no_border = true;
        redo_button.state = WidgetState::Disabled;
        redo_button.set_rect((rect.0 + 100, rect.1, 80, rect.3));
        widgets.push(redo_button);

        let mut demo_button = AtomWidget::new(vec!["Demo".to_string()], AtomWidgetType::ToolBarCheckButton,
            AtomData::new("Demo", Value::Empty()));
        demo_button.no_border = true;
        demo_button.checked = true;
        demo_button.status_help_text = Some("Switch to the Demo project.".to_string());
        demo_button.set_rect((rect.0 + 200, rect.1, 80, rect.3));
        widgets.push(demo_button);

        let mut game1_button = AtomWidget::new(vec!["G1".to_string()], AtomWidgetType::ToolBarCheckButton,
            AtomData::new("Game1", Value::Empty()));
        game1_button.status_help_text = Some("Switch to Game 1.".to_string());
        game1_button.no_border = true;
        game1_button.set_rect((rect.0 + 290, rect.1, 60, rect.3));
        widgets.push(game1_button);

        let mut game2_button = AtomWidget::new(vec!["G2".to_string()], AtomWidgetType::ToolBarCheckButton,
            AtomData::new("Game2", Value::Empty()));
        game2_button.no_border = true;
        game2_button.status_help_text = Some("Switch to Game 2.".to_string());
        game2_button.set_rect((rect.0 + 360, rect.1, 60, rect.3));
        widgets.push(game2_button);

        let mut game3_button = AtomWidget::new(vec!["G3".to_string()], AtomWidgetType::ToolBarCheckButton,
            AtomData::new("Game3", Value::Empty()));
        game3_button.no_border = true;
        game3_button.status_help_text = Some("Switch to Game 3.".to_string());
        game3_button.set_rect((rect.0 + 430, rect.1, 60, rect.3));
        widgets.push(game3_button);

        let mut game4_button = AtomWidget::new(vec!["G4".to_string()], AtomWidgetType::ToolBarCheckButton,
            AtomData::new("Game4", Value::Empty()));
        game4_button.no_border = true;
        game4_button.status_help_text = Some("Switch to Game 4.".to_string());
        game4_button.set_rect((rect.0 + 500, rect.1, 60, rect.3));
        widgets.push(game4_button);

        let mut help_button = AtomWidget::new(vec!["Help".to_string()], AtomWidgetType::ToolBarButton,
            AtomData::new("Help", Value::Empty()));
        help_button.no_border = true;
        help_button.set_rect((rect.2 - 100 - 200, rect.1, 80, rect.3));
        widgets.push(help_button);

        let mut play_button = AtomWidget::new(vec!["Play".to_string()], AtomWidgetType::ToolBarButton,
            AtomData::new("Play", Value::Empty()));
        play_button.no_border = true;
        play_button.set_rect((rect.2 - 100 - 100, rect.1, 80, rect.3));
        widgets.push(play_button);

        let mut debug_button = AtomWidget::new(vec!["Debug".to_string()], AtomWidgetType::ToolBarButton,
            AtomData::new("Debug", Value::Empty()));
        debug_button.no_border = true;
        debug_button.set_rect((rect.2 - 110, rect.1, 100, rect.3));
        widgets.push(debug_button);

        Self {
            rect,
            widgets             : widgets,
            show_help           : false,
        }
    }

    fn resize(&mut self, width: usize, _height: usize, _context: &ScreenContext) {
        self.rect.2 = width;

        let rect = self.rect;
        self.widgets[7].set_rect((rect.2 - 100 - 200, rect.1, 80, rect.3));
        self.widgets[8].set_rect((rect.2 - 100 - 100, rect.1, 80, rect.3));
        self.widgets[9].set_rect((rect.2 - 110, rect.1, 80, rect.3));
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {
        context.draw2d.draw_rect(frame, &self.rect, context.width, &context.color_toolbar);

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

                if atom_widget.atom_data.id == "Demo" {
                    for i in ControlWidgets::Demo as usize..=ControlWidgets::Game4 as usize {
                        self.widgets[i].checked = false;
                        self.widgets[i].dirty = true;
                    }
                    self.widgets[ControlWidgets::Demo as usize].checked = true;
                    context.project_to_load = Some(context.project_path.join("Demo"));
                } else
                if atom_widget.atom_data.id == "Game1" {
                    for i in ControlWidgets::Demo as usize..=ControlWidgets::Game4 as usize {
                        self.widgets[i].checked = false;
                        self.widgets[i].dirty = true;
                    }
                    self.widgets[ControlWidgets::Game1 as usize].checked = true;
                    context.project_to_load = Some(context.project_path.join("Game1"));
                } else
                if atom_widget.atom_data.id == "Game2" {
                    for i in ControlWidgets::Demo as usize..=ControlWidgets::Game4 as usize {
                        self.widgets[i].checked = false;
                        self.widgets[i].dirty = true;
                    }
                    self.widgets[ControlWidgets::Game2 as usize].checked = true;
                    context.project_to_load = Some(context.project_path.join("Game2"));
                } else
                if atom_widget.atom_data.id == "Game3" {
                    for i in ControlWidgets::Demo as usize..=ControlWidgets::Game4 as usize {
                        self.widgets[i].checked = false;
                        self.widgets[i].dirty = true;
                    }
                    self.widgets[ControlWidgets::Game3 as usize].checked = true;
                    context.project_to_load = Some(context.project_path.join("Game3"));
                } else
                if atom_widget.atom_data.id == "Game4" {
                    for i in ControlWidgets::Demo as usize..=ControlWidgets::Game4 as usize {
                        self.widgets[i].checked = false;
                        self.widgets[i].dirty = true;
                    }
                    self.widgets[ControlWidgets::Game4 as usize].checked = true;
                    context.project_to_load = Some(context.project_path.join("Game4"));
                } else
                if atom_widget.atom_data.id == "Help" {
                    self.show_help = true;
                } else
                if atom_widget.atom_data.id == "Debug" {
                    if context.is_running == false {

                        context.is_running = true;
                        context.is_debugging = true;
                        context.debug_messages = vec![];

                        atom_widget.text[0] = "Stop".to_string();

                        // Start server
                        let mut server = core_server::server::Server::new();
                        server.collect_data(&context.data);
                        _ = server.start(Some(10));
                        context.player_uuid = server.create_player_instance();

                        context.server = Some(server);

                        for index in 0..self.widgets.len() {
                            if index != ControlWidgets::Debug as usize {
                                self.widgets[index].state = WidgetState::Disabled;
                                self.widgets[index].dirty = true;
                            }
                        }
                    } else {
                        self.stop_debugging(context);
                        if let Some(server) = &mut context.server {
                            _ = server.shutdown();
                        }
                    }
                } else
                if atom_widget.atom_data.id == "Play" {
                    if context.is_running == false {

                        context.is_running = true;
                        context.is_debugging = false;
                        atom_widget.text[0] = "Stop".to_string();

                        // Start server
                        let mut server = core_server::server::Server::new();
                        server.collect_data(&context.data);
                        server.create_local_user();

                        _ = server.start(Some(10));
                        // _ = server.start(None);

                        context.player_uuid = server.create_player_instance();

                        context.server = Some(server);

                        for index in 0..self.widgets.len() {
                            if index != ControlWidgets::Play as usize {
                                self.widgets[index].state = WidgetState::Disabled;
                                self.widgets[index].dirty = true;
                            }
                        }
                    } else {

                        context.is_running = false;
                        atom_widget.text[0] = "Play".to_string();
                        context.just_stopped_running = true;

                        if let Some(server) = &mut context.server {
                            _ = server.shutdown();
                        }

                        context.server = None;

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
        // context.data.shutdown();
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