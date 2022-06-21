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
use core_render::render::GameRender;
use core_shared::asset::Asset;
use core_server::gamedata::behavior::BehaviorType;
use core_shared::actions::*;
use core_shared::update::GameUpdate;

use crate::editor::dialog::DialogWidget;

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
pub mod dialog_position;
mod log;
mod gameoptions;
pub mod traits;
pub mod codeeditorwidget;
pub mod screeneditor;
pub mod screeneditor_options;

use crate::editor::toolbar::ToolBar;
use crate::editor::controlbar::ControlBar;
use tilemapwidget::TileMapWidget;

use crate::context::ScreenContext;
use crate::editor::node::{ NodeUserData, NodeWidget };
use crate::editor::nodegraph::NodeGraph;

use self::codeeditorwidget::CodeEditorWidget;
use self::dialog::{DialogState, DialogEntry};
use self::dialog_position::DialogPositionWidget;
use self::screeneditor_options::ScreenEditorOptions;
use self::tilemapoptions::TileMapOptions;
use self::statusbar::StatusBar;

use crate::editor::traits::{ EditorContent, GraphMode, EditorOptions };

#[derive (PartialEq, Copy, Clone, Debug)]
pub enum EditorState {
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
    GameDetail,
    ScreenDetail,
}

/// The Editor struct
pub struct Editor<'a> {
    rect                            : (usize, usize, usize, usize),
    state                           : EditorState,
    context                         : ScreenContext<'a>,
    controlbar                      : ControlBar,
    toolbar                         : ToolBar,
    log                             : LogWidget,
    code_editor                     : CodeEditorWidget,

    pub content                      : Vec<(Option<Box<dyn EditorOptions>>, Option<Box<dyn EditorContent>>)>,

    log_drag_start_pos              : Option<(usize, usize)>,
    log_drag_start_rect             : (isize, isize),

    left_width                      : usize,
    mouse_pos                       : (usize, usize),
    mouse_hover_pos                 : (usize, usize),

    dialog                          : DialogWidget,
    dialog_position                 : DialogPositionWidget,

    status_bar                      : StatusBar,

    game_render                     : Option<GameRender<'a>>,

    project_to_load                 : Option<std::path::PathBuf>
}

