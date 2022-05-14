use crate::tileselector::TileSelectorWidget;
use crate::editor::regionoptions::RegionOptions;
use crate::editor::behavioroptions::BehaviorOptions;
use crate::editor::behavior_overview_options::BehaviorOverviewOptions;
use crate::editor::systemsoptions::SystemsOptions;
use crate::editor::systems_overview_options::SystemsOverviewOptions;
use crate::editor::itemsoptions::ItemsOptions;
use crate::editor::items_overview_options::ItemsOverviewOptions;
use crate::editor::regionwidget::RegionWidget;
use crate::editor::region_overview_options::RegionOverviewOptions;
use crate::editor::log::LogWidget;
use crate::editor::gameoptions::GameOptions;
use crate::widget:: {ScreenWidget, Widget, WidgetState, WidgetKey};
use crate::atom:: { AtomWidget, AtomWidgetType, AtomData };
use server::gamedata::behavior::{ BehaviorType };
use utilities::actions::*;

use crate::editor::dialog::DialogWidget;

use server::asset::Asset;

mod controlbar;
mod toolbar;
pub mod nodegraph;
mod tilemapoptions;
mod tilemapwidget;
mod region_overview_options;
mod regionwidget;
mod regionoptions;
mod behavioroptions;
mod behavior_overview_options;
mod systemsoptions;
mod systems_overview_options;
mod itemsoptions;
mod items_overview_options;
mod node;
mod node_preview;
mod statusbar;
pub mod dialog;
mod log;
mod gameoptions;
pub mod traits;

use crate::editor::toolbar::ToolBar;
use crate::editor::controlbar::ControlBar;
use tilemapwidget::TileMapWidget;

use crate::context::ScreenContext;
use crate::editor::node::{ NodeUserData, NodeWidget };
use crate::editor::nodegraph::NodeGraph;

use self::dialog::{DialogState, DialogEntry};
use self::tilemapoptions::TileMapOptions;
use self::statusbar::StatusBar;

use crate::editor::traits::{ EditorContent, GraphMode, EditorOptions };

#[derive (PartialEq)]
enum EditorState {
    TilesOverview,
    TilesDetail,
    RegionOverview,
    RegionDetail,
    BehaviorOverview,
    BehaviorDetail,
    SystemsOverview,
    SystemsDetail,
    ItemsOverview,
    ItemsDetail,
    GameDetail
}

/// The Editor struct
pub struct Editor<'a> {
    rect                            : (usize, usize, usize, usize),
    state                           : EditorState,
    context                         : ScreenContext<'a>,
    controlbar                      : ControlBar,
    toolbar                         : ToolBar,
    log                             : LogWidget,

    pub content                      : Vec<(Option<Box<dyn EditorOptions>>, Option<Box<dyn EditorContent>>)>,

    log_drag_start_pos              : Option<(usize, usize)>,
    log_drag_start_rect             : (isize, isize),

    left_width                      : usize,
    mouse_pos                       : (usize, usize),
    mouse_hover_pos                 : (usize, usize),

    dialog                          : DialogWidget,

    status_bar                      : StatusBar,

    project_to_load                 : Option<std::path::PathBuf>
}

