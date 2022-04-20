use crate::atom::AtomData;
use server::asset::Asset;

use crate::widget::atom::AtomWidget;
use crate::widget::atom::AtomWidgetType;
use crate::widget::context::ScreenContext;

pub struct RegionOptions {
    rect                    : (usize, usize, usize, usize),
    pub widgets             : Vec<AtomWidget>,
}

impl RegionOptions {

    pub fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self {

        let mut widgets : Vec<AtomWidget> = vec![];

        let mut tilemap_names = asset.tileset.maps_names.clone();
        tilemap_names.insert(0, "Tilemaps: All".to_string());

        let mut tilemaps_slider_button = AtomWidget::new(tilemap_names, AtomWidgetType::ToolBarSliderButton,
        AtomData::new_as_int("Tilemaps".to_string(), 0));
        tilemaps_slider_button.set_rect((rect.0 + 10, rect.1 + 10, rect.2 - 20, 40), asset, context);
        widgets.push(tilemaps_slider_button);

        let mut remap_button = AtomWidget::new(vec!["Remap".to_string()], AtomWidgetType::LargeButton,
        AtomData::new_as_int("remap".to_string(), 0));
        remap_button.set_rect((rect.0 + 40, rect.1 + rect.3 - 200, rect.2 - 80, 40), asset, context);
        widgets.push(remap_button);

    //     let mut group_list = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    // AtomData::new_as_int("GroupedList".to_string(), 0));

    //     group_list.add_group_list(context.color_green, context.color_light_green, vec!["Environment".to_string(), "Blocking".to_string(), "Water".to_string() ]);
    //     group_list.set_rect((rect.0, rect.1 + 60, rect.2, 400), asset, context);
    //     widgets.push(group_list);

        Self {
            rect,
            widgets,
        }
    }

    pub fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
    }

    pub fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {
        context.draw2d.draw_rect(frame, &self.rect, context.width, &context.color_black);

        for atom in &mut self.widgets {
           atom.draw(frame, context.width, anim_counter, asset, context);
        }

        if let Some(tile) = &context.curr_region_tile {
            context.draw2d.draw_animated_tile(frame, &((self.rect.2 - 100) / 2, self.rect.1 + self.rect.3 - 140), asset.get_map_of_id(tile.0), context.width, &(tile.1, tile.2), anim_counter, 100);

            context.draw2d.draw_text_rect(frame, &(0, self.rect.1 + self.rect.3 - 40, self.rect.2, 30), context.width, &asset.open_sans, 20.0, &format!("({}, {}, {})", tile.0, tile.1, tile.2), &context.color_white, &[0,0,0,255], crate::draw2d::TextAlignment::Center);
        }
    }

    pub fn _mouse_down(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }

    pub fn _mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }

    pub fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_hover(pos, asset, context) {
                return true;
            }
        }
        false
    }
}