
use crate::tileselector::TileSelectorWidget;
use crate::editor::regionoptions::RegionOptions;
use crate::editor::behavioroptions::BehaviorOptions;
use crate::editor::behavior_overview_options::BehaviorOverviewOptions;
use crate::editor::systemsoptions::SystemsOptions;
use crate::editor::systems_overview_options::SystemsOverviewOptions;
use crate::editor::regionwidget::RegionWidget;
use crate::editor::log::LogWidget;
use crate::widget:: {ScreenWidget, Widget, WidgetState, WidgetKey};

use server::gamedata::behavior::{ BehaviorType };

use crate::editor::dialog::DialogWidget;

use server::asset::Asset;

mod toolbar;
mod nodegraph;
mod tilemapoptions;
mod tilemapwidget;
mod regionwidget;
mod regionoptions;
mod behavioroptions;
mod behavior_overview_options;
mod systemsoptions;
mod systems_overview_options;
mod node;
mod node_preview;
mod statusbar;
pub mod dialog;
mod log;

use crate::editor::toolbar::ToolBar;
use tilemapwidget::TileMapWidget;

use crate::context::ScreenContext;
use crate::editor::node::NodeUserData;
use crate::editor::node::NodeWidget;
use crate::editor::nodegraph::{ NodeGraph, GraphMode };

use self::dialog::{DialogState, DialogEntry};
use self::tilemapoptions::TileMapOptions;
use self::statusbar::StatusBar;

#[derive (PartialEq)]
enum EditorState {
    TilesOverview,
    TilesDetail,
    RegionOverview,
    RegionDetail,
    BehaviorOverview,
    BehaviorDetail,
    SystemsOverview,
    SystemsDetail
}

/// The Editor struct
pub struct Editor<'a> {
    rect                            : (usize, usize, usize, usize),
    state                           : EditorState,
    context                         : ScreenContext<'a>,
    toolbar                         : ToolBar,
    log                             : LogWidget,

    tilemap_options                 : TileMapOptions,
    tilemap                         : TileMapWidget,

    region_options                  : RegionOptions,
    region_widget                   : RegionWidget,

    behavior_options                : BehaviorOptions,
    behavior_overview_options       : BehaviorOverviewOptions,

    systems_options                 : SystemsOptions,
    systems_overview_options        : SystemsOverviewOptions,

    node_graph_tiles                : NodeGraph,
    node_graph_regions                : NodeGraph,
    node_graph_behavior             : NodeGraph,
    node_graph_behavior_details     : NodeGraph,
    node_graph_systems              : NodeGraph,
    node_graph_systems_details      : NodeGraph,

    log_drag_start_pos              : Option<(usize, usize)>,
    log_drag_start_rect             : (isize, isize),

    left_width                      : usize,
    mouse_pos                       : (usize, usize),
    mouse_hover_pos                 : (usize, usize),

    dialog                          : DialogWidget,

    status_bar                      : StatusBar,
}

