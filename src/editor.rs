
use crate::tileselector::TileSelectorWidget;
use crate::editor::areaoptions::AreaOptions;
use crate::editor::areawidget::AreaWidget;
use crate::widget:: {ScreenWidget, Widget};
use crate::tileset::TileUsage;

use server::asset::Asset;

mod toolbar;
mod nodegraph;
mod tilemapoptions;
mod tilemapwidget;
mod areawidget;
mod areaoptions;

use crate::editor::toolbar::ToolBar;
use tilemapwidget::TileMapWidget;

use crate::context::ScreenContext;

use crate::node::NodeUserData;

use crate::node::NodeWidget;
use crate::widget::node::NodeWidgetType;

use crate::editor::nodegraph::{ NodeGraph, GraphMode, GraphType };

use self::tilemapoptions::TileMapOptions;

#[derive (PartialEq)]
enum EditorState {
    TilesOverview,
    TilesDetail,
    AreaOverview,
    AreaDetail
}

/// The Editor struct
pub struct Editor {
    rect                    : (usize, usize, usize, usize),
    state                   : EditorState,
    context                 : ScreenContext,
    toolbar                 : ToolBar,

    tilemap_options         : TileMapOptions,
    tilemap                 : TileMapWidget,

    area_options            : AreaOptions,
    area_widget             : AreaWidget,
    area_tile_selector      : TileSelectorWidget,

    node_graph_tiles        : NodeGraph,
    node_graph_areas        : NodeGraph,

    left_width              : usize,
}

impl ScreenWidget for Editor {

