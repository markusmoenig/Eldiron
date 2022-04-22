use crate::atom::AtomData;
use server::asset::Asset;

use crate::widget::atom::AtomWidget;
use crate::widget::atom::AtomWidgetType;
use crate::widget::context::ScreenContext;
use crate::widget::WidgetState;

use crate::editor::RegionWidget;
use crate::tileset::TileUsage;

use server::gamedata::region::RegionArea;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum RegionEditorMode {
    Tiles,
    Areas,
    Behavior
}

pub struct RegionOptions {
    rect                    : (usize, usize, usize, usize),
    pub widgets             : Vec<AtomWidget>,

    pub tile_widgets        : Vec<AtomWidget>,
    pub area_widgets        : Vec<AtomWidget>,
}

impl RegionOptions {

    pub fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self {

        let mut widgets : Vec<AtomWidget> = vec![];

        let mut mode_button = AtomWidget::new(vec!["Tiles".to_string(), "Areas".to_string(), "Nodes".to_string()], AtomWidgetType::ToolBarSliderButton,
        AtomData::new_as_int("Mode".to_string(), 0));
        mode_button.set_rect((rect.0 + 10, rect.1 + 10, rect.2 - 20, 40), asset, context);
        mode_button.custom_color = Some([217, 64, 51, 255]);
        widgets.push(mode_button);


        let mut tile_widgets : Vec<AtomWidget> = vec![];

        let mut tilemap_names = asset.tileset.maps_names.clone();
        tilemap_names.insert(0, "All".to_string());

        let mut tilemaps_button = AtomWidget::new(tilemap_names, AtomWidgetType::MenuButton,
        AtomData::new_as_int("tilemaps".to_string(), 0));
        tilemaps_button.atom_data.text = "Tilemaps".to_string();
        tilemaps_button.set_rect((rect.0 + 10, rect.1 + 80, rect.2 - 20, 25), asset, context);
        tile_widgets.push(tilemaps_button);

        let mut remap_button = AtomWidget::new(vec!["Remap".to_string()], AtomWidgetType::LargeButton,
        AtomData::new_as_int("remap".to_string(), 0));
        remap_button.set_rect((rect.0 + 40, rect.1 + rect.3 - 200, rect.2 - 80, 40), asset, context);
        tile_widgets.push(remap_button);


        let mut area_widgets : Vec<AtomWidget> = vec![];

        let mut regions_button = AtomWidget::new(vec![], AtomWidgetType::MenuButton,
        AtomData::new_as_int("region".to_string(), 0));
        regions_button.atom_data.text = "Region".to_string();
        regions_button.set_rect((rect.0 + 10, rect.1 + 80, rect.2 - 20, 25), asset, context);
        regions_button.state = WidgetState::Disabled;
        area_widgets.push(regions_button);

        let mut add_area_button = AtomWidget::new(vec!["Add Area".to_string()], AtomWidgetType::Button,
            AtomData::new_as_int("Add Area".to_string(), 0));
        //add_area_button.state = WidgetState::Disabled;
        add_area_button.set_rect((rect.0 + 10, rect.1 + 120, rect.2 - 20, 40), asset, context);

        let mut del_area_button = AtomWidget::new(vec!["Delete".to_string()], AtomWidgetType::Button,
            AtomData::new_as_int("Delete".to_string(), 0));
        del_area_button.state = WidgetState::Disabled;
        del_area_button.set_rect((rect.0 + 10, rect.1 + 155, rect.2 - 20, 40), asset, context);

        let mut rename_area_button = AtomWidget::new(vec!["Rename".to_string()], AtomWidgetType::Button,
            AtomData::new_as_int("Rename".to_string(), 0));
        rename_area_button.state = WidgetState::Disabled;
        rename_area_button.set_rect((rect.0 + 10, rect.1 + 155 + 35, rect.2 - 20, 40), asset, context);

        area_widgets.push(add_area_button);
        area_widgets.push(del_area_button);
        area_widgets.push(rename_area_button);

        Self {
            rect,
            widgets,

            tile_widgets,
            area_widgets,
        }
    }