impl ScreenWidget for Editor<'_> {

    fn new(asset: &Asset, width: usize, height: usize) -> Self where Self: Sized {

        let left_width = 180_usize;
        let context = ScreenContext::new(width, height);

        let toolbar = ToolBar::new(vec!(), (0,0, width, context.toolbar_height), asset, &context);

        // Tile views and nodes

        let tilemap_options = TileMapOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);
        let tilemap = TileMapWidget::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), asset, &context);

        let mut tile_nodes = vec![];
        for (index, t) in asset.tileset.maps_names.iter().enumerate() {
            let node = NodeWidget::new(vec![t.to_string()], NodeUserData { position: (100, 50 + 150 * index as isize) });
            tile_nodes.push(node);
        }

        let node_graph_tiles = NodeGraph::new(vec!(), (0, context.toolbar_height, width, height - context.toolbar_height), asset, &context, BehaviorType::Tiles, tile_nodes);

        // Region views and nodes

        let region_options = RegionOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);
        let region_widget = RegionWidget::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), asset, &context);

        let mut region_nodes = vec![];
        for (index, t) in context.data.regions_names.iter().enumerate() {
            let node = NodeWidget::new(vec![t.to_string()], NodeUserData { position: (100, 50 + 150 * index as isize)});
            region_nodes.push(node);
        }

        let node_graph_regions = NodeGraph::new(vec!(), (0, context.toolbar_height, width, height - context.toolbar_height), asset, &context, BehaviorType::Regions, region_nodes);

        // Behavior nodegraph

        let behavior_options = BehaviorOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);

        let behavior_overview_options = BehaviorOverviewOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);

        let mut behavior_nodes = vec![];
        for (index, behavior_name) in context.data.behaviors_names.iter().enumerate() {
            let mut node = NodeWidget::new(vec![behavior_name.to_string()],
             NodeUserData { position: (100, 50 + 150 * index as isize) });

            let node_menu_atom = crate::atom::AtomWidget::new(vec!["Rename".to_string(), "Delete".to_string()], crate::atom::AtomWidgetType::NodeMenu, crate::atom::AtomData::new_as_int("menu".to_string(), 0));
            node.menu = Some(node_menu_atom);

            behavior_nodes.push(node);
        }
        let node_graph_behavior = NodeGraph::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), asset, &context, BehaviorType::Behaviors, behavior_nodes);

        let mut node_graph_behavior_details = NodeGraph::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), asset, &context, BehaviorType::Behaviors, vec![]);

        node_graph_behavior_details.set_mode(GraphMode::Detail, &context);

        // Systems nodegraph

        let systems_options = SystemsOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);

        let systems_overview_options = SystemsOverviewOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);

        let mut systems_nodes = vec![];
        for (index, system_name) in context.data.systems_names.iter().enumerate() {
            let mut node = NodeWidget::new(vec![system_name.to_string()],
             NodeUserData { position: (100, 50 + 150 * index as isize) });

            let node_menu_atom = crate::atom::AtomWidget::new(vec!["Rename".to_string(), "Delete".to_string()], crate::atom::AtomWidgetType::NodeMenu, crate::atom::AtomData::new_as_int("menu".to_string(), 0));
            node.menu = Some(node_menu_atom);

            systems_nodes.push(node);
        }
        let node_graph_systems = NodeGraph::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), asset, &context, BehaviorType::Systems, systems_nodes);

        let mut node_graph_systems_details = NodeGraph::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), asset, &context, BehaviorType::Systems, vec![]);

        node_graph_systems_details.set_mode(GraphMode::Detail, &context);

        //

        let dialog = DialogWidget::new(asset, &context);

        let log = LogWidget::new(&context);

        Self {
            rect                    : (0, 0, width, height),
            state                   : EditorState::TilesOverview,
            context,
            toolbar,
            log,

            tilemap_options,
            tilemap,

            region_options,
            region_widget,

            behavior_options,
            behavior_overview_options,

            systems_options,
            systems_overview_options,

            node_graph_tiles,
            node_graph_regions,
            node_graph_behavior,
            node_graph_behavior_details,
            node_graph_systems,
            node_graph_systems_details,

            log_drag_start_pos      : None,
            log_drag_start_rect     : (0, 0),

            dialog,

            left_width,
            mouse_pos               : (0,0),
            mouse_hover_pos         : (0,0),

            status_bar              : StatusBar::new(),
        }
    }

    /// Update the editor
    fn update(&mut self) {
        // let start = self.get_time();
        if self.state == EditorState::BehaviorDetail {
            self.node_graph_behavior_details.update(&mut self.context);
        } else
        if self.state == EditorState::SystemsDetail {
            self.node_graph_systems_details.update(&mut self.context);
        }
        // let stop = self.get_time();
        // println!("update time {:?}", stop - start);
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.context.width = width; self.rect.2 = width;
        self.context.height = height; self.rect.3 = height;
        self.toolbar.resize(width, height, &self.context);

        self.tilemap_options.resize(self.left_width, height - self.context.toolbar_height, &self.context);
        self.tilemap.resize(width - self.left_width, height - self.context.toolbar_height, &self.context);

        self.region_options.resize(self.left_width, height - self.context.toolbar_height, &self.context);
        self.region_widget.resize(width - self.left_width, height - self.context.toolbar_height, &self.context);

        self.behavior_options.resize(self.left_width, height - self.context.toolbar_height, &self.context);
        self.behavior_overview_options.resize(self.left_width, height - self.context.toolbar_height, &self.context);

        self.systems_options.resize(self.left_width, height - self.context.toolbar_height, &self.context);
        self.systems_overview_options.resize(self.left_width, height - self.context.toolbar_height, &self.context);

        self.node_graph_tiles.resize(width, height - self.context.toolbar_height, &self.context);
        self.node_graph_regions.resize(width, height - self.context.toolbar_height, &self.context);
        self.node_graph_behavior.resize(width - self.left_width, height - self.context.toolbar_height, &self.context);
        self.node_graph_behavior_details.resize(width - self.left_width, height - self.context.toolbar_height, &self.context);
        self.node_graph_systems.resize(width - self.left_width, height - self.context.toolbar_height, &self.context);
        self.node_graph_systems_details.resize(width - self.left_width, height - self.context.toolbar_height, &self.context);
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset) {

        //let start = self.get_time();

        self.toolbar.draw(frame, anim_counter, asset, &mut self.context);

        if self.state == EditorState::TilesOverview {
            self.node_graph_tiles.draw(frame, anim_counter, asset, &mut self.context);
        } else
        if self.state == EditorState::TilesDetail {
            self.tilemap_options.draw(frame, anim_counter, asset, &mut self.context);
            self.tilemap.draw(frame, anim_counter, asset, &mut self.context);
        } else
        if self.state == EditorState::RegionOverview {
            self.node_graph_regions.draw(frame, anim_counter, asset, &mut self.context);
        } else
        if self.state == EditorState::RegionDetail {
            self.region_options.draw(frame, anim_counter, asset, &mut self.context, &mut self.region_widget);
            self.region_widget.draw(frame, anim_counter, asset, &mut self.context, &mut self.region_options);
        } else
        if self.state == EditorState::BehaviorOverview {
            self.behavior_overview_options.draw(frame, anim_counter, asset, &mut self.context);
            self.node_graph_behavior.draw(frame, anim_counter, asset, &mut self.context);
        } else
        if self.state == EditorState::BehaviorDetail {
            self.behavior_options.draw(frame, anim_counter, asset, &mut self.context);
            self.node_graph_behavior_details.draw(frame, anim_counter, asset, &mut self.context);
            self.status_bar.draw(frame, anim_counter, asset, &mut self.context);
            self.log.draw(frame, anim_counter, asset, &mut self.context);
            self.context.draw2d.blend_slice_safe(frame, &self.log.buffer[..], &self.log.rect, self.context.width, &self.node_graph_behavior_details.rect);
        } else
        if self.state == EditorState::SystemsOverview {
            self.systems_overview_options.draw(frame, anim_counter, asset, &mut self.context);
            self.node_graph_systems.draw(frame, anim_counter, asset, &mut self.context);
        } else
        if self.state == EditorState::SystemsDetail {
            self.systems_options.draw(frame, anim_counter, asset, &mut self.context);
            self.node_graph_systems_details.draw(frame, anim_counter, asset, &mut self.context);
            //self.status_bar.draw(frame, anim_counter, asset, &mut self.context);
        }

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

            if self.state == EditorState::RegionDetail && self.context.dialog_entry == DialogEntry::NewName && self.context.dialog_accepted == true {
                self.region_options.set_area_name(self.context.dialog_new_name.clone(), &mut self.context, &mut self.region_widget);
            } else
            if self.state == EditorState::TilesDetail && self.context.dialog_entry == DialogEntry::Tags && self.context.dialog_accepted == true {
                self.tilemap_options.set_tags(self.context.dialog_new_name.clone(), asset, &self.context);
            } else
            if self.state == EditorState::BehaviorDetail {
                if self.context.dialog_entry == DialogEntry::NodeTile {
                    self.node_graph_behavior_details.set_node_atom_data(self.context.dialog_node_behavior_id.clone(), self.context.dialog_node_behavior_value.clone(), &mut self.context);
                } else {
                    self.node_graph_behavior_details.update_from_dialog(&mut self.context);
                }
            } else
            if self.state == EditorState::SystemsDetail {
                if self.context.dialog_entry == DialogEntry::NodeTile {
                    self.node_graph_systems_details.set_node_atom_data(self.context.dialog_node_behavior_id.clone(), self.context.dialog_node_behavior_value.clone(), &mut self.context);
                } else {
                    self.node_graph_systems_details.update_from_dialog(&mut self.context);
                }
            } else
            if self.state == EditorState::BehaviorOverview {
                if self.context.dialog_entry == DialogEntry::NewName && self.context.dialog_accepted == true {
                    //println!("dialog ended {} {}", self.context.dialog_new_name, self.context.dialog_new_name_type);
                    self.context.data.create_behavior(self.context.dialog_new_name.clone(), 0);

                    let mut node = NodeWidget::new(vec![self.context.dialog_new_name.clone()],
                    NodeUserData { position: (100, 50 + 150 * self.node_graph_behavior.nodes.len() as isize) });

                    let node_menu_atom = crate::atom::AtomWidget::new(vec!["Rename".to_string(), "Delete".to_string()], crate::atom::AtomWidgetType::NodeMenu, crate::atom::AtomData::new_as_int("menu".to_string(), 0));
                    node.menu = Some(node_menu_atom);

                    self.node_graph_behavior.nodes.push(node);
                    self.node_graph_behavior.dirty = true;
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
                    self.node_graph_behavior.update_from_dialog(&mut self.context);
                }
            } else
            if self.state == EditorState::SystemsOverview {
                if self.context.dialog_entry == DialogEntry::NewName && self.context.dialog_accepted == true {
                    //println!("dialog ended {} {}", self.context.dialog_new_name, self.context.dialog_new_name_type);
                    self.context.data.create_system(self.context.dialog_new_name.clone(), 0);

                    let mut node = NodeWidget::new(vec![self.context.dialog_new_name.clone()],
                    NodeUserData { position: (100, 50 + 150 * self.node_graph_systems.nodes.len() as isize) });

                    let node_menu_atom = crate::atom::AtomWidget::new(vec!["Rename".to_string(), "Delete".to_string()], crate::atom::AtomWidgetType::NodeMenu, crate::atom::AtomData::new_as_int("menu".to_string(), 0));
                    node.menu = Some(node_menu_atom);

                    self.node_graph_systems.nodes.push(node);
                    self.node_graph_systems.dirty = true;
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
                    self.node_graph_systems.update_from_dialog(&mut self.context);
                }
            }
            self.context.dialog_entry = DialogEntry::None;
        }

        // Draw overlay
        self.toolbar.draw_overlay(frame, &self.rect, anim_counter, asset, &mut self.context);

        //let stop = self.get_time();
        //println!("draw time {:?}", stop - start);
    }

    fn key_down(&mut self, char: Option<char>, key: Option<WidgetKey>, asset: &mut Asset) -> bool {
        if self.context.dialog_state == DialogState::Open {
            return self.dialog.key_down(char, key, asset, &mut self.context);
        }
        false
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {

        if self.context.dialog_state == DialogState::Open {
            return self.dialog.mouse_down(pos, asset, &mut self.context);
        }

        let mut consumed = false;
        if self.toolbar.mouse_down(pos, asset, &mut self.context) {

            // Tile Button
            if self.toolbar.widgets[1].clicked {
                if self.toolbar.widgets[1].selected {
                    self.node_graph_tiles.set_mode_and_rect( GraphMode::Overview, (0, self.rect.1 + self.context.toolbar_height, self.rect.2, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::TilesOverview;
                    self.node_graph_tiles.mark_all_dirty();
                } else
                if self.toolbar.widgets[1].right_selected {
                    self.node_graph_tiles.set_mode_and_rect( GraphMode::Detail, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::TilesDetail;
                    self.context.curr_graph_type = BehaviorType::Tiles;
                }

                for i in 2..=5 {
                    self.toolbar.widgets[i].selected = false;
                    self.toolbar.widgets[i].right_selected = false;
                    self.toolbar.widgets[i].dirty = true;
                }

                self.toolbar.widgets[0].text = asset.tileset.maps_names.clone();
                self.toolbar.widgets[0].curr_index = self.context.curr_tileset_index;
                self.toolbar.widgets[0].dirty = true;
            } else
            // Region Button
            if self.toolbar.widgets[2].clicked {
                if self.toolbar.widgets[2].selected {
                    self.node_graph_regions.set_mode_and_rect( GraphMode::Overview, (0, self.rect.1 + self.context.toolbar_height, self.rect.2, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::RegionOverview;
                    self.node_graph_regions.mark_all_dirty();
                } else
                if self.toolbar.widgets[2].right_selected {
                    self.node_graph_regions.set_mode_and_rect( GraphMode::Detail, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::RegionDetail;
                    self.context.curr_graph_type = BehaviorType::Regions;
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
            } else
            // Behavior Button
            if self.toolbar.widgets[3].clicked {
                if self.toolbar.widgets[3].selected {
                    self.node_graph_behavior.set_mode_and_rect( GraphMode::Overview, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::BehaviorOverview;
                    self.node_graph_behavior.mark_all_dirty();
                } else
                if self.toolbar.widgets[3].right_selected {
                    self.node_graph_behavior.set_mode_and_rect( GraphMode::Detail, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::BehaviorDetail;
                    self.context.curr_graph_type = BehaviorType::Behaviors;
                    self.node_graph_behavior_details.set_behavior_id(self.context.data.behaviors_ids[self.context.curr_behavior_index] , &mut self.context);
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
            } else
            // Systems Button
            if self.toolbar.widgets[4].clicked {
                if self.toolbar.widgets[4].selected {
                    self.node_graph_systems.set_mode_and_rect( GraphMode::Overview, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::SystemsOverview;
                    self.node_graph_systems.mark_all_dirty();
                } else
                if self.toolbar.widgets[4].right_selected {
                    self.node_graph_systems.set_mode_and_rect( GraphMode::Detail, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.left_width, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::SystemsDetail;
                    self.context.curr_graph_type = BehaviorType::Systems;
                    self.node_graph_systems_details.set_behavior_id(self.context.data.systems_ids[self.context.curr_systems_index] , &mut self.context);
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
            }
            consumed = true;
        }

        if consumed == false && self.state == EditorState::TilesOverview {
            if consumed == false && self.node_graph_tiles.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
                if self.node_graph_tiles.clicked {
                    self.toolbar.widgets[0].curr_index = self.context.curr_tileset_index;
                    self.toolbar.widgets[0].dirty = true;
                    self.tilemap.set_tilemap_id(asset.tileset.maps_ids[self.context.curr_tileset_index]);
                    self.node_graph_tiles.clicked = false;
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
            if consumed == false && self.node_graph_regions.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
                if self.node_graph_regions.clicked {
                    self.toolbar.widgets[0].curr_index = self.context.curr_region_index;
                    self.toolbar.widgets[0].dirty = true;
                    self.region_widget.set_region_id(self.context.data.regions_ids[self.context.curr_region_index], &mut self.context, &mut self.region_options);
                    self.node_graph_regions.clicked = false;
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
                    self.node_graph_behavior.clicked = false;
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

        consumed
    }

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {

        if self.context.dialog_state == DialogState::Open {
            return self.dialog.mouse_up(pos, asset, &mut self.context);
        }

        self.log_drag_start_pos = None;

        let mut consumed = false;
        if self.toolbar.mouse_up(pos, asset, &mut self.context) {

            if self.toolbar.widgets[0].new_selection.is_some() {
                if self.state == EditorState::TilesOverview || self.state == EditorState::TilesDetail {
                    self.node_graph_tiles.changed_selection(self.context.curr_tileset_index, self.toolbar.widgets[0].curr_index);
                    self.context.curr_tileset_index = self.toolbar.widgets[0].curr_index;
                    self.tilemap.set_tilemap_id(asset.tileset.maps_ids[self.context.curr_tileset_index]);
                    self.context.curr_tile = None;
                    self.tilemap_options.set_state(WidgetState::Disabled);
                } else
                if self.state == EditorState::RegionOverview || self.state == EditorState::RegionDetail {
                    self.node_graph_regions.changed_selection(self.context.curr_region_index, self.toolbar.widgets[0].curr_index);
                    self.context.curr_region_index = self.toolbar.widgets[0].curr_index;
                    self.region_widget.set_region_id(self.context.data.regions_ids[self.context.curr_region_index], &mut self.context, &mut self.region_options);
                } else
                if self.state == EditorState::BehaviorOverview || self.state == EditorState::BehaviorDetail {
                    self.node_graph_behavior.changed_selection(self.context.curr_behavior_index, self.toolbar.widgets[0].curr_index);
                    self.context.curr_behavior_index = self.toolbar.widgets[0].curr_index;
                    self.node_graph_behavior_details.set_behavior_id(self.context.data.behaviors_ids[self.context.curr_behavior_index] , &mut self.context);
                } else
                if self.state == EditorState::SystemsOverview || self.state == EditorState::SystemsDetail {
                    self.node_graph_systems.changed_selection(self.context.curr_systems_index, self.toolbar.widgets[0].curr_index);
                    self.context.curr_systems_index = self.toolbar.widgets[0].curr_index;
                    self.node_graph_systems_details.set_behavior_id(self.context.data.systems_ids[self.context.curr_systems_index] , &mut self.context);
                }
                self.toolbar.widgets[0].new_selection = None;
            }
            consumed = true;
        }

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
        }

        // Node Drag ?
        if let Some(drag_context) = &self.context.drag_context {

            if self.state == EditorState::BehaviorOverview {
                if self.context.contains_pos_for(pos, self.node_graph_behavior.rect) {

                    let mut position = (pos.0 as isize, pos.1 as isize);
                    position.0 -= self.node_graph_behavior.rect.0 as isize + self.node_graph_behavior.offset.0 + drag_context.offset.0;
                    position.1 -= self.node_graph_behavior.rect.1 as isize + self.node_graph_behavior.offset.1 + drag_context.offset.1;

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
                if self.context.contains_pos_for(pos, self.node_graph_behavior_details.rect) {
                    let mut position = (pos.0 as isize, pos.1 as isize);
                    position.0 -= self.node_graph_behavior_details.rect.0 as isize + self.node_graph_behavior_details.offset.0 + drag_context.offset.0;
                    position.1 -= self.node_graph_behavior_details.rect.1 as isize + self.node_graph_behavior_details.offset.1 + drag_context.offset.1;

                    self.node_graph_behavior_details.add_node_of_name(drag_context.text.clone(), position, &mut self.context);
                }
            } else
            if self.state == EditorState::SystemsOverview {
                if self.context.contains_pos_for(pos, self.node_graph_systems.rect) {

                    let mut position = (pos.0 as isize, pos.1 as isize);
                    position.0 -= self.node_graph_systems.rect.0 as isize + self.node_graph_systems.offset.0 + drag_context.offset.0;
                    position.1 -= self.node_graph_systems.rect.1 as isize + self.node_graph_systems.offset.1 + drag_context.offset.1;

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
                if self.context.contains_pos_for(pos, self.node_graph_systems_details.rect) {
                    let mut position = (pos.0 as isize, pos.1 as isize);
                    position.0 -= self.node_graph_systems_details.rect.0 as isize + self.node_graph_systems_details.offset.0 + drag_context.offset.0;
                    position.1 -= self.node_graph_systems_details.rect.1 as isize + self.node_graph_systems_details.offset.1 + drag_context.offset.1;

                    self.node_graph_systems_details.add_node_of_name(drag_context.text.clone(), position, &mut self.context);
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
            if consumed == false && self.node_graph_regions.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::RegionDetail {
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
        }
        self.mouse_pos = pos.clone();
        consumed
    }

    fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {

        if self.context.dialog_state == DialogState::Open {
            return self.dialog.mouse_hover(pos, asset, &mut self.context);
        }

        let mut consumed = false;
        self.mouse_hover_pos = pos.clone();

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
        consumed
    }

    fn mouse_wheel(&mut self, delta: (isize, isize), asset: &mut Asset) -> bool {

        if self.context.dialog_state == DialogState::Open {
            return self.dialog.mouse_wheel(delta, asset, &mut self.context);
        }

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
            if consumed == false && self.context.contains_pos_for(self.mouse_hover_pos,self.region_widget.rect) && self.region_widget.mouse_wheel(delta, asset, &mut self.context) {
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
        }
        consumed
    }

    fn get_target_fps(&self) -> usize {
        self.context.target_fps
    }
}