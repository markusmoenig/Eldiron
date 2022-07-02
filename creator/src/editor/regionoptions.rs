use crate::atom::AtomData;
use core_shared::asset::Asset;
use core_shared::asset::TileUsage;

use crate::widget::atom::AtomWidget;
use crate::widget::atom::AtomWidgetType;
use crate::widget::context::ScreenContext;
use crate::widget::WidgetState;

use crate::widget::*;
use crate::widget::context::ScreenDragContext;

use crate::editor::traits::{ EditorOptions, EditorContent };

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum RegionEditorMode {
    Tiles,
    Areas,
    Behavior,
    Settings
}

pub struct RegionOptions {
    rect                    : (usize, usize, usize, usize),
    pub widgets             : Vec<AtomWidget>,

    curr_layer              : usize,

    pub tile_widgets        : Vec<AtomWidget>,
    pub area_widgets        : Vec<AtomWidget>,
    pub behavior_widgets    : Vec<AtomWidget>,
}

impl EditorOptions for RegionOptions {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self {

        let mut widgets : Vec<AtomWidget> = vec![];

        let mut mode_button = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new_as_int("Mode".to_string(), 0));
        mode_button.drag_enabled = true;

        mode_button.add_group_list([50, 50, 50, 255], [80, 80, 80, 255], vec!["Draw Tiles".to_string(), "Edit Areas".to_string(), "Area Behavior".to_string(), "Settings".to_string()]);
        mode_button.set_rect((rect.0, rect.1 + 10, rect.2, 200), asset, context);
        mode_button.centered_text = true;
        widgets.push(mode_button);

        // Tile Widgets
        let mut tile_widgets : Vec<AtomWidget> = vec![];

        let mut tilemap_names = asset.tileset.maps_names.clone();
        tilemap_names.insert(0, "All".to_string());

        let mut tilemaps_button = AtomWidget::new(tilemap_names, AtomWidgetType::SliderButton,
        AtomData::new_as_int("Tilemaps".to_string(), 0));
        tilemaps_button.atom_data.text = "Tilemaps".to_string();
        tilemaps_button.set_rect((rect.0 + 10, rect.1 + 150, rect.2 - 20, 40), asset, context);
        tile_widgets.push(tilemaps_button);

        let mut tags_button = AtomWidget::new(vec!["".to_string()], AtomWidgetType::TagsButton,
            AtomData::new_as_int("Tags".to_string(), 0));
        tags_button.set_rect((rect.0 + 10, rect.1 + 185, rect.2 - 20, 40), asset, context);
        tile_widgets.push(tags_button);