    pub fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
    }

    pub fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, _region_widget: &mut RegionWidget) {
        context.draw2d.draw_rect(frame, &self.rect, context.width, &context.color_black);

        let mode = self.widgets[0].curr_index;

        for atom in &mut self.widgets {
           atom.draw(frame, context.width, anim_counter, asset, context);
        }

        if mode == 0 {
            for atom in &mut self.tile_widgets {
                atom.draw(frame, context.width, anim_counter, asset, context);
            }
        } else
        if mode == 1 {
            for atom in &mut self.area_widgets {
                atom.draw(frame, context.width, anim_counter, asset, context);
            }
        }

        if let Some(tile) = &context.curr_region_tile {
            context.draw2d.draw_animated_tile(frame, &((self.rect.2 - 100) / 2, self.rect.1 + self.rect.3 - 140), asset.get_map_of_id(tile.0), context.width, &(tile.1, tile.2), anim_counter, 100);

            context.draw2d.draw_text_rect(frame, &(0, self.rect.1 + self.rect.3 - 40, self.rect.2, 30), context.width, &asset.open_sans, 20.0, &format!("({}, {}, {})", tile.0, tile.1, tile.2), &context.color_white, &[0,0,0,255], crate::draw2d::TextAlignment::Center);
        }
    }

    pub fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _region_widget: &mut RegionWidget) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_down(pos, asset, context) {
                if atom.clicked {
                }
                return true;
            }
        }

        let mode = self.get_editor_mode();

        if mode == RegionEditorMode::Tiles {
            for atom in &mut self.tile_widgets {
                if atom.mouse_down(pos, asset, context) {
                    return true;
                }
            }
        } else
        if mode == RegionEditorMode::Areas {
            for atom in &mut self.area_widgets {
                if atom.mouse_down(pos, asset, context) {
                    return true;
                }
            }
        }

        false
    }

    pub fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, region_widget: &mut RegionWidget) -> bool {

        let mut mode_was_updated = false;
        for atom in &mut self.widgets {
            if atom.mouse_up(pos, asset, context) {
                mode_was_updated = true;
            }
        }

        let mode = self.get_editor_mode();

        // Tiles Mode
        if mode == RegionEditorMode::Tiles {
            for atom in &mut self.tile_widgets {
                if atom.mouse_up(pos, asset, context) {
                    if atom.new_selection.is_some() {
                        if atom.atom_data.id == "Tilemaps" {
                            if atom.curr_index == 0 {
                                region_widget.tile_selector.set_tile_type(vec![TileUsage::Environment, TileUsage::EnvBlocking, TileUsage::Water], None, &asset);
                            } else {
                                region_widget.tile_selector.set_tile_type(vec![TileUsage::Environment, TileUsage::EnvBlocking, TileUsage::Water], Some(atom.curr_index - 1), &asset);
                            }
                            atom.dirty = true;
                        } else
                        if atom.atom_data.id == "remap" {
                            if let Some(region) = context.data.regions.get_mut(&region_widget.region_id) {
                                region.remap(asset);
                            }
                        }
                    }
                    return true;
                }
            }
        } else
        if mode == RegionEditorMode::Areas {
            if mode_was_updated {
                self.update_area_ui(context, region_widget);
            }
            for atom in &mut self.area_widgets {
                if atom.mouse_up(pos, asset, context) {

                    if atom.atom_data.id == "Add Area" {
                        if let Some(region) = context.data.regions.get_mut(&region_widget.region_id) {
                            let area = RegionArea { name: "New Area".to_string(), area: vec![], graph: "".to_string() };
                            region.data.areas.push(area);
                        }
                        self.update_area_ui(context, region_widget);
                    } else
                    if atom.atom_data.id == "Delete" {
                        if let Some(region) = context.data.regions.get_mut(&region_widget.region_id) {
                            region.data.areas.remove(self.area_widgets[0].curr_index);
                        }
                        self.update_area_ui(context, region_widget);
                    } else
                    if atom.atom_data.id == "Rename" {
                        use crate::editor::dialog::{DialogState, DialogEntry};
                        context.dialog_state = DialogState::Opening;
                        context.dialog_height = 0;
                        context.target_fps = 60;
                        context.dialog_entry = DialogEntry::NewName;
                        context.dialog_new_name = self.area_widgets[0].text[0].clone();
                        //if let Some(region) = context.data.regions.get_mut(&region_widget.region_id) {
                        //}
                        self.update_area_ui(context, region_widget);
                    }

                    return true;
                }
            }
        }
        false
    }

    pub fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _region_widget: &mut RegionWidget) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_hover(pos, asset, context) {
                return true;
            }
        }
        false
    }

    /// Returns the current editor mode
    pub fn get_editor_mode(&self) -> RegionEditorMode {
        let mode = self.widgets[0].curr_index;

        match mode {
            1 => RegionEditorMode::Areas,
            2 => RegionEditorMode::Behavior,
            _ => RegionEditorMode::Tiles
        }
    }

    /// Update the area ui
    pub fn update_area_ui(&mut self, context: &mut ScreenContext, region_widget: &mut RegionWidget) {

        if let Some(region) = context.data.regions.get(&region_widget.region_id) {

            let area_count = region.data.areas.len();

            if area_count == 0 {
                self.area_widgets[0].text = vec![];
                self.area_widgets[0].curr_index = 0;
                self.area_widgets[0].state = WidgetState::Disabled;
                self.area_widgets[2].state = WidgetState::Disabled;
                self.area_widgets[3].state = WidgetState::Disabled;
            } else {
                self.area_widgets[0].text = region.get_area_names();
                self.area_widgets[0].state = WidgetState::Normal;
                self.area_widgets[2].state = WidgetState::Normal;
                self.area_widgets[3].state = WidgetState::Normal;
            }

            for a in &mut self.area_widgets {
                a.dirty = true;
            }

            region.save_data();
        }
    }

    /// Returns the current area index
    pub fn get_area_index(&self) -> usize {
        self.area_widgets[0].curr_index
    }

    /// Sets a new name for the current area
    pub fn set_area_name(&mut self, name: String, context: &mut ScreenContext, region_widget: &mut RegionWidget) {

        if let Some(region) = context.data.regions.get_mut(&region_widget.region_id) {
            region.data.areas[self.get_area_index()].name = name;
            self.update_area_ui(context, region_widget);
        }
    }
}