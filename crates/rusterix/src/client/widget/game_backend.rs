use crate::{Assets, Map, SceneHandler};
use theframework::prelude::*;

use super::game::GameWidget;

pub trait GameWidgetBackend: Send + Sync {
    fn name(&self) -> &'static str;

    fn build(
        &mut self,
        widget: &mut GameWidget,
        map: &Map,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    );

    fn apply_entities(
        &mut self,
        widget: &mut GameWidget,
        map: &Map,
        assets: &Assets,
        animation_frame: usize,
        scene_handler: &mut SceneHandler,
    );

    fn draw(
        &mut self,
        widget: &mut GameWidget,
        map: &Map,
        time: &TheTime,
        animation_frame: usize,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    );

    fn prepare_frame(
        &mut self,
        widget: &mut GameWidget,
        map: &Map,
        time: &TheTime,
        animation_frame: usize,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    );
}

pub struct GraphicalGameWidgetBackend;

impl GraphicalGameWidgetBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GraphicalGameWidgetBackend {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TextGameWidgetBackend;

impl TextGameWidgetBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TextGameWidgetBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl GameWidgetBackend for GraphicalGameWidgetBackend {
    fn name(&self) -> &'static str {
        "graphical"
    }

    fn build(
        &mut self,
        widget: &mut GameWidget,
        map: &Map,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        widget.graphical_build(map, assets, scene_handler);
    }

    fn apply_entities(
        &mut self,
        widget: &mut GameWidget,
        map: &Map,
        assets: &Assets,
        animation_frame: usize,
        scene_handler: &mut SceneHandler,
    ) {
        widget.graphical_apply_entities(map, assets, animation_frame, scene_handler);
    }

    fn draw(
        &mut self,
        widget: &mut GameWidget,
        map: &Map,
        time: &TheTime,
        animation_frame: usize,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        widget.graphical_draw(map, time, animation_frame, assets, scene_handler);
    }

    fn prepare_frame(
        &mut self,
        widget: &mut GameWidget,
        map: &Map,
        time: &TheTime,
        animation_frame: usize,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        widget.graphical_prepare_frame(map, time, animation_frame, assets, scene_handler);
    }
}

impl GameWidgetBackend for TextGameWidgetBackend {
    fn name(&self) -> &'static str {
        "text"
    }

    fn build(
        &mut self,
        widget: &mut GameWidget,
        map: &Map,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        let _ = scene_handler;
        widget.text_build(map, assets);
    }

    fn apply_entities(
        &mut self,
        widget: &mut GameWidget,
        map: &Map,
        assets: &Assets,
        animation_frame: usize,
        scene_handler: &mut SceneHandler,
    ) {
        let _ = (assets, animation_frame, scene_handler);
        widget.text_apply_entities(map);
    }

    fn draw(
        &mut self,
        widget: &mut GameWidget,
        map: &Map,
        time: &TheTime,
        animation_frame: usize,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        let _ = (animation_frame, scene_handler);
        widget.text_draw(map, time, assets);
    }

    fn prepare_frame(
        &mut self,
        widget: &mut GameWidget,
        map: &Map,
        time: &TheTime,
        animation_frame: usize,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        let _ = (time, animation_frame, scene_handler);
        if map.name != widget.build_region_name {
            widget.text_build(map, assets);
        }
        widget.text_apply_entities(map);
    }
}
