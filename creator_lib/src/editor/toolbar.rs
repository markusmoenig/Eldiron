use crate::prelude::*;

pub enum ToolBarButtons {
    Iterator,
    Assets,
    Regions,
    Characters,
    Systems,
    Items,
    Game,
}

pub struct ToolBar {
    rect                    : (usize, usize, usize, usize),
    pub widgets             : Vec<AtomWidget>,
}

impl Widget for ToolBar {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, _context: &mut ScreenContext) -> Self where Self: Sized {

        let mut widgets : Vec<AtomWidget> = vec![];

        let mut item_button = AtomWidget::new(asset.tileset.maps_names.clone(), AtomWidgetType::ToolBarSliderButton,
        AtomData::new("Game", Value::Empty()));
        item_button.set_rect((rect.0 + 10, rect.1, 200, rect.3));
        item_button.status_help_text = Some("Iterate over the items in the view. ('Tab').".to_string());
        widgets.push(item_button);

        let mut tiles_button = AtomWidget::new(vec!["Assets".to_string()], AtomWidgetType::ToolBarSwitchButton,
        AtomData::new("Assets", Value::Empty()));
        tiles_button.set_rect((rect.0 + 220, rect.1, 150, rect.3));
        tiles_button.selected = true;
        tiles_button.custom_color = Some([44, 145, 209, 255]);
        tiles_button.status_help_text = Some("Edit the assets of your game ('1'). 'Space' to toggle.".to_string());
        widgets.push(tiles_button);

        let mut areas_button = AtomWidget::new(vec!["Regions".to_string()], AtomWidgetType::ToolBarSwitchButton,
        AtomData::new("Regions", Value::Empty()));
        areas_button.set_rect((rect.0 + 380, rect.1, 160, rect.3));
        areas_button.custom_color = Some([217, 64, 51, 255]);
        areas_button.status_help_text = Some("Edit the regions of your game ('2'). 'Space' to toggle.".to_string());
        widgets.push(areas_button);

        let mut behavior_button = AtomWidget::new(vec!["Characters".to_string()], AtomWidgetType::ToolBarSwitchButton,
        AtomData::new("Characters", Value::Empty()));
        behavior_button.set_rect((rect.0 + 550, rect.1, 185, rect.3));
        behavior_button.status_help_text = Some("Edit the characters of your game ('3'). 'Space' to toggle.".to_string());
        behavior_button.custom_color = Some([47, 219, 37, 255]);

        widgets.push(behavior_button);

        let mut systems_button = AtomWidget::new(vec!["Systems".to_string()], AtomWidgetType::ToolBarSwitchButton,
        AtomData::new("Systems", Value::Empty()));
        systems_button.set_rect((rect.0 + 745, rect.1, 165, rect.3));
        systems_button.custom_color = Some([23, 158, 101, 255]);
        systems_button.status_help_text = Some("Edit the systems in your game ('4'). 'Space' to toggle.".to_string());
        widgets.push(systems_button);

        let mut items_button = AtomWidget::new(vec!["Items & Spells".to_string()], AtomWidgetType::ToolBarSwitchButton,
        AtomData::new("Items", Value::Empty()));
        items_button.set_rect((rect.0 + 725 + 195, rect.1, 220, rect.3));
        items_button.status_help_text = Some("Edit the items and spells in your game ('5'). 'Space' to toggle.".to_string());
        items_button.custom_color = Some([205, 142, 67, 255]);

        widgets.push(items_button);

        let mut game_button = AtomWidget::new(vec!["Game".to_string()], AtomWidgetType::ToolBarSwitchButton,
            AtomData::new("Game", Value::Empty()));
        game_button.set_rect((rect.0 + 725 + 175 + 170 + 80, rect.1, 140, rect.3));
        game_button.custom_color = Some([215, 30, 146, 255]);
        game_button.status_help_text = Some("Edit the game logic and settings ('6'). '7' for game settings.".to_string());
        widgets.push(game_button);

        Self {
            rect,
            widgets             : widgets,
        }
    }

    fn resize(&mut self, width: usize, _height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
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
        for atom in &mut self.widgets {
            if atom.mouse_down(pos, asset, context) {
                return true;
            }
        }
        false
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

    fn get_atom_at_index(&mut self, index: usize) -> Option<&mut AtomWidget> {
        if index < self.widgets.len() {
            Some(&mut self.widgets[index])
        } else { None }
    }
}