impl ScreenWidget for Editor<'_> {

    fn new(asset: &mut Asset, width: usize, height: usize) -> Self where Self: Sized {

        asset.load_editor_font("OpenSans".to_string(), "Open_Sans/static/OpenSans/OpenSans-Regular.ttf".to_string());
        asset.load_editor_font("OpenSans_Light".to_string(), "Open_Sans/static/OpenSans/OpenSans-Light.ttf".to_string());
        asset.load_editor_font("SourceCodePro".to_string(), "Source_Code_Pro/static/SourceCodePro-Regular.ttf".to_string());

        let left_width = 180_usize;
        let mut context = ScreenContext::new(width, height);

        let controlbar = ControlBar::new(vec!(), (0,0, width, context.toolbar_height / 2), asset, &mut context);
        let toolbar = ToolBar::new(vec!(), (0, context.toolbar_height / 2, width, context.toolbar_height / 2), asset, &mut context);

        //

        let dialog = DialogWidget::new(asset, &context);
        let dialog_position = DialogPositionWidget::new(asset, &context);

        let log = LogWidget::new(&context);
        let mut status_bar = StatusBar::new();

        let code_editor =  CodeEditorWidget::new(vec!(), (0, context.toolbar_height, width, height - context.toolbar_height), asset, &context);

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
            // project_to_load = context.get_project_path(project_list[0].clone());
            project_to_load = Some(std::path::PathBuf::new()); // Load the local project for development

            status_bar.add_message(format!("Loaded Documents >> Eldiron >> {}", project_list[0]));
        }

        Self {
            rect                    : (0, 0, width, height),
            state                   :  EditorState::TilesOverview,
            context,
            controlbar,
            toolbar,
            log,
            code_editor,

            content                 : vec![],

            log_drag_start_pos      : None,
            log_drag_start_rect     : (0, 0),

            dialog,
            dialog_position,

            left_width,
            mouse_pos               : (0,0),
            mouse_hover_pos         : (0,0),

            status_bar,

            game_render             : None,

            project_to_load,
        }
    }

    /// Game tick if the game is running
    fn update(&mut self) {
        // let start = self.get_time();
        if self.context.is_debugging == true {
            self.context.data.tick();
            self.content[self.state as usize].1.as_mut().unwrap().update(&mut self.context);
        } else
        if self.context.is_running {
            self.context.data.tick();
        }
        // let stop = self.get_time();
        // println!("update time {:?}", stop - start);

        // When stopped, update the graph
        if self.context.just_stopped_running {
            self.content[self.state as usize].1.as_mut().unwrap().update(&mut self.context);
            self.context.just_stopped_running = false;
        }
    }

    /// A key was pressed
    fn key_down(&mut self, char: Option<char>, key: Option<WidgetKey>, asset: &mut Asset) -> bool {

        if self.context.is_running {

            if self.context.is_debugging {
                if key == Some(WidgetKey::Escape) {
                    self.controlbar.stop_debugging(&mut self.context);
                }
            }

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
        } else
        if self.context.dialog_position_state == DialogState::Open {
            return self.dialog_position.key_down(char, key, asset, &mut self.context);
        } else
        if self.context.code_editor_is_active {
            let mut consumed = false;
            if self.state == EditorState::ScreenDetail && key == Some(WidgetKey::Escape) {
                self.content_state_is_changing(self.state, asset, true);
                self.state = EditorState::GameDetail;
                consumed = true;
            }
            return self.code_editor.key_down(char, key, asset, &mut self.context) || consumed;
        } else
        if self.state == EditorState::ScreenDetail && key == Some(WidgetKey::Escape) {
            self.content_state_is_changing(self.state, asset, true);
            self.state = EditorState::GameDetail;
            return true;
        }
        false
    }

    // Resize the editor
    fn resize(&mut self, width: usize, height: usize) {
        self.context.width = width; self.rect.2 = width;
        self.context.height = height; self.rect.3 = height;
        self.controlbar.resize(width, height, &self.context);
        self.toolbar.resize(width, height, &self.context);

        for index in 0..self.content.len() {
            if self.content[index].0.is_some() {
                self.content[index].0.as_mut().unwrap().resize(self.left_width, height - self.context.toolbar_height, &self.context);
                self.content[index].1.as_mut().unwrap().resize(width - self.left_width, height - self.context.toolbar_height, &self.context);
            } else {
                self.content[index].1.as_mut().unwrap().resize(width, height - self.context.toolbar_height, &self.context);
            }
        }
    }

    /// Draw the editor
    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset) {

        // let start = self.get_time();

        // Playback
        if self.context.is_running && self.context.is_debugging == false {

            self.controlbar.draw(frame, anim_counter, asset, &mut self.context);

            // Clear the game area with color_black
            let clear_frame = (0, self.context.toolbar_height / 2, self.context.width, self.context.height - self.context.toolbar_height / 2);
            self.context.draw2d.draw_rect(frame, &clear_frame, self.context.width, &self.context.color_black);

            if self.game_render.is_none() {
                self.game_render = Some(GameRender::new(self.context.curr_project_path.clone()));
            }

            if let Some(render) = &mut self.game_render {
                if let Some(update_string) = self.context.data.poll_update(131313) {

                    let update = serde_json::from_str::<GameUpdate>(&update_string).ok();

                    if let Some(update) = update {
                        render.draw(anim_counter, &update);
                    }

                    let mut cx : usize = 0;
                    let mut cy : usize = 0;

                    if render.width < clear_frame.2 {
                        cx = (clear_frame.2 - render.width) / 2;
                    }

                    if render.height < clear_frame.3 {
                        cy = (clear_frame.3 - render.height) / 2;
                    }

                    let dest_frame = (cx, cy + self.context.toolbar_height / 2, render.width, render.height);
                    self.context.draw2d.copy_slice(frame, &mut render.frame, &dest_frame, self.context.width);
                }
            }
            return;
        }

        // To update the variables
        if self.context.just_stopped_running {
            self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().set_dirty();

            if let Some(preview) = self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().get_preview_widget() {
                preview.dirty = true;
            }
        }

        // Do we need to load a new project ?
        if self.project_to_load.is_some() {
            self.load_project(self.project_to_load.clone().unwrap(), asset);
            self.project_to_load = None;
        }

        // Do we need to switch to another state ?
        if let Some(state) = self.context.switch_editor_state {

            if state != self.state {

                self.content_state_is_changing(state, asset, false);
                self.content_state_is_changing(self.state, asset, true);
            }

            self.state = state;
            self.context.switch_editor_state = None;

            if state == EditorState::TilesDetail {
                self.context.curr_graph_type = BehaviorType::Tiles;
                self.content[EditorState::TilesDetail as usize].1.as_mut().unwrap().set_tilemap_id(asset.tileset.maps_ids[self.context.curr_tileset_index]);
            } else
            if state == EditorState::RegionDetail {
                self.context.curr_graph_type = BehaviorType::Regions;

                let index = EditorState::RegionDetail as usize;
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
            if state == EditorState::BehaviorDetail {
                self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().set_mode_and_rect( GraphMode::Detail, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);

                self.context.curr_graph_type = BehaviorType::Behaviors;
                self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().set_behavior_id(self.context.data.behaviors_ids[self.context.curr_behavior_index] , &mut self.context);
            } else
            if state == EditorState::SystemsDetail {
                self.content[EditorState::SystemsDetail as usize].1.as_mut().unwrap().set_mode_and_rect( GraphMode::Detail, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);

                self.context.curr_graph_type = BehaviorType::Systems;
                self.content[EditorState::SystemsDetail as usize].1.as_mut().unwrap().set_behavior_id(self.context.data.systems_ids[self.context.curr_systems_index] , &mut self.context);
            } else
            if state == EditorState::ItemsDetail {
            }
        }

        self.controlbar.draw(frame, anim_counter, asset, &mut self.context);

        if self.content.is_empty() == false {
            self.toolbar.draw(frame, anim_counter, asset, &mut self.context);
            let index = self.state as usize;
            let mut options : Option<Box<dyn EditorOptions>> = None;
            let mut content : Option<Box<dyn EditorContent>> = None;

            if let Some(element) = self.content.drain(index..index+1).next() {
                options = element.0;
                content = element.1;

                if let Some(mut el_option) = options {
                    el_option.draw(frame, anim_counter, asset, &mut self.context, &mut content);
                    options = Some(el_option);
                    self.status_bar.rect.0 = self.left_width;
                } else {
                    self.status_bar.rect.0 = 0;
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

        // Content: Code Editor ?
        if self.context.code_editor_is_active {

            // Do we need to update the node from the code editor ?
            if self.context.code_editor_update_node {
                self.context.code_editor_node_behavior_value.4 = self.context.code_editor_value.clone();
                self.context.dialog_node_behavior_value = self.context.code_editor_node_behavior_value.clone();
                self.context.dialog_node_behavior_id = self.context.code_editor_node_behavior_id.clone();
                if self.state == EditorState::ScreenDetail {
                    self.content[EditorState::GameDetail as usize].1.as_mut().unwrap().update_from_dialog(&mut self.context);
                } else {
                    self.content[self.state as usize].1.as_mut().unwrap().update_from_dialog(&mut self.context);
                }
                self.context.data.set_behavior_id_value(self.context.code_editor_node_behavior_id.clone(), self.context.code_editor_node_behavior_value.clone(), self.context.curr_graph_type);

                self.context.code_editor_update_node = false;
            }

            if self.context.code_editor_just_opened {
                self.code_editor.set_text_mode(self.context.code_editor_text_mode);
                self.code_editor.set_code(self.context.code_editor_node_behavior_value.4.clone());
                self.context.code_editor_just_opened = false;
            }

            self.code_editor.draw(frame, (self.left_width, self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), anim_counter, asset, &mut self.context);
        }

        // Status bar
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

                        // Update the node and its widget with the new value
                        self.content[EditorState::TilesOverview as usize].1.as_mut().unwrap().get_nodes().unwrap()[index].widgets[0].atom_data.data.4 = self.context.dialog_node_behavior_value.4.clone();
                        self.content[EditorState::TilesOverview as usize].1.as_mut().unwrap().get_nodes().unwrap()[index].widgets[0].dirty = true;
                        self.content[EditorState::TilesOverview as usize].1.as_mut().unwrap().get_nodes().unwrap()[index].dirty = true;
                        self.content[EditorState::TilesOverview as usize].1.as_mut().unwrap().set_dirty();
                    }
                }
            } else
            if self.state == EditorState::RegionOverview {
                if self.context.dialog_entry == DialogEntry::NewName && self.context.dialog_accepted == true {
                    //println!("dialog ended {} {}", self.context.dialog_new_name, self.context.dialog_new_name_type);

                    if self.context.data.create_region(self.context.dialog_new_name.clone()) {
                        let mut node = NodeWidget::new(vec![self.context.dialog_new_name.clone()],
                        NodeUserData { position: (100, 50 + 150 * self.content[EditorState::RegionOverview as usize].1.as_mut().unwrap().get_nodes().unwrap().len() as isize) });

                        let node_menu_atom = crate::atom::AtomWidget::new(vec!["Rename".to_string(), "Delete".to_string()], crate::atom::AtomWidgetType::NodeMenu, crate::atom::AtomData::new_as_int("menu".to_string(), 0));
                        node.menu = Some(node_menu_atom);

                        self.content[EditorState::RegionOverview as usize].1.as_mut().unwrap().get_nodes().unwrap().push(node);
                        self.content[EditorState::RegionOverview as usize].1.as_mut().unwrap().set_dirty();
                        self.toolbar.widgets[0].text = self.context.data.regions_names.clone();
                        self.toolbar.widgets[0].dirty = true;
                    }
                } else {
                    if self.context.dialog_entry == DialogEntry::NodeName {
                        if self.context.dialog_accepted == true {
                            if let Some(region) = self.context.data.regions.get_mut(&self.context.data.regions_ids[self.context.curr_region_index]) {
                                self.content[EditorState::RegionOverview as usize].1.as_mut().unwrap().get_nodes().unwrap()[self.context.curr_region_index].text[0] = self.context.dialog_node_behavior_value.4.clone();
                                self.content[EditorState::RegionOverview as usize].1.as_mut().unwrap().get_nodes().unwrap()[self.context.curr_region_index].dirty = true;
                                self.content[EditorState::RegionOverview as usize].1.as_mut().unwrap().set_dirty();
                                region.rename(self.context.dialog_node_behavior_value.4.clone());
                                self.context.data.regions_names[self.context.curr_region_index] = self.context.dialog_node_behavior_value.4.clone();
                                self.toolbar.widgets[0].text = self.context.data.regions_names.clone();
                                self.toolbar.widgets[0].dirty = true;
                            }
                        }
                    } else {
                        self.content[EditorState::RegionOverview as usize].1.as_mut().unwrap().update_from_dialog(&mut self.context);
                    }
                }
            } else
            if self.state == EditorState::RegionDetail && self.context.dialog_entry == DialogEntry::NewName && self.context.dialog_accepted == true {
                let index = EditorState::RegionDetail as usize;
                let mut options : Option<Box<dyn EditorOptions>> = None;
                let mut content : Option<Box<dyn EditorContent>> = None;

                if let Some(element) = self.content.drain(index..index+1).next() {
                    options = element.0;
                    content = element.1;

                    if let Some(mut el_option) = options {
                        el_option.set_area_name(self.context.dialog_new_name.clone(), &mut self.context, &mut content);
                        options = Some(el_option);
                    }
                }
                self.content.insert(index, (options, content));
            } else
            if self.state == EditorState::ScreenDetail && self.context.dialog_entry == DialogEntry::NewName && self.context.dialog_accepted == true {
                let index = EditorState::ScreenDetail as usize;
                let mut options : Option<Box<dyn EditorOptions>> = None;
                let mut content : Option<Box<dyn EditorContent>> = None;

                if let Some(element) = self.content.drain(index..index+1).next() {
                    options = element.0;
                    content = element.1;

                    if let Some(mut el_option) = options {
                        el_option.set_widget_name(self.context.dialog_new_name.clone(), &mut self.context, &mut content);
                        options = Some(el_option);
                    }
                }
                self.content.insert(index, (options, content));
            } else
            if self.state == EditorState::TilesDetail && self.context.dialog_entry == DialogEntry::Tags && self.context.dialog_accepted == true {
                let index = EditorState::TilesDetail as usize;
                let mut options : Option<Box<dyn EditorOptions>> = None;
                let mut content : Option<Box<dyn EditorContent>> = None;

                if let Some(element) = self.content.drain(index..index+1).next() {
                    options = element.0;
                    content = element.1;

                    if let Some(mut el_option) = options {
                        el_option.set_tags(self.context.dialog_new_name.clone(), asset, &self.context);
                        options = Some(el_option);
                    }
                }
                self.content.insert(index, (options, content));
            } else
            if self.state == EditorState::RegionDetail && self.context.dialog_entry == DialogEntry::Tags && self.context.dialog_accepted == true {
                let index = EditorState::RegionDetail as usize;
                let mut options : Option<Box<dyn EditorOptions>> = None;
                let mut content : Option<Box<dyn EditorContent>> = None;

                if let Some(element) = self.content.drain(index..index+1).next() {
                    options = element.0;
                    content = element.1;

                    if let Some(mut el_option) = options {
                        el_option.set_tags(self.context.dialog_new_name.clone(), asset, &self.context);//, &mut content);
                        options = Some(el_option);
                    }
                }
                self.content.insert(index, (options, content));
            } else
            if self.state == EditorState::RegionDetail {
                self.content[EditorState::RegionDetail as usize].1.as_mut().unwrap().update_from_dialog(&mut self.context);
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
                                self.content[EditorState::BehaviorOverview as usize].1.as_mut().unwrap().get_nodes().unwrap()[self.context.curr_behavior_index].text[0] = self.context.dialog_node_behavior_value.4.clone();
                                self.content[EditorState::BehaviorOverview as usize].1.as_mut().unwrap().get_nodes().unwrap()[self.context.curr_behavior_index].dirty = true;
                                self.content[EditorState::BehaviorOverview as usize].1.as_mut().unwrap().set_dirty();
                                behavior.rename(self.context.dialog_node_behavior_value.4.clone());
                                self.context.data.behaviors_names[self.context.curr_behavior_index] = self.context.dialog_node_behavior_value.4.clone();
                                self.toolbar.widgets[0].text = self.context.data.behaviors_names.clone();
                                self.toolbar.widgets[0].dirty = true;
                            }
                        }
                    } else {
                        self.content[EditorState::BehaviorOverview as usize].1.as_mut().unwrap().update_from_dialog(&mut self.context);
                    }
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
                                self.content[EditorState::SystemsOverview as usize].1.as_mut().unwrap().get_nodes().unwrap()[self.context.curr_systems_index].text[0] = self.context.dialog_node_behavior_value.4.clone();
                                self.content[EditorState::SystemsOverview as usize].1.as_mut().unwrap().get_nodes().unwrap()[self.context.curr_systems_index].dirty = true;
                                self.content[EditorState::SystemsOverview as usize].1.as_mut().unwrap().set_dirty();
                                system.rename(self.context.dialog_node_behavior_value.4.clone());
                                self.context.data.systems_names[self.context.curr_systems_index] = self.context.dialog_node_behavior_value.4.clone();
                                self.toolbar.widgets[0].text = self.context.data.systems_names.clone();
                                self.toolbar.widgets[0].dirty = true;
                            }
                        }
                    }
                    self.content[EditorState::SystemsOverview as usize].1.as_mut().unwrap().update_from_dialog(&mut self.context);
                }
            } else
            if self.state == EditorState::GameDetail {
                self.content[EditorState::GameDetail as usize].1.as_mut().unwrap().update_from_dialog(&mut self.context);
            }
            self.context.dialog_entry = DialogEntry::None;
        }

        // Dialog Position
        if self.context.dialog_position_state != DialogState::Closed {
            self.dialog_position.rect.0 = (self.context.width - self.dialog.rect.2) / 2;
            self.dialog_position.draw(frame, anim_counter, asset, &mut self.context);
        } else
        if self.dialog_position.new_value {
            if self.state == EditorState::BehaviorDetail {
                self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().update_from_dialog(&mut self.context);
                self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().set_dirty();
                if let Some(preview) = self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().get_preview_widget() {
                    preview.dirty = true;
                }
            } else
            if self.state == EditorState::RegionDetail {
                self.content[EditorState::RegionDetail as usize].1.as_mut().unwrap().update_from_dialog(&mut self.context);
                self.content[EditorState::RegionDetail as usize].1.as_mut().unwrap().set_dirty();
                if let Some(preview) = self.content[EditorState::RegionDetail as usize].1.as_mut().unwrap().get_preview_widget() {
                    preview.dirty = true;
                }
            }
            self.dialog_position.new_value = false;
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
        if self.context.dialog_position_state == DialogState::Open {
            return self.dialog_position.mouse_down(pos, asset, &mut self.context);
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
                } else
                if self.toolbar.widgets[EditorState::TilesDetail as usize].right_selected && asset.tileset.maps_ids.is_empty() == false {
                    self.state = EditorState::TilesDetail;
                    self.context.curr_graph_type = BehaviorType::Tiles;

                    self.content[EditorState::TilesDetail as usize].1.as_mut().unwrap().set_tilemap_id(asset.tileset.maps_ids[self.context.curr_tileset_index]);
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
                } else
                if self.toolbar.widgets[3].right_selected {
                    self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().set_mode_and_rect( GraphMode::Detail, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::BehaviorDetail;
                    self.context.curr_graph_type = BehaviorType::Behaviors;
                    self.content[EditorState::BehaviorDetail as usize].1.as_mut().unwrap().set_behavior_id(self.context.data.behaviors_ids[self.context.curr_behavior_index] , &mut self.context);
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
                } else
                if self.toolbar.widgets[4].right_selected {
                    self.content[EditorState::SystemsDetail as usize].1.as_mut().unwrap().set_mode_and_rect( GraphMode::Detail, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::SystemsDetail;
                    self.context.curr_graph_type = BehaviorType::Systems;
                    self.content[EditorState::SystemsDetail as usize].1.as_mut().unwrap().set_behavior_id(self.context.data.systems_ids[self.context.curr_systems_index] , &mut self.context);
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
                } else
                if self.toolbar.widgets[5].right_selected {
                    self.content[EditorState::ItemsDetail as usize].1.as_mut().unwrap().set_mode_and_rect( GraphMode::Detail, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::ItemsDetail;
                    self.context.curr_graph_type = BehaviorType::Items;
                    //self.node_graph_items_details.set_behavior_id(self.context.data.items_ids[self.context.curr_items_index], &mut self.context);
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
            }
            consumed = true;
        }

        if self.context.code_editor_is_active && self.context.contains_pos_for(pos, self.code_editor.rect) {
            consumed = self.code_editor.mouse_down(pos, asset, &mut self.context);
        } else {
            let index = self.state as usize;
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
        }

        consumed
    }

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {

        if self.context.dialog_state == DialogState::Open {
            return self.dialog.mouse_up(pos, asset, &mut self.context);
        }
        if self.context.dialog_position_state == DialogState::Open {
            return self.dialog_position.mouse_up(pos, asset, &mut self.context);
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

                    let index = EditorState::RegionDetail as usize;
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

        if self.context.code_editor_is_active && self.context.contains_pos_for(pos, self.code_editor.rect) {
            self.code_editor.mouse_up(pos, asset, &mut self.context);
        } else {
            let index = self.state as usize;
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
        }

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
                let rect = self.content[EditorState::RegionDetail as usize].1.as_mut().unwrap().get_rect();
                let offset = self.content[EditorState::RegionDetail as usize].1.as_mut().unwrap().get_offset();

                        println!("add_node_of_name {:?} {:?}", rect, pos);

                if self.context.contains_pos_for(pos, rect) {
                    let mut position = (pos.0 as isize, pos.1 as isize);
                    position.0 -= rect.0 as isize + offset.0 + drag_context.offset.0;
                    position.1 -= rect.1 as isize + offset.1 + drag_context.offset.1;

                     self.content[EditorState::RegionDetail as usize].1.as_mut().unwrap().add_node_of_name(drag_context.text.clone(), position, &mut self.context);
                }
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
        if self.context.dialog_position_state == DialogState::Open {
            return self.dialog_position.mouse_dragged(pos, asset, &mut self.context);
        }

        if let Some(log_drag_start_pos) = self.log_drag_start_pos {
            self.log.rect.0 = self.log_drag_start_rect.0 - (log_drag_start_pos.0 as isize - pos.0 as isize);
            self.log.rect.1 = self.log_drag_start_rect.1 - (log_drag_start_pos.1 as isize - pos.1 as isize);
            return true;
        }

        let mut consumed = false;
        self.toolbar.mouse_dragged(pos, asset, &mut self.context);

        if self.context.code_editor_is_active && self.context.contains_pos_for(pos, self.code_editor.rect) {
            consumed = self.code_editor.mouse_dragged(pos, asset, &mut self.context);
        } else {
            let index = self.state as usize;
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
        }

        self.mouse_pos = pos.clone();
        consumed
    }

    fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {

        if self.context.dialog_state == DialogState::Open {
            return self.dialog.mouse_hover(pos, asset, &mut self.context);
        }
        if self.context.dialog_position_state == DialogState::Open {
            return self.dialog_position.mouse_hover(pos, asset, &mut self.context);
        }

        let mut consumed = false;

        if consumed == false && self.toolbar.mouse_hover(pos, asset, &mut self.context) {
            consumed = true;
        } else {

            self.mouse_hover_pos = pos.clone();

            let index = self.state as usize;
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

        consumed
    }

    fn mouse_wheel(&mut self, delta: (isize, isize), asset: &mut Asset) -> bool {

        if self.context.dialog_state == DialogState::Open {
            return self.dialog.mouse_wheel(delta, asset, &mut self.context);
        }
        if self.context.dialog_position_state == DialogState::Open {
            return self.dialog_position.mouse_wheel(delta, asset, &mut self.context);
        }

        let mut consumed = false;

        if self.context.code_editor_is_active && self.context.contains_pos_for(self.mouse_hover_pos, self.code_editor.rect) {
            self.code_editor.mouse_wheel(delta, asset, &mut self.context);
        } else {
            let index = self.state as usize;
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
        }

        consumed
    }

    fn modifier_changed(&mut self, shift: bool, ctrl: bool, alt: bool, logo: bool, asset: &mut Asset) -> bool {

        let mut consumed = false;

        if self.context.code_editor_is_active {
            consumed = self.code_editor.modifier_changed(shift, ctrl, alt, logo, asset, &mut self.context);
        }

        consumed
    }

    fn get_target_fps(&self) -> usize {
        self.context.target_fps
    }

    /// Send opening / closing messages to the given state
    fn content_state_is_changing(&mut self, state: EditorState, asset: &mut Asset, closing: bool) {
        let index = state as usize;
        let mut options : Option<Box<dyn EditorOptions>> = None;
        let mut content : Option<Box<dyn EditorContent>> = None;

        if let Some(element) = self.content.drain(index..index+1).next() {
            options = element.0;
            content = element.1;

            if let Some(mut el_content) = content {

                if closing == false {
                    el_content.opening(asset, &mut self.context, &mut options);
                } else {
                    el_content.closing(asset, &mut self.context, &mut options);
                }
                content = Some(el_content);
            }

            if let Some(mut el_options) = options {

                if closing == false {
                    el_options.opening(asset, &mut self.context, &mut content);
                } else {
                    el_options.closing(asset, &mut self.context, &mut content);
                }
                options = Some(el_options);
            }
        }
        self.content.insert(index, (options, content));

        // if closing && state == EditorState::ScreenDetail {
        //     self.content[EditorState::GameDetail as usize].1.as_mut().unwrap().update_from_dialog(&mut self.context);
        // }
    }

    /// Loads the project from the given path
    fn load_project(&mut self, path: std::path::PathBuf, asset: &mut Asset) {
        asset.load_from_path(path.clone());
        self.context.data = core_server::gamedata::GameData::load_from_path(path.clone());

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
            let mut node = NodeWidget::new(vec![t.to_string()], NodeUserData { position: p});

            let node_menu_atom = crate::atom::AtomWidget::new(vec!["Rename".to_string(), "Delete".to_string()], crate::atom::AtomWidgetType::NodeMenu, crate::atom::AtomData::new_as_int("menu".to_string(), 0));
            node.menu = Some(node_menu_atom);

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

        // Screen Editor

        let screen_editor_options = ScreenEditorOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);

        let screen_editor = screeneditor::ScreenEditor::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), BehaviorType::Tiles, asset, &context);

        self.content.push( (Some(Box::new(screen_editor_options)), Some(Box::new(screen_editor))) );

        //

        self.state = EditorState::TilesOverview;
        self.toolbar.widgets[0].text = asset.tileset.maps_names.clone();
        self.controlbar.widgets[2].state = WidgetState::Normal;
        self.controlbar.widgets[2].dirty = true;
    }
}