        let mut usage_list = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new_as_int("UsageList".to_string(), 0));
        usage_list.add_group_list(context.color_blue, context.color_light_blue, vec!["Environment".to_string(), "Road".to_string(), "Blocking".to_string(), "Water".to_string(), "UI Element".to_string()]);

        usage_list.set_rect((rect.0 + 10, rect.1 + 230, rect.2 - 20, 200), asset, context);
        tile_widgets.push(usage_list);

        let mut layer_button = AtomWidget::new(vec!["1".to_string(), "2".to_string(), "3".to_string(), "4".to_string()], AtomWidgetType::NumberRow, AtomData::new_as_int("Layer".to_string(), 0));
        layer_button.set_rect((rect.0 + 10, rect.1 + 410, rect.2 - 20, 30), asset, context);
        tile_widgets.push(layer_button);

        let mut remap_button = AtomWidget::new(vec!["Remap".to_string()], AtomWidgetType::Button,
        AtomData::new_as_int("remap".to_string(), 0));
        remap_button.set_rect((rect.0 + 10, rect.1 + rect.3 - 160, rect.2 - 20, 40), asset, context);
        tile_widgets.push(remap_button);

        // Area Widgets
        let mut area_widgets : Vec<AtomWidget> = vec![];

        let mut regions_button = AtomWidget::new(vec![], AtomWidgetType::SliderButton,
        AtomData::new_as_int("Area".to_string(), 0));
        regions_button.atom_data.text = "Area".to_string();
        regions_button.set_rect((rect.0 + 10, rect.1 + 150, rect.2 - 20, 40), asset, context);
        regions_button.state = WidgetState::Disabled;
        area_widgets.push(regions_button);

        let mut add_area_button = AtomWidget::new(vec!["Add Area".to_string()], AtomWidgetType::Button,
            AtomData::new_as_int("Add Area".to_string(), 0));
        //add_area_button.state = WidgetState::Disabled;
        add_area_button.set_rect((rect.0 + 10, rect.1 + 200, rect.2 - 20, 40), asset, context);

        let mut del_area_button = AtomWidget::new(vec!["Delete".to_string()], AtomWidgetType::Button,
            AtomData::new_as_int("Delete".to_string(), 0));
        del_area_button.state = WidgetState::Disabled;
        del_area_button.set_rect((rect.0 + 10, rect.1 + 240, rect.2 - 20, 40), asset, context);

        let mut rename_area_button = AtomWidget::new(vec!["Rename".to_string()], AtomWidgetType::Button,
            AtomData::new_as_int("Rename".to_string(), 0));
        rename_area_button.state = WidgetState::Disabled;
        rename_area_button.set_rect((rect.0 + 10, rect.1 + 275, rect.2 - 20, 40), asset, context);

        let mut area_editing_mode = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new_as_int("EditingMode".to_string(), 0));
        area_editing_mode.drag_enabled = true;

        area_editing_mode.add_group_list(context.color_blue, context.color_light_blue, vec!["Add Mode".to_string(), "Remove".to_string()]);
        area_editing_mode.set_rect((rect.0 + 10, rect.1 + 330, rect.2 - 20, 200), asset, context);

        area_widgets.push(add_area_button);
        area_widgets.push(del_area_button);
        area_widgets.push(rename_area_button);
        area_widgets.push(area_editing_mode);

        // Area Behavior

        let mut behavior_widgets : Vec<AtomWidget> = vec![];

        let mut node_list = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new_as_int("NodeList".to_string(), 0));
        node_list.drag_enabled = true;

        node_list.add_group_list(context.color_green, context.color_light_green, vec!["Enter Area".to_string(), "Leave Area".to_string(), "Inside Area".to_string(), "Spawn".to_string()]);

        node_list.add_group_list(context.color_blue, context.color_light_blue, vec!["Audio Area".to_string(), "Displace Tiles".to_string(),  "Message Area".to_string(), "Teleport Area".to_string()]);

        node_list.set_rect((rect.0 + 10, rect.1 + 200, rect.2 - 20, rect.3 - 200), asset, context);
        behavior_widgets.push(node_list);

        Self {
            rect,
            widgets,

            curr_layer                  : 1,

            tile_widgets,
            area_widgets,
            behavior_widgets,
        }
    }

    fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) {
        context.draw2d.draw_rect(frame, &self.rect, context.width, &context.color_black);

        let mode = self.get_editor_mode();

        for atom in &mut self.widgets {
           atom.draw(frame, context.width, anim_counter, asset, context);
        }

        if mode == RegionEditorMode::Tiles {
            for atom in &mut self.tile_widgets {
                atom.draw(frame, context.width, anim_counter, asset, context);
            }
        } else
        if mode == RegionEditorMode::Areas {
            for atom in &mut self.area_widgets {
                atom.draw(frame, context.width, anim_counter, asset, context);
            }
        } else
        if mode == RegionEditorMode::Behavior {
            self.area_widgets[0].draw(frame, context.width, anim_counter, asset, context);
            for atom in &mut self.behavior_widgets {
                atom.draw(frame, context.width, anim_counter, asset, context);
            }
        }

        if mode == RegionEditorMode::Tiles {
            if let Some(content) = content {
                if let Some(tile) = content.get_selected_tile() {
                    context.draw2d.draw_animated_tile(frame, &((self.rect.2 - 100) / 2, self.rect.1 + self.rect.3 - 110), asset.get_map_of_id(tile.0), context.width, &(tile.1, tile.2), anim_counter, 100);

                    //context.draw2d.draw_text_rect(frame, &(0, self.rect.1 + self.rect.3 - 22, self.rect.2, 20), context.width, &asset.get_editor_font("OpenSans"), 15.0, &format!("{}, {})", /*tile.0,*/ tile.1, tile.2), &context.color_white, &[0,0,0,255], crate::draw2d::TextAlignment::Center);
                }
                for atom in &mut self.tile_widgets {
                    atom.draw_overlay(frame, &self.rect, anim_counter, asset, context);
                }
            }
        } else
        if mode == RegionEditorMode::Areas {
            for atom in &mut self.area_widgets {
                atom.draw_overlay(frame, &self.rect, anim_counter, asset, context);
            }
        }
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_down(pos, asset, context) {

                if atom.atom_data.id == "Mode" {

                    if atom.curr_item_index == 3 {
                        if let Some(content) = content {
                            if let Some(region) = context.data.regions.get_mut(&content.get_region_id()) {

                                context.code_editor_is_active = true;
                                context.code_editor_just_opened = true;
                                context.code_editor_node_behavior_value.4 = region.data.settings.to_string();
                                context.code_editor_node_behavior_id.0 = 130000;
                            }
                        }
                    } else {
                        context.code_editor_is_active = false;
                    }
                }
                return true;
            }
        }

        let mode = self.get_editor_mode();

        if mode == RegionEditorMode::Tiles {
            for atom in &mut self.tile_widgets {
                if atom.mouse_down(pos, asset, context) {
                    if let Some(content) = content {
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
                        }
                    }
                    return true;
                }
            }
        } else
        if mode == RegionEditorMode::Areas || mode == RegionEditorMode::Behavior {
            for atom in &mut self.area_widgets {
                if atom.mouse_down(pos, asset, context) {
                    return true;
                }
                if mode == RegionEditorMode::Behavior {
                    break;
                }
            }
        }
        if mode == RegionEditorMode::Behavior {
            for atom in &mut self.behavior_widgets {
                if atom.mouse_down(pos, asset, context) {
                    return true;
                }
            }
        }

        false
    }

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) -> bool {

        let mut mode_was_updated = false;
        for atom in &mut self.widgets {
            if atom.mouse_up(pos, asset, context) {
                mode_was_updated = true;
            }
        }

        let mode = self.get_editor_mode();
        let tags = self.get_tags();
        let usage = self.get_tile_usage();

        // Tiles Mode
        if mode == RegionEditorMode::Tiles {
            for atom in &mut self.tile_widgets {
                if atom.mouse_up(pos, asset, context) {

                    if atom.new_selection.is_some() {
                        if atom.atom_data.id == "Tilemaps" {
                            if let Some(el_content) = content {
                                if let Some(tile_selector) = el_content.get_tile_selector() {
                                    if atom.curr_index == 0 {
                                        tile_selector.set_tile_type(vec![usage], None, tags, &asset);
                                    } else {
                                        tile_selector.set_tile_type(vec![usage], Some(atom.curr_index - 1), tags, &asset);
                                    }
                                    atom.dirty = true;
                                }
                            }
                        }

                    }
                    return true;
                }
            }
        } else
        if mode == RegionEditorMode::Areas || mode == RegionEditorMode::Behavior {
            if mode_was_updated {
                self.update_area_ui(context, content);
            }
            for atom in &mut self.area_widgets {
                if atom.mouse_up(pos, asset, context) {

                    if atom.atom_data.id == "Area" {
                        self.update_area_ui(context, content);
                        if let Some(el_content) = content {
                            if let Some(region) = context.data.regions.get_mut(&el_content.get_region_id()) {
                                if let Some(graph) = el_content.get_behavior_graph() {
                                    graph.set_behavior_id(region.behaviors[context.curr_region_area_index].data.id, context);
                                }
                            }
                        }
                    } else
                    if atom.atom_data.id == "Add Area" {
                        if let Some(el_content) = content {
                            if let Some(region) = context.data.regions.get_mut(&el_content.get_region_id()) {
                                let id = region.create_area();
                                context.curr_region_area_index = region.behaviors.len() - 1;
                                if let Some(graph) = el_content.get_behavior_graph() {
                                    graph.set_behavior_id(id, context);
                                }
                            }
                        }
                        self.update_area_ui(context, content);
                    } else
                    if atom.atom_data.id == "Delete" {
                        if let Some(el_content) = content {
                            if let Some(region) = context.data.regions.get_mut(&el_content.get_region_id()) {
                                region.delete_area(context.curr_region_area_index);
                            }
                        }
                        self.update_area_ui(context, content);
                    } else
                    if atom.atom_data.id == "Rename" {
                        use crate::editor::dialog::{DialogState, DialogEntry};
                        context.dialog_state = DialogState::Opening;
                        context.dialog_height = 0;
                        context.target_fps = 60;
                        context.dialog_entry = DialogEntry::NewName;
                        if let Some(el_content) = content {
                            if let Some(region) = context.data.regions.get_mut(&el_content.get_region_id()) {
                                context.dialog_new_name = region.get_area_names()[context.curr_region_area_index].clone();
                            }
                        }
                        self.update_area_ui(context, content);
                    }

                    return true;
                }
                if mode == RegionEditorMode::Behavior {
                    break;
                }
            }
        }
        if mode == RegionEditorMode::Behavior {
            for atom in &mut self.behavior_widgets {
                if atom.mouse_up(pos, asset, context) {

                    return true;
                }
            }
        }
        false
    }

    fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_hover(pos, asset, context) {
                return true;
            }
        }

        let mode = self.get_editor_mode();

        if mode == RegionEditorMode::Tiles {
            for atom in &mut self.tile_widgets {
                if atom.mouse_hover(pos, asset, context) {
                    return true;
                }
            }
        } else
        if mode == RegionEditorMode::Areas {
            for atom in &mut self.area_widgets {
                if atom.mouse_hover(pos, asset, context) {
                    return true;
                }
            }
        } else
        if mode == RegionEditorMode::Behavior {
            self.area_widgets[0].mouse_hover(pos, asset, context);
            for atom in &mut self.behavior_widgets {
                if atom.mouse_hover(pos, asset, context) {
                    return true;
                }
            }
        }
        false
    }

    fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) -> bool {

        let mode = self.get_editor_mode();
        if mode == RegionEditorMode::Behavior {
            if let Some(drag_context) = &self.behavior_widgets[0].drag_context {
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
                self.behavior_widgets[0].drag_context = None;
            }
        } else
        if mode == RegionEditorMode::Tiles {
            for atom in &mut self.tile_widgets {
                if atom.mouse_dragged(pos, asset, context) {
                    return true;
                }
            }
        } else
        if mode == RegionEditorMode::Areas {
            for atom in &mut self.area_widgets {
                if atom.mouse_dragged(pos, asset, context) {
                    return true;
                }
            }
        }
        false
    }

    /// Returns the current editor mode
    fn get_editor_mode(&self) -> RegionEditorMode {
        let mode = self.widgets[0].curr_item_index;

        match mode {
            1 => RegionEditorMode::Areas,
            2 => RegionEditorMode::Behavior,
            3 => RegionEditorMode::Settings,
            _ => RegionEditorMode::Tiles
        }
    }

    /// Update the area ui
    fn update_area_ui(&mut self, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) {

        if let Some(content) = content {
            if let Some(region) = context.data.regions.get(&content.get_region_id()) {

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
                    self.area_widgets[1].state = WidgetState::Normal;
                    self.area_widgets[2].state = WidgetState::Normal;
                    self.area_widgets[3].state = WidgetState::Normal;
                }

                for a in &mut self.area_widgets {
                    a.dirty = true;
                }

                context.curr_region_area_index = self.area_widgets[0].curr_index;

                region.save_data();
            }
        }
    }

    /// Sets a new name for the current area
    fn set_area_name(&mut self, name: String, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) {

        if let Some(el_content) = content {
            if let Some(region) = context.data.regions.get_mut(&el_content.get_region_id()) {
                region.data.areas[context.curr_region_area_index].name = name;
                self.update_area_ui(context, content);
            }
        }
    }

    /// Get the current tile usage
    fn get_tile_usage(&self) -> TileUsage {
        match self.tile_widgets[2].curr_item_index {
            1 => TileUsage::EnvRoad,
            2 => TileUsage::EnvBlocking,
            3 => TileUsage::Water,
            4 => TileUsage::UIElement,
            _ => TileUsage::Environment,
        }
    }

    /// Get the current tile_id if any
    fn get_tilemap_index(&self) -> Option<usize> {
        if self.tile_widgets[0].curr_index > 0 {
            return Some(self.tile_widgets[0].curr_index - 1);
        }
        None
    }

    /// Get the current tags
    fn get_tags(&self) -> Option<String> {
        if self.tile_widgets[1].text[0].len() > 0 {
            return Some(self.tile_widgets[1].text[0].clone());
        }
        None
    }

    /// Get the current layer
    fn get_layer(&self) -> usize {
        self.curr_layer
    }

    /// Set the tags
    fn set_region_tags(&mut self, tags: String, asset: &mut Asset, _context: &ScreenContext, content: &mut Option<Box<dyn EditorContent>>) {

        self.tile_widgets[1].text[0] = tags.clone().to_lowercase();
        self.tile_widgets[1].dirty = true;

        if let Some(content) = content {
            if let Some(tile_selector) = content.get_tile_selector() {
                tile_selector.set_tile_type(vec![self.get_tile_usage()], self.get_tilemap_index(), self.get_tags(), &asset);
            }
        }
    }

    /// Sets the area names
    fn set_area_names(&mut self, names: Vec<String>) {
        self.area_widgets[0].text = names;
        self.area_widgets[0].dirty = true;
    }
}