    fn new(asset: &Asset, width: usize, height: usize) -> Self where Self: Sized {

        let left_width = 180_usize;
        let context = ScreenContext::new(width, height);

        let toolbar = ToolBar::new(vec!(), (0,0, width, context.toolbar_height), asset, &context);

        // Tile views and nodes
        let tilemap_options = TileMapOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);
        let tilemap = TileMapWidget::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), asset, &context);

        let mut tile_nodes = vec![];
        for (index, t) in asset.tileset.maps_names.iter().enumerate() {
            let node = NodeWidget::new(vec![t.to_string()], NodeWidgetType::Tile, vec![], NodeUserData { overview_position: (100, 50 + 150 * index as isize), position: (0, 0)});
            tile_nodes.push(node);
        }

        let node_graph_tiles = NodeGraph::new(vec!(), (0, context.toolbar_height, width, height - context.toolbar_height), asset, &context, GraphType::Tiles, tile_nodes);

        // Area views and nodes
        let area_options = AreaOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);
        let area_widget = AreaWidget::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height - 250), asset, &context);
        let mut area_tile_selector = TileSelectorWidget::new(vec!(), (left_width, area_widget.rect.1 + area_widget.rect.3, width - left_width, 250), asset, &context);
        area_tile_selector.set_tile_type(TileUsage::Environment, &asset);

        let mut area_nodes = vec![];
        for (index, area) in &context.data.areas {//asset.tileset.maps_names.iter().enumerate() {
            let node = NodeWidget::new(vec![area.name.to_string()], NodeWidgetType::Tile, vec![], NodeUserData { overview_position: (100, 50 + 150 * *index as isize), position: (0, 0)});
            area_nodes.push(node);
        }

        let node_graph_areas = NodeGraph::new(vec!(), (0, context.toolbar_height, width, height - context.toolbar_height), asset, &context, GraphType::Areas, area_nodes);

        Self {
            rect                    : (0, 0, width, height),
            state                   : EditorState::TilesOverview,
            context,
            toolbar,

            tilemap_options,
            tilemap,

            area_options,
            area_widget,
            area_tile_selector,

            node_graph_tiles,
            node_graph_areas,
            left_width
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.context.width = width; self.rect.2 = width;
        self.context.height = height; self.rect.3 = height;
        self.toolbar.resize(width, height, &self.context);

        self.tilemap_options.resize(self.left_width, height - self.context.toolbar_height, &self.context);
        self.tilemap.resize(width - self.left_width, height - self.context.toolbar_height, &self.context);

        self.area_options.resize(self.left_width, height - self.context.toolbar_height, &self.context);

        self.area_widget.rect = (self.left_width, self.context.toolbar_height, width - self.left_width, height - self.context.toolbar_height - 250);
        self.area_tile_selector.rect = (self.left_width, self.area_widget.rect.1 + self.area_widget.rect.3, width - self.left_width, 250);
        self.node_graph_tiles.resize(width, height - self.context.toolbar_height, &self.context);
        self.node_graph_areas.resize(width, height - self.context.toolbar_height, &self.context);
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
        if self.state == EditorState::AreaOverview {
            self.node_graph_areas.draw(frame, anim_counter, asset, &mut self.context);
        } else
        if self.state == EditorState::AreaDetail {
            self.area_options.draw(frame, anim_counter, asset, &mut self.context);
            self.area_widget.draw(frame, anim_counter, asset, &mut self.context);
            self.area_tile_selector.draw(frame, anim_counter, asset, &mut self.context);
        }
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {
        let mut consumed = false;

        if self.toolbar.mouse_down(pos, asset, &mut self.context) {

            if self.toolbar.widgets[0].clicked {
                if self.state == EditorState::TilesOverview || self.state == EditorState::TilesDetail {
                    self.node_graph_tiles.changed_selection(self.context.curr_tileset_index, self.toolbar.widgets[0].curr_index);
                    self.context.curr_tileset_index = self.toolbar.widgets[0].curr_index;
                    self.tilemap.set_tilemap_id(asset.tileset.maps_ids[self.context.curr_tileset_index]);
                } else
                if self.state == EditorState::AreaOverview || self.state == EditorState::AreaDetail {
                    self.node_graph_areas.changed_selection(self.context.curr_area_index, self.toolbar.widgets[0].curr_index);
                    self.context.curr_area_index = self.toolbar.widgets[0].curr_index;
                    self.area_widget.set_area_id(self.context.data.areas_ids[self.context.curr_area_index]);
                }
                self.toolbar.widgets[0].clicked = false;
            } else
            // Tile Button
            if self.toolbar.widgets[1].clicked {
                if self.toolbar.widgets[1].selected {
                    self.node_graph_tiles.set_mode( GraphMode::Overview, (0, self.rect.1 + self.context.toolbar_height, self.rect.2, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::TilesOverview;
                    self.node_graph_tiles.mark_all_dirty();

                    self.toolbar.widgets[2].selected = false;
                    self.toolbar.widgets[2].right_selected = false;
                    self.toolbar.widgets[2].dirty = true;
                } else
                if self.toolbar.widgets[1].right_selected {
                    self.node_graph_tiles.set_mode( GraphMode::Detail, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.rect.2, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::TilesDetail;

                    self.toolbar.widgets[2].selected = false;
                    self.toolbar.widgets[2].right_selected = false;
                    self.toolbar.widgets[2].dirty = true;
                }

                self.toolbar.widgets[0].text = asset.tileset.maps_names.clone();
                self.toolbar.widgets[0].curr_index = self.context.curr_tileset_index;
                self.toolbar.widgets[0].dirty = true;
            } else
            // Area Button
            if self.toolbar.widgets[2].clicked {
                if self.toolbar.widgets[2].selected {
                    self.node_graph_areas.set_mode( GraphMode::Overview, (0, self.rect.1 + self.context.toolbar_height, self.rect.2, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::AreaOverview;
                    self.node_graph_areas.mark_all_dirty();

                    self.toolbar.widgets[1].selected = false;
                    self.toolbar.widgets[1].right_selected = false;
                    self.toolbar.widgets[1].dirty = true;
                } else
                if self.toolbar.widgets[2].right_selected {
                    self.node_graph_areas.set_mode( GraphMode::Detail, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.rect.2, self.rect.3 - self.context.toolbar_height), &self.context);
                    self.state = EditorState::AreaDetail;

                    self.toolbar.widgets[1].selected = false;
                    self.toolbar.widgets[1].right_selected = false;
                    self.toolbar.widgets[1].dirty = true;
                }

                self.toolbar.widgets[0].text = self.context.data.areas_names.clone();
                self.toolbar.widgets[0].curr_index = self.context.curr_area_index;
                self.toolbar.widgets[0].dirty = true;
            }
            consumed = true;
        }

        if self.state == EditorState::TilesOverview {
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
        if self.state == EditorState::TilesDetail {
            if consumed == false && self.tilemap_options.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.tilemap.mouse_down(pos, asset, &mut self.context) {
                if self.tilemap.clicked == true {
                    self.tilemap_options.adjust_tile_usage(asset, &self.context);
                }
                consumed = true;
            }
        } else
        if self.state == EditorState::AreaOverview {
            if consumed == false && self.node_graph_areas.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
                if self.node_graph_areas.clicked {
                    self.toolbar.widgets[0].curr_index = self.context.curr_area_index;
                    self.toolbar.widgets[0].dirty = true;
                    self.area_widget.set_area_id(self.context.data.areas_ids[self.context.curr_area_index]);
                    self.node_graph_areas.clicked = false;
                }
            }
        } else
        if self.state == EditorState::AreaDetail {
            for atom in &mut self.area_options.widgets {
                if atom.mouse_down(pos, asset, &mut self.context) {
                    if atom.clicked {
                        if atom.atom_data.name == "GroupedList" {
                            if atom.curr_item_index == 0 {
                                self.area_tile_selector.set_tile_type(TileUsage::Environment, asset);
                            } else
                            if atom.curr_item_index == 1 {
                                self.area_tile_selector.set_tile_type(TileUsage::EnvBlocking, asset);
                            } else
                            if atom.curr_item_index == 2 {
                                self.area_tile_selector.set_tile_type(TileUsage::Water, asset);
                            }
                        }
                    }
                    consumed = true;
                }
            }
            if consumed == false && self.area_tile_selector.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.area_widget.mouse_down(pos, asset, &mut self.context) {

                if let Some(clicked) = self.area_widget.clicked {
                    if let Some(selected) = &self.area_tile_selector.selected {

                        //let area = self.context.data.areas.get(&self.area_widget.area_index).unwrap();
                        if let Some(area) = self.context.data.areas.get_mut(&self.area_widget.area_id) {
                            area.set_value(clicked, selected.clone());
                            area.save_data();
                        }
                    }

                }
                consumed = true;
            }
        }

        consumed
    }

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {
        let mut consumed = false;
        if self.toolbar.mouse_up(pos, asset, &mut self.context) {
            self.tilemap.set_tilemap_id(asset.tileset.maps_ids[self.toolbar.widgets[0].curr_index]);
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
        }
        consumed
    }

    fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {
        let mut consumed = false;

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
        }
        consumed
    }

    fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {
        let mut consumed = false;
        //consumed = self.widgets[self.curr_index as usize].mouse_hover(pos, asset);

        if consumed == false && self.toolbar.mouse_hover(pos, asset, &mut self.context) {
            consumed = true;
        }

        if self.state == EditorState::TilesDetail {
            if consumed == false && self.tilemap_options.mouse_hover(pos, asset, &mut self.context) {
                consumed = true;
            }
        }
        consumed
    }

    fn mouse_wheel(&mut self, delta: (isize, isize), asset: &mut Asset) -> bool {
        let mut consumed = false;
        //consumed = self.widgets[self.curr_index as usize].mouse_hover(pos, asset);

        if consumed == false && self.toolbar.mouse_wheel(delta, asset, &mut self.context) {
            consumed = true;
        }
        if self.state == EditorState::TilesOverview {
            if consumed == false && self.node_graph_tiles.mouse_wheel(delta, asset, &mut self.context) {
                consumed = true;
            }
        }
        if self.state == EditorState::TilesDetail {
            if consumed == false && self.tilemap.mouse_wheel(delta, asset, &mut self.context) {
                consumed = true;
            }
        }
        if self.state == EditorState::AreaOverview {
            if consumed == false && self.node_graph_areas.mouse_wheel(delta, asset, &mut self.context) {
                consumed = true;
            }
        }
        if self.state == EditorState::AreaDetail {
            if consumed == false && self.area_widget.mouse_wheel(delta, asset, &mut self.context) {
                consumed = true;
            }
        }
        consumed
    }

    fn get_target_fps(&self) -> usize {
        self.context.target_fps
    }
}