impl ScreenWidget for Editor<'_> {

    fn new(asset: &mut Asset, width: usize, height: usize) -> Self where Self: Sized {

        asset.load_editor_font("OpenSans".to_string(), "Open_Sans/static/OpenSans/OpenSans-Regular.ttf".to_string());

        let left_width = 180_usize;
        let mut context = ScreenContext::new(width, height);

        let controlbar = ControlBar::new(vec!(), (0,0, width, context.toolbar_height / 2), asset, &mut context);
        let toolbar = ToolBar::new(vec!(), (0, context.toolbar_height / 2, width, context.toolbar_height / 2), asset, &mut context);

        //

        let dialog = DialogWidget::new(asset, &context);
        let log = LogWidget::new(&context);
        let mut status_bar = StatusBar::new();

        // Set current project

        let mut project_to_load: Option<std::path::PathBuf> = None;
        let project_list = context.get_project_list();

        if project_list.is_empty() {
            // Show Dialog to create a new project
            context.dialog_state = DialogState::Opening;
            context.dialog_height = 0;
            context.target_fps = 60;
            context.dialog_entry = DialogEntry::NewProjectName;
            context.dialog_new_name = "New Game".to_string();
        } else {
            project_to_load = context.get_project_path(project_list[0].clone());
            status_bar.add_message(format!("Loaded Documents >> Eldiron >> {}", project_list[0]));
        }

        Self {
            rect                    : (0, 0, width, height),
            state                   :  EditorState::TilesOverview,
            context,
            controlbar,
            toolbar,
            log,

            content                 : vec![],

            log_drag_start_pos      : None,
            log_drag_start_rect     : (0, 0),

            dialog,

            left_width,
            mouse_pos               : (0,0),
            mouse_hover_pos         : (0,0),

            status_bar,

            project_to_load,
        }
    }

    /// Update the editor
    fn update(&mut self) {
        // let start = self.get_time();
        if self.context.is_debugging == true {
            self.content[self.context.content_index * 2 + self.context.content_switch].1.as_mut().unwrap().update(&mut self.context);
        } else {
            self.context.data.tick();
        }
        // let stop = self.get_time();
        // println!("update time {:?}", stop - start);
    }

    fn key_down(&mut self, char: Option<char>, key: Option<WidgetKey>, asset: &mut Asset) -> bool {

        if self.context.is_running {
            if key == Some(WidgetKey::Up) {
                if let Some(cmd) = pack_action(self.context.player_id, "onMove".to_string(), PlayerDirection::North, "".to_string()) {
                    self.context.data.execute_packed_instance_action(cmd);
                }
            } else
            if key == Some(WidgetKey::Right) {
                if let Some(cmd) = pack_action(self.context.player_id, "onMove".to_string(), PlayerDirection::East, "".to_string()) {
                    self.context.data.execute_packed_instance_action(cmd);
                }
            } else
            if key == Some(WidgetKey::Down) {
                if let Some(cmd) = pack_action(self.context.player_id, "onMove".to_string(), PlayerDirection::South, "".to_string()) {
                    self.context.data.execute_packed_instance_action(cmd);
                }
            } else
            if key == Some(WidgetKey::Left) {
                if let Some(cmd) = pack_action(self.context.player_id, "onMove".to_string(), PlayerDirection::West, "".to_string()) {
                    self.context.data.execute_packed_instance_action(cmd);
                }
            }
        } else
        if self.context.dialog_state == DialogState::Open {
            return self.dialog.key_down(char, key, asset, &mut self.context);
        }
        false
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.context.width = width; self.rect.2 = width;
        self.context.height = height; self.rect.3 = height;
        self.controlbar.resize(width, height, &self.context);
        self.toolbar.resize(width, height, &self.context);

        for index in 0..self.content.len() {
            if self.content[index].0.is_some() {
                self.content[index].0.as_mut().unwrap().resize(self.left_width, height - self.context.toolbar_height, &self.context);
                self.content[index].1.as_mut().unwrap().resize(width, height - self.context.toolbar_height, &self.context);
            } else {
                self.content[index].1.as_mut().unwrap().resize(width - self.left_width, height - self.context.toolbar_height, &self.context);
            }
        }
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset) {

        // let start = self.get_time();

        // Playback
        if self.context.is_running && self.context.is_debugging == false {

            self.context.draw2d.draw_rect(frame, &self.rect, self.rect.2, &self.context.color_black);
            self.controlbar.draw(frame, anim_counter, asset, &mut self.context);

            let region_id = self.context.data.regions_ids[0];

            if let Some(region) = self.context.data.regions.get(&region_id) {
                // Find the behavior instance for the current behavior id
                let mut inst_index = 0_usize;
                let behavior_id = self.context.data.behaviors_ids[self.context.curr_behavior_index];
                for index in 0..self.context.data.instances.len() {
                    if self.context.data.instances[index].behavior_id == behavior_id {
                        inst_index = index;
                        break;
                    }
                }

                _ = self.context.draw2d.draw_region_centered_with_instances(frame, region, &self.rect, inst_index, self.rect.2, 32, anim_counter, asset, &self.context);
            }

            // let stop = self.get_time();
            // println!("draw time {:?}", stop - start);

            return;
        }

        // To update the variables
        if self.context.just_stopped_running {
            self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().set_dirty();
            //if let Some(preview) = &mut self.node_graph_behavior_details.preview {
            //    preview.dirty = true;
            //}
        }

        // Do we need to load a new project ?
        if self.project_to_load.is_some() {
            self.load_project(self.project_to_load.clone().unwrap(), asset);
            self.project_to_load = None;
        }

        self.controlbar.draw(frame, anim_counter, asset, &mut self.context);
        self.toolbar.draw(frame, anim_counter, asset, &mut self.context);

        //

        if self.content.is_empty() == false {
            let index = self.context.content_index * 2 + self.context.content_switch;
            let mut options : Option<Box<dyn EditorOptions>> = None;
            let mut content : Option<Box<dyn EditorContent>> = None;

            if let Some(element) = self.content.drain(index..index+1).next() {
                options = element.0;
                content = element.1;

                if let Some(mut el_option) = options {
                    el_option.draw(frame, anim_counter, asset, &mut self.context, &mut content);
                    options = Some(el_option);
                }

                if let Some(mut el_content) = content {
                    el_content.draw(frame, anim_counter, asset, &mut self.context, &mut options);
                    content = Some(el_content);
                }
            }
            self.content.insert(index, (options, content));
        } else {
            self.context.draw2d.draw_rect(frame, &self.rect, self.rect.2, &self.context.color_black);
        }

        // Log
        if self.state == EditorState::BehaviorDetail {
            self.log.draw(frame, anim_counter, asset, &mut self.context);
            self.context.draw2d.blend_slice_safe(frame, &self.log.buffer[..], &self.log.rect, self.context.width, &self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().get_rect());
        }

        /*
        if self.state == EditorState::Empty {
            self.context.draw2d.draw_rect(frame, &self.node_graph_tiles.rect, self.context.width, &self.context.color_black);
            self.status_bar.rect.0 = 0;
        } else
        if self.state == EditorState::TilesOverview {
            self.node_graph_tiles.draw(frame, anim_counter, asset, &mut self.context, &mut None);
            self.status_bar.rect.0 = 0;
        } else
        if self.state == EditorState::TilesDetail {
            self.tilemap_options.draw(frame, anim_counter, asset, &mut self.context, &mut None);
            self.tilemap.draw(frame, anim_counter, asset, &mut self.context, &mut None);
        } else
        if self.state == EditorState::RegionOverview {
            self.region_overview_options.draw(frame, anim_counter, asset, &mut self.context);
            self.node_graph_regions.draw(frame, anim_counter, asset, &mut self.context, &mut None);
            self.status_bar.rect.0 = 180;
        } else
        if self.state == EditorState::RegionDetail {
            self.region_options.draw(frame, anim_counter, asset, &mut self.context, &mut self.region_widget);
            self.region_widget.draw(frame, anim_counter, asset, &mut self.context, &mut self.region_options);
            self.status_bar.rect.0 = 180;
        } else
        if self.state == EditorState::BehaviorOverview {
            self.behavior_overview_options.draw(frame, anim_counter, asset, &mut self.context);
            self.node_graph_behavior.draw(frame, anim_counter, asset, &mut self.context, &mut None);
            self.status_bar.rect.0 = 180;
        } else
        if self.state == EditorState::BehaviorDetail {
            self.behavior_options.draw(frame, anim_counter, asset, &mut self.context);
            self.node_graph_behavior_details.draw(frame, anim_counter, asset, &mut self.context, &mut None);
            self.log.draw(frame, anim_counter, asset, &mut self.context);
            self.context.draw2d.blend_slice_safe(frame, &self.log.buffer[..], &self.log.rect, self.context.width, &self.node_graph_behavior_details.rect);
            self.status_bar.rect.0 = 180;
        } else
        if self.state == EditorState::SystemsOverview {
            self.systems_overview_options.draw(frame, anim_counter, asset, &mut self.context);
            self.node_graph_systems.draw(frame, anim_counter, asset, &mut self.context, &mut None);
            self.status_bar.rect.0 = 180;
        } else
        if self.state == EditorState::SystemsDetail {
            self.systems_options.draw(frame, anim_counter, asset, &mut self.context);
            self.node_graph_systems_details.draw(frame, anim_counter, asset, &mut self.context, &mut None);
            self.status_bar.rect.0 = 180;
        } else
        if self.state == EditorState::ItemsOverview {
            self.items_overview_options.draw(frame, anim_counter, asset, &mut self.context);
            self.node_graph_items.draw(frame, anim_counter, asset, &mut self.context, &mut None);
            self.status_bar.rect.0 = 180;
        } else
        if self.state == EditorState::ItemsDetail {
            self.items_options.draw(frame, anim_counter, asset, &mut self.context);
            self.node_graph_items_details.draw(frame, anim_counter, asset, &mut self.context, &mut None);
            self.status_bar.rect.0 = 180;
        } else
        if self.state == EditorState::GameDetail {
            self.game_options.draw(frame, anim_counter, asset, &mut self.context);
            self.node_graph_game_details.draw(frame, anim_counter, asset, &mut self.context, &mut None);
            self.status_bar.rect.0 = 180;
        }*/

        self.status_bar.draw(frame, anim_counter, asset, &mut self.context);

        // Drag and drop
        if let Some(drag_context) = &self.context.drag_context {
            if let Some(mut buffer) = drag_context.buffer {
                self.context.draw2d.blend_slice_safe(frame, &mut buffer[..], &(self.mouse_pos.0 as isize - drag_context.offset.0, self.mouse_pos.1 as isize - drag_context.offset.1, 180, 32), self.context.width, &self.rect);
            }
        }

        // Dialog
        if self.context.dialog_state != DialogState::Closed {
            self.dialog.rect.0 = (self.context.width - self.dialog.rect.2) / 2;
            self.dialog.draw(frame, anim_counter, asset, &mut self.context);
        } else
        if self.context.dialog_entry != DialogEntry::None {

            if self.context.dialog_entry == DialogEntry::NewProjectName {
                match self.context.create_project(self.context.dialog_new_name.clone()) {
                    Ok(path) => {
                        self.context.curr_project_path = path;
                        self.state = EditorState::TilesOverview;
                        self.controlbar.widgets[2].text = self.context.get_project_list();
                        self.controlbar.widgets[2].dirty = true;
                        self.project_to_load = self.context.get_project_path(self.context.dialog_new_name.clone());
                        self.status_bar.add_message(format!("Created Documents >> Eldiron >> {}", self.context.dialog_new_name.clone()));
                    },
                    Err(err) => print!("Error: {}", err)
                }
            } else
            if self.state == EditorState::TilesOverview && self.context.dialog_entry == DialogEntry::NodeGridSize && self.context.dialog_accepted == true {
                if let Some(value) = self.context.dialog_node_behavior_value.4.parse::<usize>().ok() {
                    let index = self.context.dialog_node_behavior_value.0 as usize;
                    if let Some(tilemap) = asset.tileset.maps.get_mut(&asset.tileset.maps_ids[index]) {
                        tilemap.settings.grid_size = value;
                        tilemap.save_settings();

                        /* TODO
                        self.node_graph_tiles.nodes[index].widgets[0].atom_data.data.4 = self.context.dialog_node_behavior_value.4.clone();
                        self.node_graph_tiles.nodes[index].widgets[0].dirty = true;
                        self.node_graph_tiles.nodes[index].dirty = true;
                        self.node_graph_tiles.dirty = true;
                        */
                    }
                }
            } else
            if self.state == EditorState::RegionOverview {
                if self.context.dialog_entry == DialogEntry::NewName && self.context.dialog_accepted == true {
                    println!("dialog ended {} {}", self.context.dialog_new_name, self.context.dialog_new_name_type);
                    /*
                    self.context.data.create_behavior(self.context.dialog_new_name.clone(), 0);

                    let mut node = NodeWidget::new(vec![self.context.dialog_new_name.clone()],
                    NodeUserData { position: (100, 50 + 150 * self.node_graph_behavior.nodes.len() as isize) });

                    let node_menu_atom = crate::atom::AtomWidget::new(vec!["Rename".to_string(), "Delete".to_string()], crate::atom::AtomWidgetType::NodeMenu, crate::atom::AtomData::new_as_int("menu".to_string(), 0));
                    node.menu = Some(node_menu_atom);

                    self.node_graph_behavior.nodes.push(node);
                    self.node_graph_behavior.dirty = true;
                    self.toolbar.widgets[0].text = self.context.data.behaviors_names.clone();
                    self.toolbar.widgets[0].dirty = true;
                    */
                } /*else {
                    if self.context.dialog_entry == DialogEntry::NodeName {
                        if self.context.dialog_accepted == true {
                            if let Some(behavior) = self.context.data.behaviors.get_mut(&self.context.data.behaviors_ids[self.context.curr_behavior_index]) {
                                behavior.rename(self.context.dialog_node_behavior_value.4.clone(), "behavior".to_string());
                            }
                        }
                    }
                    self.node_graph_behavior.update_from_dialog(&mut self.context);
                }*/
            } else
            if self.state == EditorState::RegionDetail && self.context.dialog_entry == DialogEntry::NewName && self.context.dialog_accepted == true {
                //self.region_options.set_area_name(self.context.dialog_new_name.clone(), &mut self.context, &mut self.region_widget);
            } else
            if self.state == EditorState::TilesDetail && self.context.dialog_entry == DialogEntry::Tags && self.context.dialog_accepted == true {
                //TODO self.tilemap_options.set_tags(self.context.dialog_new_name.clone(), asset, &self.context);
            } else
            if self.state == EditorState::RegionDetail && self.context.dialog_entry == DialogEntry::Tags && self.context.dialog_accepted == true {
                //self.region_options.set_tags(self.context.dialog_new_name.clone(), asset, &self.context, &mut self.region_widget);
            } else
            if self.state == EditorState::RegionDetail {
                //self.region_widget.behavior_graph.update_from_dialog(&mut self.context);
            } else
            if self.state == EditorState::BehaviorDetail {
                if self.context.dialog_entry == DialogEntry::NodeTile {
                    self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().set_node_atom_data(self.context.dialog_node_behavior_id.clone(), self.context.dialog_node_behavior_value.clone(), &mut self.context);
                } else {
                    self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().update_from_dialog(&mut self.context);
                }
            } else
            if self.state == EditorState::SystemsDetail {
                if self.context.dialog_entry == DialogEntry::NodeTile {
                    self.content[EditorState::SystemsDetail as usize].1.as_mut().unwrap().set_node_atom_data(self.context.dialog_node_behavior_id.clone(), self.context.dialog_node_behavior_value.clone(), &mut self.context);
                } else {
                    self.content[EditorState::SystemsDetail as usize].1.as_mut().unwrap().update_from_dialog(&mut self.context);
                }
            } else
            if self.state == EditorState::BehaviorOverview {
                if self.context.dialog_entry == DialogEntry::NewName && self.context.dialog_accepted == true {
                    //println!("dialog ended {} {}", self.context.dialog_new_name, self.context.dialog_new_name_type);
                    self.context.data.create_behavior(self.context.dialog_new_name.clone(), 0);

                    let mut node = NodeWidget::new(vec![self.context.dialog_new_name.clone()],
                    NodeUserData { position: (100, 50 + 150 * self.content[EditorState::BehaviorOverview as usize].1.as_mut().unwrap().get_nodes().unwrap().len() as isize) });

                    let node_menu_atom = crate::atom::AtomWidget::new(vec!["Rename".to_string(), "Delete".to_string()], crate::atom::AtomWidgetType::NodeMenu, crate::atom::AtomData::new_as_int("menu".to_string(), 0));
                    node.menu = Some(node_menu_atom);

                    self.content[EditorState::BehaviorOverview as usize].1.as_mut().unwrap().get_nodes().unwrap().push(node);
                    self.content[EditorState::BehaviorOverview as usize].1.as_mut().unwrap().set_dirty();
                    self.toolbar.widgets[0].text = self.context.data.behaviors_names.clone();
                    self.toolbar.widgets[0].dirty = true;
                } else {
                    if self.context.dialog_entry == DialogEntry::NodeName {
                        if self.context.dialog_accepted == true {
                            if let Some(behavior) = self.context.data.behaviors.get_mut(&self.context.data.behaviors_ids[self.context.curr_behavior_index]) {
                                behavior.rename(self.context.dialog_node_behavior_value.4.clone(), "behavior".to_string());
                            }
                        }
                    }
                    self.content[EditorState::BehaviorOverview as usize].1.as_mut().unwrap().update_from_dialog(&mut self.context);
                }
            } else
            if self.state == EditorState::SystemsOverview {
                if self.context.dialog_entry == DialogEntry::NewName && self.context.dialog_accepted == true {
                    //println!("dialog ended {} {}", self.context.dialog_new_name, self.context.dialog_new_name_type);
                    self.context.data.create_system(self.context.dialog_new_name.clone(), 0);

                    let mut node = NodeWidget::new(vec![self.context.dialog_new_name.clone()],
                    NodeUserData { position: (100, 50 + 150 * self.content[EditorState::SystemsOverview as usize].1.as_mut().unwrap().get_nodes().unwrap().len() as isize) });

                    let node_menu_atom = crate::atom::AtomWidget::new(vec!["Rename".to_string(), "Delete".to_string()], crate::atom::AtomWidgetType::NodeMenu, crate::atom::AtomData::new_as_int("menu".to_string(), 0));
                    node.menu = Some(node_menu_atom);

                    self.content[EditorState::SystemsOverview as usize].1.as_mut().unwrap().get_nodes().unwrap().push(node);
                    self.content[EditorState::SystemsOverview as usize].1.as_mut().unwrap().set_dirty();
                    self.toolbar.widgets[0].text = self.context.data.systems_names.clone();
                    self.toolbar.widgets[0].dirty = true;
                } else {
                    if self.context.dialog_entry == DialogEntry::NodeName {
                        if self.context.dialog_accepted == true {
                            if let Some(system) = self.context.data.systems.get_mut(&self.context.data.systems_ids[self.context.curr_systems_index]) {
                                system.rename(self.context.dialog_node_behavior_value.4.clone(), "systems".to_string());
                            }
                        }
                    }
                    self.content[EditorState::SystemsOverview as usize].1.as_mut().unwrap().update_from_dialog(&mut self.context);
                }
            }
            self.context.dialog_entry = DialogEntry::None;
        }

        // Draw overlay
        self.toolbar.draw_overlay(frame, &self.rect, anim_counter, asset, &mut self.context);

        // let stop = self.get_time();
        // println!("draw time {:?}", stop - start);
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {

        if self.context.dialog_state == DialogState::Open {
            return self.dialog.mouse_down(pos, asset, &mut self.context);
        }

        let mut consumed = false;

        if self.controlbar.mouse_down(pos, asset, &mut self.context) {
            consumed = true;
            if self.controlbar.show_help {
                match self.state {
                    EditorState::TilesOverview => _ = open::that("https://book.eldiron.com/tiles/overview.html"),
                    EditorState::TilesDetail => _ = open::that("https://book.eldiron.com/tiles/details.html"),

                    _ => _ = open::that("https://book.eldiron.com")
                }
                self.controlbar.show_help = false;
            }
        }
        if consumed == false && self.toolbar.mouse_down(pos, asset, &mut self.context) {

            // Tile Button
            if self.toolbar.widgets[1].clicked {
                if self.toolbar.widgets[1].selected {
                    self.content[EditorState::TilesOverview as usize].1.as_mut().unwrap().set_mode_and_rect( GraphMode::Overview, (0, self.rect.1 + self.context.toolbar_height, self.rect.2, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::TilesOverview;
                    self.content[EditorState::TilesOverview as usize].1.as_mut().unwrap().mark_all_dirty();

                    self.context.content_index = 0;
                    self.context.content_switch = 0;
                } else
                if self.toolbar.widgets[EditorState::TilesDetail as usize].right_selected && asset.tileset.maps_ids.is_empty() == false {
                    self.state = EditorState::TilesDetail;
                    self.context.curr_graph_type = BehaviorType::Tiles;

                    self.content[EditorState::TilesDetail as usize].1.as_mut().unwrap().set_tilemap_id(asset.tileset.maps_ids[self.context.curr_tileset_index]);

                    self.context.content_index = 0;
                    self.context.content_switch = 1;
                }

                for i in 2..=5 {
                    self.toolbar.widgets[i].selected = false;
                    self.toolbar.widgets[i].right_selected = false;
                    self.toolbar.widgets[i].dirty = true;
                }

                self.toolbar.widgets[0].text = asset.tileset.maps_names.clone();
                self.toolbar.widgets[0].curr_index = self.context.curr_tileset_index;
                self.toolbar.widgets[0].dirty = true;

                self.toolbar.widgets[6].checked = false;
                self.toolbar.widgets[6].dirty = true;
            } else
            // Region Button
            if self.toolbar.widgets[2].clicked {
                if self.toolbar.widgets[2].selected {
                    self.content[EditorState::RegionOverview as usize].1.as_mut().unwrap().set_mode_and_rect( GraphMode::Overview, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::RegionOverview;
                    self.content[EditorState::RegionOverview as usize].1.as_mut().unwrap().mark_all_dirty();

                    self.context.content_index = 1;
                    self.context.content_switch = 0;
                } else
                if self.toolbar.widgets[2].right_selected {
                    self.state = EditorState::RegionDetail;
                    self.context.curr_graph_type = BehaviorType::Regions;

                    let index = 3;
                    let mut options : Option<Box<dyn EditorOptions>> = None;
                    let mut content : Option<Box<dyn EditorContent>> = None;

                    if let Some(element) = self.content.drain(index..index+1).next() {
                        options = element.0;
                        content = element.1;
                        if let Some(mut el_content) = content {
                            el_content.set_region_id(self.context.data.regions_ids[self.context.curr_region_index], &mut self.context, &mut options);
                            content = Some(el_content);
                        }
                    }
                    self.content.insert(index, (options, content));

                    self.context.content_index = 1;
                    self.context.content_switch = 1;
                }

                for i in 1..=5 {
                    if i == 2 { continue; }
                    self.toolbar.widgets[i].selected = false;
                    self.toolbar.widgets[i].right_selected = false;
                    self.toolbar.widgets[i].dirty = true;
                }

                self.toolbar.widgets[0].text = self.context.data.regions_names.clone();
                self.toolbar.widgets[0].curr_index = self.context.curr_region_index;
                self.toolbar.widgets[0].dirty = true;

                self.toolbar.widgets[6].checked = false;
                self.toolbar.widgets[6].dirty = true;
            } else
            // Behavior Button
            if self.toolbar.widgets[3].clicked {
                if self.toolbar.widgets[3].selected {
                    self.content[EditorState::BehaviorOverview as usize].1.as_mut().unwrap().set_mode_and_rect( GraphMode::Overview, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::BehaviorOverview;
                    self.content[EditorState::BehaviorOverview as usize].1.as_mut().unwrap().mark_all_dirty();

                    self.context.content_index = 2;
                    self.context.content_switch = 0;
                } else
                if self.toolbar.widgets[3].right_selected {
                    self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().set_mode_and_rect( GraphMode::Detail, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::BehaviorDetail;
                    self.context.curr_graph_type = BehaviorType::Behaviors;
                    self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().set_behavior_id(self.context.data.behaviors_ids[self.context.curr_behavior_index] , &mut self.context);

                    self.context.content_index = 2;
                    self.context.content_switch = 1;
                }

                for i in 1..=5 {
                    if i == 3 { continue; }
                    self.toolbar.widgets[i].selected = false;
                    self.toolbar.widgets[i].right_selected = false;
                    self.toolbar.widgets[i].dirty = true;
                }

                self.toolbar.widgets[0].text = self.context.data.behaviors_names.clone();
                self.toolbar.widgets[0].curr_index = self.context.curr_behavior_index;
                self.toolbar.widgets[0].dirty = true;

                self.toolbar.widgets[6].checked = false;
                self.toolbar.widgets[6].dirty = true;
            } else
            // Systems Button
            if self.toolbar.widgets[4].clicked {
                if self.toolbar.widgets[4].selected {
                    self.content[EditorState::SystemsOverview as usize].1.as_mut().unwrap().set_mode_and_rect( GraphMode::Overview, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::SystemsOverview;
                    self.content[EditorState::SystemsOverview as usize].1.as_mut().unwrap().mark_all_dirty();

                    self.context.content_index = 3;
                    self.context.content_switch = 0;
                } else
                if self.toolbar.widgets[4].right_selected {
                    self.content[EditorState::SystemsDetail as usize].1.as_mut().unwrap().set_mode_and_rect( GraphMode::Detail, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::SystemsDetail;
                    self.context.curr_graph_type = BehaviorType::Systems;
                    self.content[EditorState::SystemsDetail as usize].1.as_mut().unwrap().set_behavior_id(self.context.data.systems_ids[self.context.curr_systems_index] , &mut self.context);

                    self.context.content_index = 3;
                    self.context.content_switch = 1;
                }

                for i in 1..=5 {
                    if i == 4 { continue; }
                    self.toolbar.widgets[i].selected = false;
                    self.toolbar.widgets[i].right_selected = false;
                    self.toolbar.widgets[i].dirty = true;
                }

                self.toolbar.widgets[0].text = self.context.data.systems_names.clone();
                self.toolbar.widgets[0].curr_index = self.context.curr_systems_index;
                self.toolbar.widgets[0].dirty = true;

                self.toolbar.widgets[6].checked = false;
                self.toolbar.widgets[6].dirty = true;
            } else
            // Items Button
            if self.toolbar.widgets[5].clicked {
                if self.toolbar.widgets[5].selected {
                    self.content[EditorState::ItemsOverview as usize].1.as_mut().unwrap().set_mode_and_rect( GraphMode::Overview, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::ItemsOverview;
                    self.content[EditorState::ItemsOverview as usize].1.as_mut().unwrap().mark_all_dirty();

                    self.context.content_index = 4;
                    self.context.content_switch = 0;
                } else
                if self.toolbar.widgets[5].right_selected {
                    self.content[EditorState::ItemsDetail as usize].1.as_mut().unwrap().set_mode_and_rect( GraphMode::Detail, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::ItemsDetail;
                    self.context.curr_graph_type = BehaviorType::Items;
                    //self.node_graph_items_details.set_behavior_id(self.context.data.items_ids[self.context.curr_items_index], &mut self.context);

                    self.context.content_index = 4;
                    self.context.content_switch = 1;
                }

                for i in 1..5 {
                    self.toolbar.widgets[i].selected = false;
                    self.toolbar.widgets[i].right_selected = false;
                    self.toolbar.widgets[i].dirty = true;
                }

                self.toolbar.widgets[0].text = self.context.data.items_names.clone();
                self.toolbar.widgets[0].curr_index = self.context.curr_items_index;
                self.toolbar.widgets[0].dirty = true;

                self.toolbar.widgets[6].checked = false;
                self.toolbar.widgets[6].dirty = true;
            } else
            // Game Button
            if self.toolbar.widgets[6].clicked {
                self.content[EditorState::GameDetail as usize].1.as_mut().unwrap().set_mode_and_rect( GraphMode::Detail, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                self.state = EditorState::GameDetail;
                self.context.curr_graph_type = BehaviorType::GameLogic;
                self.toolbar.widgets[6].checked = true;
                self.content[EditorState::GameDetail as usize].1.as_mut().unwrap().set_behavior_id(0, &mut self.context);

                for i in 1..=5 {
                    self.toolbar.widgets[i].selected = false;
                    self.toolbar.widgets[i].right_selected = false;
                    self.toolbar.widgets[i].dirty = true;
                }

                self.toolbar.widgets[0].text = vec!["Game Logic".to_string()];
                self.toolbar.widgets[0].curr_index = 0;
                self.toolbar.widgets[0].dirty = true;

                self.context.content_index = 5;
                self.context.content_switch = 0;
            }
            consumed = true;
        }

        let index = self.context.content_index * 2 + self.context.content_switch;
        let mut options : Option<Box<dyn EditorOptions>> = None;
        let mut content : Option<Box<dyn EditorContent>> = None;

        if let Some(element) = self.content.drain(index..index+1).next() {
            options = element.0;
            content = element.1;

            if consumed == false {
                if let Some(mut el_option) = options {
                    consumed = el_option.mouse_down(pos, asset, &mut self.context, &mut content);
                    options = Some(el_option);
                }
            }

            if consumed == false {
                if let Some(mut el_content) = content {
                    consumed = el_content.mouse_down(pos, asset, &mut self.context, &mut options, &mut Some(&mut self.toolbar));
                    content = Some(el_content);
                }
            }
        }
        self.content.insert(index, (options, content));

        /*
        if consumed == false && self.state == EditorState::TilesOverview {
            if consumed == false && self.node_graph_tiles.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
                if self.node_graph_tiles.clicked {
                    self.toolbar.widgets[0].curr_index = self.context.curr_tileset_index;
                    self.toolbar.widgets[0].dirty = true;
                    self.tilemap.set_tilemap_id(asset.tileset.maps_ids[self.context.curr_tileset_index]);
                    self.node_graph_tiles.clicked = false;
                }
                if self.node_graph_tiles.clicked_preview {
                    self.state = EditorState::TilesDetail;
                    self.node_graph_tiles.clicked_preview = false;
                    self.toolbar.widgets[1].selected = false;
                    self.toolbar.widgets[1].right_selected = true;
                    self.toolbar.widgets[1].dirty = true;
                }
            }
        } else
        if consumed == false && self.state == EditorState::TilesDetail {
            if consumed == false && self.tilemap_options.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.tilemap.mouse_down(pos, asset, &mut self.context) {
                if self.tilemap.clicked == true {
                    self.tilemap_options.adjust_tile_usage(asset, &self.context);
                }
                if self.context.curr_tile.is_some() {
                    self.tilemap_options.set_state(WidgetState::Normal);
                } else {
                    self.tilemap_options.set_state(WidgetState::Disabled);
                }
                consumed = true;
            }
        } else
        if consumed == false && self.state == EditorState::RegionOverview {
            if consumed == false && self.region_overview_options.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.node_graph_regions.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
                if self.node_graph_regions.clicked {
                    self.toolbar.widgets[0].curr_index = self.context.curr_region_index;
                    self.toolbar.widgets[0].dirty = true;
                    self.region_widget.set_region_id(self.context.data.regions_ids[self.context.curr_region_index], &mut self.context, &mut self.region_options);
                    self.node_graph_regions.clicked = false;
                }
                if self.node_graph_regions.clicked_preview {
                    self.state = EditorState::RegionDetail;
                    self.node_graph_regions.clicked_preview = false;
                    self.toolbar.widgets[2].selected = false;
                    self.toolbar.widgets[2].right_selected = true;
                    self.toolbar.widgets[2].dirty = true;
                }
            }
        } else
        if consumed == false && self.state == EditorState::RegionDetail {
            if consumed == false && self.region_options.mouse_down(pos, asset, &mut self.context, &mut self.region_widget) {
                consumed = true;
            }
            if consumed == false && self.region_widget.mouse_down(pos, asset, &mut self.context, &mut self.region_options) {
                consumed = true;
            }
        }
        if consumed == false && self.state == EditorState::BehaviorOverview {
            if consumed == false && self.behavior_overview_options.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.node_graph_behavior.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
                if self.node_graph_behavior.clicked {
                    self.toolbar.widgets[0].curr_index = self.context.curr_behavior_index;
                    self.toolbar.widgets[0].dirty = true;
                    self.node_graph_behavior.clicked = false;
                }
                if self.node_graph_behavior.clicked_preview {
                    self.state = EditorState::BehaviorDetail;
                    self.node_graph_behavior.clicked_preview = false;
                    self.toolbar.widgets[3].selected = false;
                    self.toolbar.widgets[3].right_selected = true;
                    self.toolbar.widgets[3].dirty = true;
                    self.context.curr_behavior_index = self.toolbar.widgets[0].curr_index;
                    self.node_graph_behavior_details.set_behavior_id(self.context.data.behaviors_ids[self.context.curr_behavior_index] , &mut self.context);
                }
            }
        }
        if consumed == false && self.state == EditorState::BehaviorDetail {
            if consumed == false && self.behavior_options.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.context.contains_pos_for_isize(pos, self.log.rect) {
                consumed = true;
                self.log_drag_start_pos = Some(pos.clone());
                self.log_drag_start_rect = (self.log.rect.0, self.log.rect.1);
            }
            if consumed == false && self.node_graph_behavior_details.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
        }
        if consumed == false && self.state == EditorState::SystemsOverview {
            if consumed == false && self.systems_overview_options.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.node_graph_systems.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
                if self.node_graph_systems.clicked {
                    self.toolbar.widgets[0].curr_index = self.context.curr_systems_index;
                    self.toolbar.widgets[0].dirty = true;
                    self.node_graph_systems.clicked = false;
                }
                if self.node_graph_systems.clicked_preview {
                    self.state = EditorState::SystemsDetail;
                    self.node_graph_systems.clicked_preview = false;
                    self.toolbar.widgets[4].selected = false;
                    self.toolbar.widgets[4].right_selected = true;
                    self.toolbar.widgets[4].dirty = true;
                    self.context.curr_systems_index = self.toolbar.widgets[0].curr_index;
                    self.node_graph_systems_details.set_behavior_id(self.context.data.systems_ids[self.context.curr_systems_index] , &mut self.context);
                }
            }
        }
        if consumed == false && self.state == EditorState::SystemsDetail {
            if consumed == false && self.systems_options.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.node_graph_systems_details.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
        }
        if consumed == false && self.state == EditorState::ItemsOverview {
            if consumed == false && self.items_overview_options.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.node_graph_items.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
                if self.node_graph_items.clicked {
                    self.toolbar.widgets[0].curr_index = self.context.curr_items_index;
                    self.toolbar.widgets[0].dirty = true;
                    self.node_graph_items.clicked = false;
                }
                if self.node_graph_items.clicked_preview {
                    self.state = EditorState::ItemsDetail;
                    self.node_graph_items.clicked_preview = false;
                    self.toolbar.widgets[5].selected = false;
                    self.toolbar.widgets[5].right_selected = true;
                    self.toolbar.widgets[5].dirty = true;
                    self.context.curr_systems_index = self.toolbar.widgets[0].curr_index;
                    self.node_graph_systems_details.set_behavior_id(self.context.data.systems_ids[self.context.curr_systems_index] , &mut self.context);
                }
            }
        }
        if consumed == false && self.state == EditorState::ItemsDetail {
            if consumed == false && self.items_options.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.node_graph_items_details.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
        }
        if consumed == false && self.state == EditorState::GameDetail {
            if consumed == false && self.game_options.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.node_graph_game_details.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
        }*/

        consumed
    }

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {

        if self.context.dialog_state == DialogState::Open {
            return self.dialog.mouse_up(pos, asset, &mut self.context);
        }

        self.log_drag_start_pos = None;

        let mut consumed = false;
        if self.controlbar.mouse_up(pos, asset, &mut self.context) {
            consumed = true;
        }
        if self.toolbar.mouse_up(pos, asset, &mut self.context) {

            if self.toolbar.widgets[0].new_selection.is_some() {
                if self.state == EditorState::TilesOverview || self.state == EditorState::TilesDetail {
                    self.content[0].1.as_mut().unwrap().changed_selection(self.context.curr_tileset_index, self.toolbar.widgets[0].curr_index);
                    self.context.curr_tileset_index = self.toolbar.widgets[0].curr_index;
                    self.content[1].1.as_mut().unwrap().set_tilemap_id(asset.tileset.maps_ids[self.context.curr_tileset_index]);
                    self.context.curr_tile = None;
                    self.content[1].0.as_mut().unwrap().set_state(WidgetState::Disabled);
                } else
                if self.state == EditorState::RegionOverview || self.state == EditorState::RegionDetail {
                    self.content[2].1.as_mut().unwrap().changed_selection(self.context.curr_region_index, self.toolbar.widgets[0].curr_index);
                    self.context.curr_region_index = self.toolbar.widgets[0].curr_index;

                    let index = 3;
                    let mut options : Option<Box<dyn EditorOptions>> = None;
                    let mut content : Option<Box<dyn EditorContent>> = None;

                    if let Some(element) = self.content.drain(index..index+1).next() {
                        options = element.0;
                        content = element.1;

                        if let Some(mut el_content) = content {

                            el_content.set_region_id(self.context.data.regions_ids[self.context.curr_region_index], &mut self.context, &mut options);
                            content = Some(el_content);
                        }
                    }
                    self.content.insert(index, (options, content));
                } else
                if self.state == EditorState::BehaviorOverview || self.state == EditorState::BehaviorDetail {
                    self.content[EditorState::BehaviorOverview as usize].1.as_mut().unwrap().changed_selection(self.context.curr_behavior_index, self.toolbar.widgets[0].curr_index);
                    self.context.curr_behavior_index = self.toolbar.widgets[0].curr_index;
                    self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().set_behavior_id(self.context.data.behaviors_ids[self.context.curr_behavior_index] , &mut self.context);
                } else
                if self.state == EditorState::SystemsOverview || self.state == EditorState::SystemsDetail {
                    self.content[EditorState::SystemsOverview as usize].1.as_mut().unwrap().changed_selection(self.context.curr_systems_index, self.toolbar.widgets[0].curr_index);
                    self.context.curr_systems_index = self.toolbar.widgets[0].curr_index;
                    self.content[EditorState::SystemsDetail as usize].1.as_mut().unwrap().set_behavior_id(self.context.data.systems_ids[self.context.curr_systems_index] , &mut self.context);
                }
                self.toolbar.widgets[0].new_selection = None;
            }
            consumed = true;
        }

        let index = self.context.content_index * 2 + self.context.content_switch;
        let mut options : Option<Box<dyn EditorOptions>> = None;
        let mut content : Option<Box<dyn EditorContent>> = None;

        if let Some(element) = self.content.drain(index..index+1).next() {
            options = element.0;
            content = element.1;

            if consumed == false {
                if let Some(mut el_option) = options {
                    consumed = el_option.mouse_up(pos, asset, &mut self.context, &mut content);
                    options = Some(el_option);
                }
            }

            if consumed == false {
                if let Some(mut el_content) = content {
                    consumed = el_content.mouse_up(pos, asset, &mut self.context, &mut options, &mut Some(&mut self.toolbar));
                    content = Some(el_content);
                }
            }
        }
        self.content.insert(index, (options, content));
        /*
        if self.state == EditorState::TilesOverview {
            if consumed == false && self.node_graph_tiles.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::TilesDetail {
            if self.tilemap_options.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
            if self.tilemap.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::RegionOverview {
            if self.region_overview_options.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
        }
        if self.state == EditorState::RegionDetail {
            if self.region_options.mouse_up(pos, asset, &mut self.context, &mut self.region_widget) {
                consumed = true;
            }
            if self.region_widget.mouse_up(pos, asset, &mut self.context, &mut self.region_options) {
                consumed = true;
            }
        } else
        if self.state == EditorState::BehaviorOverview {
            if self.behavior_overview_options.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
            if self.node_graph_behavior.mouse_up(pos, asset, &mut self.context) {
                consumed = true;

                // In case a behavior was deleted
                if self.toolbar.widgets[0].text.len() != self.context.data.behaviors_names.len() {
                    self.toolbar.widgets[0].text = self.context.data.behaviors_names.clone();
                    self.context.curr_behavior_index = 0;
                    self.toolbar.widgets[0].dirty = true;
                    self.toolbar.widgets[0].curr_index = 0;
                }
            }
        } else
        if self.state == EditorState::BehaviorDetail {
            if self.behavior_options.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
            if self.node_graph_behavior_details.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::SystemsOverview {
            if self.systems_overview_options.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
            if self.node_graph_systems.mouse_up(pos, asset, &mut self.context) {
                consumed = true;

                // In case a behavior was deleted
                if self.toolbar.widgets[0].text.len() != self.context.data.systems_names.len() {
                    self.toolbar.widgets[0].text = self.context.data.systems_names.clone();
                    self.context.curr_systems_index = 0;
                    self.toolbar.widgets[0].dirty = true;
                    self.toolbar.widgets[0].curr_index = 0;
                }
            }
        } else
        if self.state == EditorState::SystemsDetail {
            if self.systems_options.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
            if self.node_graph_systems_details.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::ItemsOverview {
            if self.items_overview_options.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
            if self.node_graph_items.mouse_up(pos, asset, &mut self.context) {
                consumed = true;

                // In case a behavior was deleted
                if self.toolbar.widgets[0].text.len() != self.context.data.items_names.len() {
                    self.toolbar.widgets[0].text = self.context.data.items_names.clone();
                    self.context.curr_items_index = 0;
                    self.toolbar.widgets[0].dirty = true;
                    self.toolbar.widgets[0].curr_index = 0;
                }
            }
        } else
        if self.state == EditorState::ItemsDetail {
            if self.items_options.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
            if self.node_graph_items_details.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::GameDetail {
            if self.game_options.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
            if self.node_graph_game_details.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
        }*/

        // Node Drag ?
        if let Some(drag_context) = &self.context.drag_context {

            if self.state == EditorState::RegionOverview {
                let rect = self.content[EditorState::RegionOverview as usize].1.as_mut().unwrap().get_rect();
                let offset = self.content[EditorState::RegionOverview as usize].1.as_mut().unwrap().get_offset();
                if self.context.contains_pos_for(pos, rect) {

                    let mut position = (pos.0 as isize, pos.1 as isize);
                    position.0 -= rect.0 as isize + offset.0 + drag_context.offset.0;
                    position.1 -= rect.1 as isize + offset.1 + drag_context.offset.1;

                    self.context.dialog_state = DialogState::Opening;
                    self.context.dialog_height = 0;
                    self.context.target_fps = 60;
                    self.context.dialog_entry = DialogEntry::NewName;
                    self.context.dialog_new_name = format!("New {}", drag_context.text).to_string();
                    self.context.dialog_new_name_type = format!("NewRegion_{}", drag_context.text);
                    self.context.dialog_new_node_position = position;
                }
            } else
            if self.state == EditorState::RegionDetail {
                /* TODO
                if self.context.contains_pos_for(pos, self.region_widget.behavior_graph.rect) {
                    let mut position = (pos.0 as isize, pos.1 as isize);
                    position.0 -= self.region_widget.behavior_graph.rect.0 as isize + self.region_widget.behavior_graph.offset.0 + drag_context.offset.0;
                    position.1 -= self.region_widget.behavior_graph.rect.1 as isize + self.region_widget.behavior_graph.offset.1 + drag_context.offset.1;

                    self.region_widget.behavior_graph.add_node_of_name(drag_context.text.clone(), position, &mut self.context);
                }*/
            } else
            if self.state == EditorState::BehaviorOverview {
                let rect = self.content[EditorState::BehaviorOverview as usize].1.as_mut().unwrap().get_rect();
                let offset = self.content[EditorState::BehaviorOverview as usize].1.as_mut().unwrap().get_offset();
                if self.context.contains_pos_for(pos, rect) {

                    let mut position = (pos.0 as isize, pos.1 as isize);
                    position.0 -= rect.0 as isize + offset.0 + drag_context.offset.0;
                    position.1 -= rect.1 as isize + offset.1 + drag_context.offset.1;

                    self.context.dialog_state = DialogState::Opening;
                    self.context.dialog_height = 0;
                    self.context.target_fps = 60;
                    self.context.dialog_entry = DialogEntry::NewName;
                    self.context.dialog_new_name = "New Behavior".to_string();
                    self.context.dialog_new_name_type = format!("NewBehavior_{}", drag_context.text);
                    self.context.dialog_new_node_position = position;
                }
            } else
            if self.state == EditorState::BehaviorDetail {
                let rect = self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().get_rect();
                let offset = self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().get_offset();
                if self.context.contains_pos_for(pos, rect) {

                    let mut position = (pos.0 as isize, pos.1 as isize);
                    position.0 -= rect.0 as isize + offset.0 + drag_context.offset.0;
                    position.1 -= rect.1 as isize + offset.1 + drag_context.offset.1;

                    self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().add_node_of_name(drag_context.text.clone(), position, &mut self.context);
                }
            } else
            if self.state == EditorState::SystemsOverview {
                let rect = self.content[EditorState::SystemsOverview as usize].1.as_mut().unwrap().get_rect();
                let offset = self.content[EditorState::SystemsOverview as usize].1.as_mut().unwrap().get_offset();
                if self.context.contains_pos_for(pos, rect) {

                    let mut position = (pos.0 as isize, pos.1 as isize);
                    position.0 -= rect.0 as isize + offset.0 + drag_context.offset.0;
                    position.1 -= rect.1 as isize + offset.1 + drag_context.offset.1;

                    self.context.dialog_state = DialogState::Opening;
                    self.context.dialog_height = 0;
                    self.context.target_fps = 60;
                    self.context.dialog_entry = DialogEntry::NewName;
                    self.context.dialog_new_name = "New System".to_string();
                    self.context.dialog_new_name_type = format!("NewBehavior_{}", drag_context.text);
                    self.context.dialog_new_node_position = position;
                }
            } else
            if self.state == EditorState::SystemsDetail {
                let rect = self.content[EditorState::SystemsDetail as usize].1.as_mut().unwrap().get_rect();
                let offset = self.content[EditorState::SystemsDetail as usize].1.as_mut().unwrap().get_offset();
                if self.context.contains_pos_for(pos, rect) {

                    let mut position = (pos.0 as isize, pos.1 as isize);
                    position.0 -= rect.0 as isize + offset.0 + drag_context.offset.0;
                    position.1 -= rect.1 as isize + offset.1 + drag_context.offset.1;

                     self.content[EditorState::SystemsDetail as usize].1.as_mut().unwrap().add_node_of_name(drag_context.text.clone(), position, &mut self.context);
                }
            } else
            if self.state == EditorState::GameDetail {
                let rect = self.content[EditorState::GameDetail as usize].1.as_mut().unwrap().get_rect();
                let offset = self.content[EditorState::GameDetail as usize].1.as_mut().unwrap().get_offset();
                if self.context.contains_pos_for(pos, rect) {

                    let mut position = (pos.0 as isize, pos.1 as isize);
                    position.0 -= rect.0 as isize + offset.0 + drag_context.offset.0;
                    position.1 -= rect.1 as isize + offset.1 + drag_context.offset.1;

                     self.content[EditorState::GameDetail as usize].1.as_mut().unwrap().add_node_of_name(drag_context.text.clone(), position, &mut self.context);
                }
            }


            // Cleanup DnD
            self.context.drag_context = None;
            self.context.target_fps = self.context.default_fps;
            consumed = true;
        }
        consumed
    }

    fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {

        if self.context.dialog_state == DialogState::Open {
            return self.dialog.mouse_dragged(pos, asset, &mut self.context);
        }

        if let Some(log_drag_start_pos) = self.log_drag_start_pos {
            self.log.rect.0 = self.log_drag_start_rect.0 - (log_drag_start_pos.0 as isize - pos.0 as isize);
            self.log.rect.1 = self.log_drag_start_rect.1 - (log_drag_start_pos.1 as isize - pos.1 as isize);
            return true;
        }

        let mut consumed = false;
        self.toolbar.mouse_dragged(pos, asset, &mut self.context);

        let index = self.context.content_index * 2 + self.context.content_switch;
        let mut options : Option<Box<dyn EditorOptions>> = None;
        let mut content : Option<Box<dyn EditorContent>> = None;

        if let Some(element) = self.content.drain(index..index+1).next() {
            options = element.0;
            content = element.1;

            if consumed == false {
                if let Some(mut el_option) = options {
                    consumed = el_option.mouse_dragged(pos, asset, &mut self.context, &mut content);
                    options = Some(el_option);
                }
            }

            if consumed == false {
                if let Some(mut el_content) = content {
                    consumed = el_content.mouse_dragged(pos, asset, &mut self.context, &mut options, &mut Some(&mut self.toolbar));
                    content = Some(el_content);
                }
            }
        }
        self.content.insert(index, (options, content));
        /*
        if self.state == EditorState::TilesOverview {
            if consumed == false && self.node_graph_tiles.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::TilesDetail {
            if self.tilemap_options.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
            if self.tilemap.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::RegionOverview {
            if consumed == false && self.region_overview_options.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.node_graph_regions.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::RegionDetail {
            if consumed == false && self.region_options.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.region_widget.mouse_dragged(pos, asset, &mut self.context, &mut self.region_options) {
                consumed = true;
            }
        } else
        if self.state == EditorState::BehaviorOverview {
            if consumed == false && self.behavior_overview_options.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.node_graph_behavior.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::BehaviorDetail {
            if consumed == false && self.behavior_options.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.node_graph_behavior_details.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::SystemsOverview {
            if consumed == false && self.systems_overview_options.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.node_graph_systems.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::SystemsDetail {
            if consumed == false && self.systems_options.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.node_graph_systems_details.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::ItemsOverview {
            if consumed == false && self.items_overview_options.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.node_graph_items.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::ItemsDetail {
            if consumed == false && self.items_options.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.node_graph_items_details.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::GameDetail {
            if consumed == false && self.game_options.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.node_graph_game_details.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
        }*/
        self.mouse_pos = pos.clone();
        consumed
    }

    fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {

        if self.context.dialog_state == DialogState::Open {
            return self.dialog.mouse_hover(pos, asset, &mut self.context);
        }

        let mut consumed = false;

        if consumed == false && self.toolbar.mouse_hover(pos, asset, &mut self.context) {
            consumed = true;
        } else {

            self.mouse_hover_pos = pos.clone();

            let index = self.context.content_index * 2 + self.context.content_switch;
            let mut options : Option<Box<dyn EditorOptions>> = None;
            let mut content : Option<Box<dyn EditorContent>> = None;

            if let Some(element) = self.content.drain(index..index+1).next() {
                options = element.0;
                content = element.1;

                if consumed == false {
                    if let Some(mut el_option) = options {
                        consumed = el_option.mouse_hover(pos, asset, &mut self.context, &mut content);
                        options = Some(el_option);
                    }
                }

                if consumed == false {
                    if let Some(mut el_content) = content {
                        consumed = el_content.mouse_hover(pos, asset, &mut self.context, &mut options, &mut Some(&mut self.toolbar));
                        content = Some(el_content);
                    }
                }
            }
            self.content.insert(index, (options, content));
        }

        /*
        if consumed == false && self.toolbar.mouse_hover(pos, asset, &mut self.context) {
            consumed = true;
        } else
        if self.state == EditorState::TilesDetail {
            if consumed == false && self.tilemap_options.mouse_hover(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::RegionDetail {
            if consumed == false && self.region_options.mouse_hover(pos, asset, &mut self.context, &mut self.region_widget) {
                consumed = true;
            }
            if consumed == false && self.region_widget.mouse_hover(pos, asset, &mut self.context, &mut self.region_options) {
                consumed = true;
            }
        } else
        if self.state == EditorState::BehaviorDetail {
            if consumed == false && self.node_graph_behavior_details.mouse_hover(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::SystemsDetail {
            if consumed == false && self.node_graph_systems_details.mouse_hover(pos, asset, &mut self.context) {
                consumed = true;
            }
        }
        */
        consumed
    }

    fn mouse_wheel(&mut self, delta: (isize, isize), asset: &mut Asset) -> bool {

        if self.context.dialog_state == DialogState::Open {
            return self.dialog.mouse_wheel(delta, asset, &mut self.context);
        }

        let mut consumed = false;
        let index = self.context.content_index * 2 + self.context.content_switch;
        let mut options : Option<Box<dyn EditorOptions>> = None;
        let mut content : Option<Box<dyn EditorContent>> = None;

        if let Some(element) = self.content.drain(index..index+1).next() {
            options = element.0;
            content = element.1;

            if consumed == false {
                if let Some(mut el_option) = options {
                    consumed = el_option.mouse_wheel(delta, asset, &mut self.context, &mut content);
                    options = Some(el_option);
                }
            }

            if consumed == false {
                if let Some(mut el_content) = content {
                    consumed = el_content.mouse_wheel(delta, asset, &mut self.context, &mut options, &mut Some(&mut self.toolbar));
                    content = Some(el_content);
                }
            }
        }
        self.content.insert(index, (options, content));
        /*
        let mut consumed = false;
        if consumed == false && self.toolbar.mouse_wheel(delta, asset, &mut self.context) {
            consumed = true;
        } else
        if self.state == EditorState::TilesOverview {
            if consumed == false && self.node_graph_tiles.mouse_wheel(delta, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::TilesDetail {
            if consumed == false && self.tilemap.mouse_wheel(delta, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::RegionOverview {
            if consumed == false && self.node_graph_regions.mouse_wheel(delta, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::RegionDetail {
            if consumed == false && self.context.contains_pos_for(self.mouse_hover_pos,self.region_widget.rect) && self.region_widget.mouse_wheel(delta, asset, &mut self.context, &mut self.region_options) {
                consumed = true;
            }
        } else
        if self.state == EditorState::BehaviorOverview {
            if consumed == false && self.node_graph_behavior.mouse_wheel(delta, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::BehaviorDetail {
            if consumed == false && self.node_graph_behavior_details.mouse_wheel(delta, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::SystemsOverview {
            if consumed == false && self.node_graph_systems.mouse_wheel(delta, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::SystemsDetail {
            if consumed == false && self.node_graph_systems_details.mouse_wheel(delta, asset, &mut self.context) {
                consumed = true;
            }
        }*/
        consumed
    }

    fn get_target_fps(&self) -> usize {
        self.context.target_fps
    }

    /// Loads the project from the given path
    fn load_project(&mut self, path: std::path::PathBuf, asset: &mut Asset) {
        asset.load_from_path(path.clone());
        self.context.data = server::gamedata::GameData::load_from_path(path.clone());

        let left_width = 180_usize;
        let width = self.rect.2;
        let height = self.rect.3;
        let context = &mut self.context;

        // Calculate an overview node position based on it's index
        let get_pos = |index: usize, max_width: usize| -> (isize, isize) {
            let item_width = (250 + 20) as isize;
            let item_height = (120 + 20) as isize;
            let per_row = max_width as isize % item_width;
            (20 + (index as isize % per_row) * item_width, 20 + (index as isize / per_row) * item_height)
        };

        // Tile views and nodes

        let tilemap_options = TileMapOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);
        let tilemap = TileMapWidget::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), BehaviorType::Tiles, asset, &context);

        let mut tile_nodes = vec![];
        for (index, t) in asset.tileset.maps_names.iter().enumerate() {
            let p = get_pos(index, width - left_width);
            let mut node = NodeWidget::new(vec![t.to_string()], NodeUserData { position: p });

            let mut size_text = "".to_string();
            if let Some(tilemap) = asset.tileset.maps.get(&asset.tileset.maps_ids[index]) {
                size_text = format!("{}", tilemap.settings.grid_size);
            }

            let mut size_atom = AtomWidget::new(vec!["Grid Size".to_string()], AtomWidgetType::NodeGridSizeButton,
            AtomData::new_as_int("grid_size".to_string(), 0));
            size_atom.atom_data.text = "Grid Size".to_string();
            size_atom.atom_data.data = (index as f64, 0.0, 0.0, 0.0, size_text);
            size_atom.behavior_id = Some((index, 0, "".to_string()));
            //size_atom.atom_data.data = context.data.get_behavior_id_value(id, (0.0,0.0,0.0,0.0, "Hello".to_string()), self.graph_type);
            node.widgets.push(size_atom);
            tile_nodes.push(node);
        }

        let mut node_graph_tiles = NodeGraph::new(vec!(), (0, context.toolbar_height, width, height - context.toolbar_height), BehaviorType::Tiles, asset, &context);
        node_graph_tiles.set_mode_and_nodes(GraphMode::Overview, tile_nodes, &context);

        self.content.push( (None, Some(Box::new(node_graph_tiles))) );
        self.content.push( (Some(Box::new(tilemap_options)), Some(Box::new(tilemap))) );

        // Region views and nodes

        let region_options = RegionOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);

        let region_overview_options = RegionOverviewOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);

        let region_widget = RegionWidget::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), BehaviorType::Tiles, asset, &context);

        let mut region_nodes = vec![];
        for (index, t) in context.data.regions_names.iter().enumerate() {
            let p = get_pos(index, width - left_width);
            let node = NodeWidget::new(vec![t.to_string()], NodeUserData { position: p});
            region_nodes.push(node);
        }

        let mut node_graph_regions = NodeGraph::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), BehaviorType::Regions, asset, &context );
        node_graph_regions.set_mode_and_nodes(GraphMode::Overview, region_nodes, &context);

        self.content.push( (Some(Box::new(region_overview_options)), Some(Box::new(node_graph_regions))) );
        self.content.push( (Some(Box::new(region_options)), Some(Box::new(region_widget))) );

        // Behavior nodegraph

        let behavior_options = BehaviorOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);

        let behavior_overview_options = BehaviorOverviewOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);

        let mut behavior_nodes = vec![];
        for (index, behavior_name) in context.data.behaviors_names.iter().enumerate() {
            let p = get_pos(index, width - left_width);
            let mut node = NodeWidget::new(vec![behavior_name.to_string()],
             NodeUserData { position: p });

            let node_menu_atom = crate::atom::AtomWidget::new(vec!["Rename".to_string(), "Delete".to_string()], crate::atom::AtomWidgetType::NodeMenu, crate::atom::AtomData::new_as_int("menu".to_string(), 0));
            node.menu = Some(node_menu_atom);

            behavior_nodes.push(node);
        }
        let mut node_graph_behavior = NodeGraph::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), BehaviorType::Behaviors, asset, &context);
        node_graph_behavior.set_mode_and_nodes(GraphMode::Overview, behavior_nodes, &context);

        let mut node_graph_behavior_details = NodeGraph::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), BehaviorType::Behaviors, asset, &context);
        node_graph_behavior_details.set_mode(GraphMode::Detail, &context);

        self.content.push( (Some(Box::new(behavior_overview_options)), Some(Box::new(node_graph_behavior))) );
        self.content.push( (Some(Box::new(behavior_options)), Some(Box::new(node_graph_behavior_details))) );

        // Systems nodegraph

        let systems_options = SystemsOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);

        let systems_overview_options = SystemsOverviewOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);

        let mut systems_nodes = vec![];
        for (index, system_name) in context.data.systems_names.iter().enumerate() {
            let p = get_pos(index, width - left_width);
            let mut node = NodeWidget::new(vec![system_name.to_string()],
             NodeUserData { position: p });

            let node_menu_atom = crate::atom::AtomWidget::new(vec!["Rename".to_string(), "Delete".to_string()], crate::atom::AtomWidgetType::NodeMenu, crate::atom::AtomData::new_as_int("menu".to_string(), 0));
            node.menu = Some(node_menu_atom);

            systems_nodes.push(node);
        }
        let mut node_graph_systems = NodeGraph::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), BehaviorType::Systems, asset, &context);
        node_graph_systems.set_mode_and_nodes(GraphMode::Overview, systems_nodes, &context);

        let mut node_graph_systems_details = NodeGraph::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), BehaviorType::Systems, asset, &context);
        node_graph_systems_details.set_mode(GraphMode::Overview, &context);

        self.content.push( (Some(Box::new(systems_overview_options)), Some(Box::new(node_graph_systems))) );
        self.content.push( (Some(Box::new(systems_options)), Some(Box::new(node_graph_systems_details))) );

        // Items nodegraph

        let items_options = ItemsOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);

        let items_overview_options = ItemsOverviewOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);

        let items_nodes = vec![];

        let mut node_graph_items = NodeGraph::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), BehaviorType::Items, asset, &context);
        node_graph_items.set_mode_and_nodes(GraphMode::Overview, items_nodes, &context);

        let mut node_graph_items_details = NodeGraph::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), BehaviorType::Items, asset, &context);
        node_graph_items_details.set_mode(GraphMode::Detail, &context);

        self.content.push( (Some(Box::new(items_overview_options)), Some(Box::new(node_graph_items))) );
        self.content.push( (Some(Box::new(items_options)), Some(Box::new(node_graph_items_details))) );

        // Game NodeGraph

        let game_options = GameOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);

        let mut node_graph_game_details = NodeGraph::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), BehaviorType::GameLogic,  asset, &context);
        node_graph_game_details.set_mode(GraphMode::Detail, &context);

        self.content.push( (Some(Box::new(game_options)), Some(Box::new(node_graph_game_details))) );

        //

        self.state = EditorState::TilesOverview;
        self.toolbar.widgets[0].text = asset.tileset.maps_names.clone();
    }
}