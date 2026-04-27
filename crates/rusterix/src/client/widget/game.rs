use crate::client::draw2d::Draw2D;
use crate::client::{apply_2d_visibility_mask, draw2d};
use crate::prelude::*;
use crate::{Assets, Map, MapMini, Pixel, PlayerCamera, Rect, SceneHandler, WHITE};
use crate::{ValueGroups, ValueTomlLoader};
use theframework::prelude::*;
use vek::Vec2;

use super::game_backend::{GameWidgetBackend, GraphicalGameWidgetBackend, TextGameWidgetBackend};

fn render_debug_enabled() -> bool {
    std::env::var("ELDIRON_RENDER_DEBUG")
        .map(|v| v != "0")
        .unwrap_or(false)
}

fn render_debug_log(message: &str) {
    use std::io::Write;

    eprintln!("{message}");

    let mut paths = Vec::new();
    paths.push(std::path::PathBuf::from("eldiron-render-debug.log"));
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        paths.push(parent.join("eldiron-render-debug.log"));
    }
    paths.push(std::env::temp_dir().join("eldiron-render-debug.log"));

    let mut seen = std::collections::HashSet::new();
    for path in paths {
        let key = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !seen.insert(key) {
            continue;
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(file, "{message}");
        }
    }
}

pub struct GameWidget {
    pub name: String,
    pub scenemanager: SceneManager,
    pub backend: Option<Box<dyn GameWidgetBackend>>,
    pub backend_name: String,

    pub camera_d3: Box<dyn D3Camera>,

    pub rect: Rect,

    pub scene: Scene,

    pub buffer: TheRGBABuffer,

    pub map_bbox: Vec4<f32>,

    pub grid_size: f32,
    pub top_left: Vec2<f32>,

    pub player_pos: Vec2<f32>,

    pub toml_str: String,
    pub table: ValueGroups,

    pub camera: PlayerCamera,

    // Used to detect region changes (have to rebuild the geometry)
    pub build_region_name: String,

    // Upscale factor (1.0 = no upscaling, >1.0 = render at lower res and upscale)
    pub upscale: f32,
    // Secondary buffer for rendering at lower resolution when upscale > 1
    pub upscale_buffer: TheRGBABuffer,

    pub current_sector_name: String,
    pub iso_hidden_sectors: FxHashSet<u32>,
    pub iso_sector_fade: FxHashMap<u32, f32>,
    pub force_dynamics_rebuild: bool,
    pub firstp_eye_level: f32,
    pub loaded_chunks: FxHashSet<(i32, i32)>,
    pub stream_load_radius_chunks: i32,
    pub stream_prefetch_radius_chunks: i32,
    pub chunk_build_budget_near: i32,
    pub chunk_build_budget_far: i32,
    pub last_stream_focus_chunk: Option<(i32, i32)>,
    pub text_draw2d: Draw2D,
    pub text_font: Option<fontdue::Font>,
    pub text_font_name: String,
    pub text_font_size: f32,
    pub text_color: Pixel,
    pub mapmini: MapMini,
}

impl Default for GameWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl GameWidget {
    fn is_2d_camera(camera: &PlayerCamera) -> bool {
        matches!(camera, PlayerCamera::D2 | PlayerCamera::D2Grid)
    }

    pub fn new() -> Self {
        Self {
            name: String::new(),
            scenemanager: SceneManager::default(),
            backend: Some(Box::new(GraphicalGameWidgetBackend::new())),
            backend_name: "graphical".to_string(),

            camera_d3: Box::new(D3FirstPCamera::new()),

            rect: Rect::default(),

            scene: Scene::default(),

            buffer: TheRGBABuffer::default(),

            map_bbox: Vec4::zero(),

            grid_size: 32.0,
            top_left: Vec2::zero(),

            player_pos: Vec2::zero(),

            toml_str: String::new(),
            table: ValueGroups::default(),

            camera: PlayerCamera::D2,

            build_region_name: String::new(),

            upscale: 1.0,
            upscale_buffer: TheRGBABuffer::default(),
            current_sector_name: String::new(),
            iso_hidden_sectors: FxHashSet::default(),
            iso_sector_fade: FxHashMap::default(),
            force_dynamics_rebuild: true,
            firstp_eye_level: 1.7,
            loaded_chunks: FxHashSet::default(),
            stream_load_radius_chunks: 2,
            stream_prefetch_radius_chunks: 5,
            chunk_build_budget_near: 10,
            chunk_build_budget_far: 2,
            last_stream_focus_chunk: None,
            text_draw2d: Draw2D::default(),
            text_font: None,
            text_font_name: String::new(),
            text_font_size: 18.0,
            text_color: WHITE,
            mapmini: MapMini::default(),
        }
    }

    fn set_backend_by_name(&mut self, name: &str) {
        self.backend_name = name.to_string();
        self.backend = Some(match name {
            "text" => Box::new(TextGameWidgetBackend::new()),
            _ => Box::new(GraphicalGameWidgetBackend::new()),
        });
    }

    fn apply_iso_camera_overrides(&self, iso: &mut D3IsoCamera) {
        if let Some(camera) = self.table.get("camera") {
            let default_azimuth = iso.get_parameter_f32("azimuth_deg");
            let default_elevation = iso.get_parameter_f32("elevation_deg");
            let default_scale = iso.scale();

            let azimuth = camera.get_float_default(
                "azimuth",
                camera.get_float_default("azimuth_deg", default_azimuth),
            );
            let elevation = camera.get_float_default(
                "elevation",
                camera.get_float_default("elevation_deg", default_elevation),
            );
            let scale = camera.get_float_default("scale", default_scale);

            iso.set_parameter_f32("azimuth_deg", azimuth);
            iso.set_parameter_f32("elevation_deg", elevation);
            iso.set_parameter_f32("scale", scale);
        }
    }

    pub fn set_camera_mode(&mut self, camera: PlayerCamera) {
        self.camera = camera;
        self.force_dynamics_rebuild = true;
        match self.camera {
            PlayerCamera::D2 | PlayerCamera::D2Grid => {}
            PlayerCamera::D3Iso => {
                let mut iso = D3IsoCamera::new();
                self.apply_iso_camera_overrides(&mut iso);
                self.camera_d3 = Box::new(iso);
            }
            PlayerCamera::D3FirstP | PlayerCamera::D3FirstPGrid => {
                self.camera_d3 = Box::new(D3FirstPCamera::new());
            }
        }
    }

    pub fn init(&mut self) {
        // Parse UI settings via the shared TOML loader to stay consistent.
        if let Ok(groups) = ValueTomlLoader::from_str(&self.toml_str) {
            if let Some(ui) = groups.get("ui") {
                self.grid_size = ui.get_float_default("grid_size", self.grid_size);
                self.upscale = ui.get_float_default("upscale", 1.0).max(1.0);
                self.stream_load_radius_chunks =
                    ui.get_int_default("chunk_load_radius", self.stream_load_radius_chunks);
                self.stream_prefetch_radius_chunks =
                    ui.get_int_default("chunk_prefetch_radius", self.stream_prefetch_radius_chunks);
                self.chunk_build_budget_near =
                    ui.get_int_default("chunk_build_budget_near", self.chunk_build_budget_near);
                self.chunk_build_budget_far =
                    ui.get_int_default("chunk_build_budget_far", self.chunk_build_budget_far);
                self.text_font_name =
                    ui.get_str_default("font".into(), self.text_font_name.clone());
                self.text_font_size = ui.get_float_default("font_size", self.text_font_size);
                if let Some(color) = ui.get_str("color".into()) {
                    self.text_color = Self::hex_to_rgba_u8(&color);
                }
                let presentation =
                    ui.get_str_default("presentation".into(), self.backend_name.clone());
                self.set_backend_by_name(&presentation);
            }
            self.table = groups;
            if let Some(camera) = self.table.get("camera") {
                let camera_type = camera.get_str_default("type".into(), "2d".into());
                if camera_type == "iso" {
                    self.set_camera_mode(PlayerCamera::D3Iso);
                } else if camera_type == "firstp" {
                    self.set_camera_mode(PlayerCamera::D3FirstP);
                } else {
                    self.set_camera_mode(PlayerCamera::D2);
                }
            }
        }
        self.force_dynamics_rebuild = true;
    }

    fn ensure_text_resources(&mut self, assets: &Assets) {
        if self.text_font.is_none() {
            if !self.text_font_name.is_empty()
                && let Some(font) = assets.fonts.get(&self.text_font_name)
            {
                self.text_font = Some(font.clone());
            }
            if self.text_font.is_none()
                && let Some(font) = assets.fonts.values().next()
            {
                self.text_font = Some(font.clone());
            }
        }
    }

    fn hex_to_rgba_u8(hex: &str) -> Pixel {
        let hex = hex.trim_start_matches('#');
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
                [r, g, b, 255]
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
                let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
                [r, g, b, a]
            }
            _ => WHITE,
        }
    }

    fn update_player_context(&mut self, map: &Map) {
        for entity in &map.entities {
            if entity.is_player() {
                if !Self::is_2d_camera(&self.camera) {
                    entity.apply_to_camera(&mut self.camera_d3, self.firstp_eye_level);
                }
                self.player_pos = entity.get_pos_xz();
                self.current_sector_name = entity
                    .get_attr_string("sector")
                    .filter(|s| !s.is_empty())
                    .or_else(|| map.find_sector_at(self.player_pos).map(|s| s.name.clone()))
                    .unwrap_or_default();
                break;
            }
        }
    }

    fn current_sector<'a>(&self, map: &'a Map) -> Option<&'a crate::Sector> {
        if !self.current_sector_name.is_empty() {
            map.sectors
                .iter()
                .find(|sector| sector.name == self.current_sector_name)
                .or_else(|| map.find_sector_at(self.player_pos))
        } else {
            map.find_sector_at(self.player_pos)
        }
    }

    fn sector_text_metadata(&self, sector: &crate::Sector) -> (String, String) {
        let mut title = sector.name.clone();
        let mut description = String::new();

        if let Some(crate::Value::Str(data)) = sector.properties.get("data")
            && let Ok(table) = data.parse::<toml::Table>()
        {
            for section in ["text_adventure", "text", "ui"] {
                if let Some(group) = table.get(section).and_then(toml::Value::as_table) {
                    if let Some(value) = group.get("title").and_then(toml::Value::as_str)
                        && !value.trim().is_empty()
                    {
                        title = value.to_string();
                    }
                    if let Some(value) = group.get("description").and_then(toml::Value::as_str)
                        && !value.trim().is_empty()
                    {
                        description = value.to_string();
                    }
                }
            }
        }

        (title, description)
    }

    fn text_lines(&self, map: &Map) -> Vec<String> {
        let mut lines = Vec::new();
        let Some(sector) = self.current_sector(map) else {
            lines.push("No current room.".to_string());
            return lines;
        };

        let (title, description) = self.sector_text_metadata(sector);
        if !title.is_empty() {
            lines.push(title);
            lines.push(String::new());
        }
        if !description.is_empty() {
            lines.push(description);
            lines.push(String::new());
        }

        let exits: Vec<String> = sector
            .linedefs
            .iter()
            .filter_map(|linedef_id| map.find_linedef(*linedef_id))
            .map(|linedef| linedef.name.clone())
            .filter(|name| !name.trim().is_empty())
            .collect();
        if !exits.is_empty() {
            lines.push(format!("Exits: {}", exits.join(", ")));
        }

        let entities: Vec<String> = map
            .entities
            .iter()
            .filter(|entity| !entity.is_player())
            .filter(|entity| {
                entity
                    .get_attr_string("sector")
                    .filter(|s| !s.is_empty())
                    .map(|s| s == sector.name)
                    .unwrap_or_else(|| {
                        map.find_sector_at(entity.get_pos_xz()).map(|s| s.id) == Some(sector.id)
                    })
            })
            .map(|entity| {
                entity
                    .get_attr_string("name")
                    .or_else(|| entity.get_attr_string("class_name"))
                    .unwrap_or_else(|| format!("Entity {}", entity.id))
            })
            .filter(|name| !name.trim().is_empty())
            .collect();
        if !entities.is_empty() {
            lines.push(format!("Characters: {}", entities.join(", ")));
        }

        let items: Vec<String> = map
            .items
            .iter()
            .filter(|item| {
                item.get_attr_string("sector")
                    .filter(|s| !s.is_empty())
                    .map(|s| s == sector.name)
                    .unwrap_or_else(|| {
                        map.find_sector_at(item.get_pos_xz()).map(|s| s.id) == Some(sector.id)
                    })
            })
            .map(|item| {
                item.get_attr_string("name")
                    .or_else(|| item.get_attr_string("class_name"))
                    .unwrap_or_else(|| format!("Item {}", item.id))
            })
            .filter(|name| !name.trim().is_empty())
            .collect();
        if !items.is_empty() {
            lines.push(format!("Items: {}", items.join(", ")));
        }

        if lines.is_empty() {
            lines.push("No room text available.".to_string());
        }
        lines
    }

    fn player_chunk_origin(&self, chunk_size: i32) -> (i32, i32) {
        let px = self.player_pos.x.floor() as i32;
        let py = self.player_pos.y.floor() as i32;
        (
            px.div_euclid(chunk_size) * chunk_size,
            py.div_euclid(chunk_size) * chunk_size,
        )
    }

    fn desired_stream_chunks(&self, map: &Map, radius_chunks: i32) -> FxHashSet<(i32, i32)> {
        let chunk_size = 32;
        let bbox = map.bbox();
        let min_x = (bbox.min.x / chunk_size as f32).floor() as i32;
        let min_y = (bbox.min.y / chunk_size as f32).floor() as i32;
        let max_x = (bbox.max.x / chunk_size as f32).ceil() as i32;
        let max_y = (bbox.max.y / chunk_size as f32).ceil() as i32;

        let center = self.player_chunk_origin(chunk_size);
        let center_cx = center.0.div_euclid(chunk_size);
        let center_cy = center.1.div_euclid(chunk_size);

        let mut desired = FxHashSet::default();
        for cy in (center_cy - radius_chunks)..=(center_cy + radius_chunks) {
            for cx in (center_cx - radius_chunks)..=(center_cx + radius_chunks) {
                if cx < min_x || cy < min_y || cx >= max_x || cy >= max_y {
                    continue;
                }
                desired.insert((cx * chunk_size, cy * chunk_size));
            }
        }
        desired
    }

    fn update_streaming_chunks(&mut self, map: &Map, scene_handler: &mut SceneHandler) {
        let chunk_size = 32;
        let focus = self.player_chunk_origin(chunk_size);
        if self.last_stream_focus_chunk == Some(focus) {
            return;
        }
        self.last_stream_focus_chunk = Some(focus);
        self.scenemanager.set_focus_chunk(Some(focus));

        let load_radius = self.stream_load_radius_chunks.max(1);
        let prefetch_radius = self.stream_prefetch_radius_chunks.max(load_radius);
        let desired_load = self.desired_stream_chunks(map, load_radius);
        let desired_prefetch = self.desired_stream_chunks(map, prefetch_radius);

        // Unload chunks that are now outside the prefetch radius.
        let unload: Vec<(i32, i32)> = self
            .loaded_chunks
            .iter()
            .copied()
            .filter(|c| !desired_prefetch.contains(c))
            .collect();
        for coord in unload {
            scene_handler.vm.execute(scenevm::Atom::RemoveChunkAt {
                origin: Vec2::new(coord.0, coord.1),
            });
            scene_handler.build_index.remove_chunk_origin(coord);
            self.loaded_chunks.remove(&coord);
        }

        // Prioritize immediate neighborhood, then allow wider prefetch queue.
        let mut to_request: Vec<(i32, i32)> = desired_load
            .iter()
            .copied()
            .filter(|c| !self.loaded_chunks.contains(c))
            .collect();
        to_request.extend(
            desired_prefetch
                .iter()
                .copied()
                .filter(|c| !self.loaded_chunks.contains(c) && !desired_load.contains(c)),
        );
        if !to_request.is_empty() {
            self.scenemanager.add_dirty(to_request);
        }
    }

    pub fn build(&mut self, map: &Map, assets: &Assets, scene_handler: &mut SceneHandler) {
        let mut backend = self
            .backend
            .take()
            .unwrap_or_else(|| Box::new(GraphicalGameWidgetBackend::new()));
        backend.build(self, map, assets, scene_handler);
        self.backend = Some(backend);
    }

    pub fn graphical_build(
        &mut self,
        map: &Map,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        if let Some(bbox) = map.bounding_box() {
            self.map_bbox = bbox;
        }
        // Force dynamic overlays (billboards/lights) to rebuild immediately after map swaps.
        scene_handler.mark_dynamics_dirty();
        self.force_dynamics_rebuild = true;

        self.scenemanager
            .set_tile_list(assets.tile_list.clone(), assets.tile_indices.clone());
        self.scenemanager
            .set_palette(assets.palette.clone(), assets.palette_materials.clone());
        self.mapmini = map.as_mini(&assets.blocking_tiles());

        self.scenemanager.send(SceneManagerCmd::SetMap(map.clone()));
        self.loaded_chunks.clear();
        self.last_stream_focus_chunk = None;
        // Replace full-map queue with a player-centric startup queue.
        let startup = self.desired_stream_chunks(map, self.stream_load_radius_chunks.max(1));
        self.scenemanager
            .replace_dirty(startup.iter().copied().collect::<Vec<_>>());
        self.scenemanager
            .set_focus_chunk(Some(self.player_chunk_origin(32)));
        self.build_region_name = map.name.clone();
        self.iso_hidden_sectors.clear();
        self.iso_sector_fade.clear();
    }

    pub fn apply_entities(
        &mut self,
        map: &Map,
        assets: &Assets,
        animation_frame: usize,
        scene_handler: &mut SceneHandler,
    ) {
        let mut backend = self
            .backend
            .take()
            .unwrap_or_else(|| Box::new(GraphicalGameWidgetBackend::new()));
        backend.apply_entities(self, map, assets, animation_frame, scene_handler);
        self.backend = Some(backend);
    }

    pub fn graphical_apply_entities(
        &mut self,
        map: &Map,
        assets: &Assets,
        animation_frame: usize,
        scene_handler: &mut SceneHandler,
    ) {
        if self.force_dynamics_rebuild {
            scene_handler.mark_dynamics_dirty();
            self.force_dynamics_rebuild = false;
        }

        self.update_player_context(map);
        if Self::is_2d_camera(&self.camera) {
            scene_handler.build_dynamics_2d(map, animation_frame, assets);
        } else {
            scene_handler.build_dynamics_3d(map, self.camera_d3.as_ref(), animation_frame, assets);
        }
    }

    pub fn text_build(&mut self, map: &Map, assets: &Assets) {
        if let Some(bbox) = map.bounding_box() {
            self.map_bbox = bbox;
        }
        self.ensure_text_resources(assets);
        self.build_region_name = map.name.clone();
    }

    pub fn text_apply_entities(&mut self, map: &Map) {
        self.update_player_context(map);
    }

    pub fn text_draw(&mut self, map: &Map, _time: &TheTime, assets: &Assets) {
        self.ensure_text_resources(assets);
        self.buffer.fill([0, 0, 0, 255]);

        let Some(font) = &self.text_font else {
            return;
        };

        let stride = self.buffer.stride();
        let width = self.buffer.dim().width as isize;
        let height = self.buffer.dim().height as isize;
        let mut y: isize = 12;
        let line_h = self.text_font_size.ceil() as isize + 6;

        for line in self.text_lines(map) {
            let rect = (12, y, width.saturating_sub(24), line_h);
            self.text_draw2d.text_rect_blend_safe(
                self.buffer.pixels_mut(),
                &rect,
                stride,
                font,
                self.text_font_size,
                &line,
                &self.text_color,
                draw2d::TheHorizontalAlign::Left,
                draw2d::TheVerticalAlign::Top,
                &(0, 0, width, height),
            );
            y += line_h;
            if y >= height {
                break;
            }
        }
    }

    pub fn draw(
        &mut self,
        map: &Map,
        time: &TheTime,
        animation_frame: usize,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        let mut backend = self
            .backend
            .take()
            .unwrap_or_else(|| Box::new(GraphicalGameWidgetBackend::new()));
        backend.draw(self, map, time, animation_frame, assets, scene_handler);
        self.backend = Some(backend);
    }

    pub fn graphical_draw(
        &mut self,
        map: &Map,
        time: &TheTime,
        animation_frame: usize,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        self.graphical_prepare_frame(map, time, animation_frame, assets, scene_handler);
        if Self::is_2d_camera(&self.camera) {
            self.render_prepared_d2(time, animation_frame, scene_handler);
        } else {
            self.render_prepared_d3(time, animation_frame, scene_handler);
        }
    }

    /// Prepare the SceneVM state for this widget without CPU readback.
    pub fn prepare_frame(
        &mut self,
        map: &Map,
        time: &TheTime,
        animation_frame: usize,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        let mut backend = self
            .backend
            .take()
            .unwrap_or_else(|| Box::new(GraphicalGameWidgetBackend::new()));
        backend.prepare_frame(self, map, time, animation_frame, assets, scene_handler);
        self.backend = Some(backend);
    }

    pub fn graphical_prepare_frame(
        &mut self,
        map: &Map,
        time: &TheTime,
        animation_frame: usize,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        let debug_enabled = render_debug_enabled();
        let debug_total_start = debug_enabled.then(std::time::Instant::now);
        let mut debug_build_ms = 0.0;
        let mut debug_stream_ms = 0.0;
        let mut debug_tick_ms = 0.0;
        let mut debug_receive_ms = 0.0;
        let mut debug_visibility_ms = 0.0;
        let mut debug_prepare_mode_ms = 0.0;
        let mut debug_processed_chunks = 0usize;
        let mut debug_received_chunks = 0usize;
        let mut debug_received_clears = 0usize;
        let debug_dirty_before = self.scenemanager.dirty_count();
        let debug_loaded_before = self.loaded_chunks.len();

        if map.name != self.build_region_name {
            let start = debug_enabled.then(std::time::Instant::now);
            self.graphical_build(map, assets, scene_handler);
            if let Some(start) = start {
                debug_build_ms = start.elapsed().as_secs_f64() * 1000.0;
            }
        }
        let start = debug_enabled.then(std::time::Instant::now);
        self.update_streaming_chunks(map, scene_handler);
        if let Some(start) = start {
            debug_stream_ms = start.elapsed().as_secs_f64() * 1000.0;
        }
        // Process more chunks per frame while nearby chunks are still missing.
        let desired_load = self.desired_stream_chunks(map, self.stream_load_radius_chunks.max(1));
        let missing_nearby = desired_load
            .iter()
            .filter(|coord| !self.loaded_chunks.contains(coord))
            .count();
        let budget = if missing_nearby > 0 {
            self.chunk_build_budget_near.max(1) as usize
        } else {
            self.chunk_build_budget_far.max(1) as usize
        };
        if self.scenemanager.is_busy() {
            let start = debug_enabled.then(std::time::Instant::now);
            debug_processed_chunks = self.scenemanager.tick_batch(budget);
            if let Some(start) = start {
                debug_tick_ms = start.elapsed().as_secs_f64() * 1000.0;
            }
        }

        // Apply scene manager chunks
        let mut geometry_changed = false;
        let start = debug_enabled.then(std::time::Instant::now);
        while let Some(result) = self.scenemanager.receive() {
            match result {
                SceneManagerResult::Chunk(chunk, _togo, _total, billboards) => {
                    debug_received_chunks += 1;
                    geometry_changed = true;
                    self.loaded_chunks.insert((chunk.origin.x, chunk.origin.y));
                    scene_handler
                        .build_index
                        .remove_chunk_origin((chunk.origin.x, chunk.origin.y));
                    scene_handler.vm.execute(scenevm::Atom::RemoveChunkAt {
                        origin: chunk.origin,
                    });

                    scene_handler.build_index.index_chunk(&chunk);
                    scene_handler.vm.execute(scenevm::Atom::AddChunk {
                        id: Uuid::new_v4(),
                        chunk: chunk,
                    });

                    // Add billboards to scene_handler (indexed by GeoId)
                    for billboard in billboards {
                        scene_handler.billboards.insert(billboard.geo_id, billboard);
                    }
                }
                SceneManagerResult::Clear => {
                    debug_received_clears += 1;
                    geometry_changed = true;
                    self.loaded_chunks.clear();
                    scene_handler.build_index.clear();
                    scene_handler.vm.execute(scenevm::Atom::ClearGeometry);
                    scene_handler.billboards.clear();
                    scene_handler.billboard_anim_states.clear();
                }
                _ => {}
            }
        }
        if let Some(start) = start {
            debug_receive_ms = start.elapsed().as_secs_f64() * 1000.0;
        }

        // Geometry streaming/reset can happen before dynamics are visible on first game frames.
        // Force a fresh dynamics pass whenever static scene content changed.
        if geometry_changed {
            scene_handler.mark_dynamics_dirty();
            self.force_dynamics_rebuild = true;
        }
        let start = debug_enabled.then(std::time::Instant::now);
        self.apply_iso_sector_visibility(map, scene_handler, geometry_changed);
        if let Some(start) = start {
            debug_visibility_ms = start.elapsed().as_secs_f64() * 1000.0;
        }

        if scene_handler.vm.vm_layer_count() > 1 {
            scene_handler.vm.set_layer_enabled(1, false);
        }
        if scene_handler.vm.vm_layer_count() > 2 {
            scene_handler.vm.set_layer_enabled(2, false);
        }

        if Self::is_2d_camera(&self.camera) {
            let start = debug_enabled.then(std::time::Instant::now);
            self.prepare_d2(time, animation_frame, scene_handler);
            if let Some(start) = start {
                debug_prepare_mode_ms = start.elapsed().as_secs_f64() * 1000.0;
            }
        } else {
            let start = debug_enabled.then(std::time::Instant::now);
            self.prepare_d3(map, time, animation_frame, scene_handler);
            if let Some(start) = start {
                debug_prepare_mode_ms = start.elapsed().as_secs_f64() * 1000.0;
            }
        }

        if let Some(start) = debug_total_start {
            let total_ms = start.elapsed().as_secs_f64() * 1000.0;
            if total_ms < 5.0
                && debug_processed_chunks == 0
                && debug_received_chunks == 0
                && debug_received_clears == 0
            {
                return;
            }
            render_debug_log(&format!(
                "[RenderDebug][GameWidget] prepare_frame total={:.2}ms build={:.2} stream={:.2} tick_batch={:.2} receive_upload={:.2} visibility={:.2} prepare_mode={:.2} budget={} processed={} received_chunks={} clears={} dirty {}->{} loaded {}->{} geometry_changed={}",
                total_ms,
                debug_build_ms,
                debug_stream_ms,
                debug_tick_ms,
                debug_receive_ms,
                debug_visibility_ms,
                debug_prepare_mode_ms,
                budget,
                debug_processed_chunks,
                debug_received_chunks,
                debug_received_clears,
                debug_dirty_before,
                self.scenemanager.dirty_count(),
                debug_loaded_before,
                self.loaded_chunks.len(),
                geometry_changed
            ));
        }
    }

    fn prepare_d2(
        &mut self,
        time: &TheTime,
        animation_frame: usize,
        scene_handler: &mut SceneHandler,
    ) {
        let full_width = self.buffer.dim().width as usize;
        let full_height = self.buffer.dim().height as usize;

        // Determine render dimensions based on upscale factor
        let (width, height, _render_buffer) = if self.upscale > 1.0 {
            let scaled_width = (full_width as f32 / self.upscale).round() as usize;
            let scaled_height = (full_height as f32 / self.upscale).round() as usize;

            // Allocate/resize upscale buffer if needed
            if self.upscale_buffer.dim().width as usize != scaled_width
                || self.upscale_buffer.dim().height as usize != scaled_height
            {
                self.upscale_buffer = TheRGBABuffer::new(TheDim::new(
                    0,
                    0,
                    scaled_width as i32,
                    scaled_height as i32,
                ));
            }
            (scaled_width, scaled_height, true)
        } else {
            (full_width, full_height, false)
        };

        let screen_size = Vec2::new(width as f32, height as f32);

        let bbox = self.map_bbox;

        let start = Vec2::new(bbox.x, bbox.y);
        let end = Vec2::new(bbox.x + bbox.z, bbox.y + bbox.w);

        let start_pixels = start * self.grid_size;
        let end_pixels = end * self.grid_size;

        // Ensure min < max even if grid_size has negative components
        let min_world = Vec2::new(
            start_pixels.x.min(end_pixels.x),
            start_pixels.y.min(end_pixels.y),
        );
        let max_world = Vec2::new(
            start_pixels.x.max(end_pixels.x),
            start_pixels.y.max(end_pixels.y),
        );

        let half_screen = screen_size / 2.0;

        // Compute unclamped camera center in world space
        let mut camera_pos = self.player_pos * self.grid_size;

        let map_width_px = max_world.x - min_world.x;
        let map_height_px = max_world.y - min_world.y;

        if map_width_px > screen_size.x {
            camera_pos.x = camera_pos
                .x
                .clamp(min_world.x + half_screen.x, max_world.x - half_screen.x);
        } else {
            // Center map horizontally
            camera_pos.x = (min_world.x + max_world.x) / 2.0;
        }

        if map_height_px > screen_size.y {
            camera_pos.y = camera_pos
                .y
                .clamp(min_world.y + half_screen.y, max_world.y - half_screen.y);
        } else {
            // Center map vertically
            camera_pos.y = (min_world.y + max_world.y) / 2.0;
        }

        let translation_matrix =
            Mat3::<f32>::translation_2d((screen_size / 2.0 - camera_pos).floor());

        self.top_left = (camera_pos - screen_size / 2.0).floor() / self.grid_size;

        let scale_matrix = Mat3::new(
            self.grid_size,
            0.0,
            0.0,
            0.0,
            self.grid_size,
            0.0,
            0.0,
            0.0,
            1.0,
        );
        let transform = translation_matrix * scale_matrix;

        let hour = time.to_f32();
        let scenevm_mode_2d = scene_handler.settings.scenevm_mode_2d();
        scene_handler.vm.set_active_vm(0);
        if matches!(scenevm_mode_2d, scenevm::RenderMode::Compute2D) {
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP0(Vec4::zero()));
        }

        scene_handler
            .vm
            .execute(scenevm::Atom::SetRenderMode(scenevm_mode_2d));

        scene_handler.settings.apply_hour(hour);
        scene_handler.apply_runtime_render_state_settings();
        scene_handler.settings.apply_2d(&mut scene_handler.vm);
        scene_handler.apply_runtime_render_state_2d();

        scene_handler
            .vm
            .execute(scenevm::Atom::SetTransform2D(transform));

        if scene_handler.vm.vm_layer_count() > 1 {
            scene_handler.vm.set_active_vm(1);
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP0(Vec4::zero()));
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP2(Vec4::zero()));
            scene_handler.vm.set_active_vm(0);
        }
        if scene_handler.vm.vm_layer_count() > 2 {
            scene_handler.vm.set_active_vm(2);
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP0(Vec4::zero()));
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP2(Vec4::zero()));
            scene_handler.vm.set_active_vm(0);
        }

        scene_handler
            .vm
            .execute(scenevm::Atom::SetAnimationCounter(animation_frame));

        // Draw Messages

        /*
        if let Some(font) = &self.messages_font {
            for (grid_pos, message, text_size, _) in self.messages_to_draw.values() {
                let position = map_grid_to_local(screen_size, *grid_pos, map);

                let tuple = (
                    position.x as isize - *text_size as isize / 2 - 5,
                    position.y as isize - self.messages_font_size as isize - map.grid_size as isize,
                    *text_size as isize + 10,
                    22,
                );

                self.draw2d.blend_rect_safe(
                    pixels,
                    &tuple,
                    width,
                    &[0, 0, 0, 128],
                    &(0, 0, width as isize, height as isize),
                );

                self.draw2d.text_rect_blend_safe(
                    pixels,
                    &tuple,
                    width,
                    font,
                    self.messages_font_size,
                    message,
                    &self.messages_font_color,
                    draw2d::TheHorizontalAlign::Center,
                    draw2d::TheVerticalAlign::Center,
                    &(0, 0, width as isize, height as isize),
                );
            }
        }*/
    }

    fn prepare_d3(
        &mut self,
        map: &Map,
        time: &TheTime,
        animation_frame: usize,
        scene_handler: &mut SceneHandler,
    ) {
        let full_width = self.buffer.dim().width as usize;
        let full_height = self.buffer.dim().height as usize;

        // Determine render dimensions based on upscale factor
        let (_width, _height, _render_buffer) = if self.upscale > 1.0 {
            let scaled_width = (full_width as f32 / self.upscale).round() as usize;
            let scaled_height = (full_height as f32 / self.upscale).round() as usize;

            // Allocate/resize upscale buffer if needed
            if self.upscale_buffer.dim().width as usize != scaled_width
                || self.upscale_buffer.dim().height as usize != scaled_height
            {
                self.upscale_buffer = TheRGBABuffer::new(TheDim::new(
                    0,
                    0,
                    scaled_width as i32,
                    scaled_height as i32,
                ));
            }
            (scaled_width, scaled_height, true)
        } else {
            (full_width, full_height, false)
        };

        let hour = time.to_f32();

        scene_handler.settings.apply_hour(hour);
        scene_handler.apply_dungeon_render_overrides(map);
        scene_handler.apply_runtime_render_state_settings();
        scene_handler.settings.apply_3d(&mut scene_handler.vm);
        scene_handler.apply_runtime_render_state_3d();

        scene_handler
            .vm
            .execute(scenevm::Atom::SetAnimationCounter(animation_frame));

        scene_handler
            .vm
            .execute(scenevm::Atom::SetBackground(Vec4::new(0.0, 0.0, 0.0, 1.0)));

        scene_handler.vm.execute(scenevm::Atom::SetRenderMode(
            scene_handler.settings.scenevm_mode_3d(),
        ));

        scene_handler.vm.execute(scenevm::Atom::SetCamera3D {
            camera: self.camera_d3.as_scenevm_camera(),
        });

        // scene_handler.vm.print_geometry_stats();
    }

    fn render_prepared_d2(
        &mut self,
        _time: &TheTime,
        _animation_frame: usize,
        scene_handler: &mut SceneHandler,
    ) {
        let full_width = self.buffer.dim().width as usize;
        let full_height = self.buffer.dim().height as usize;
        let (width, height, render_buffer) = if self.upscale > 1.0 {
            let scaled_width = (full_width as f32 / self.upscale).round() as usize;
            let scaled_height = (full_height as f32 / self.upscale).round() as usize;
            if self.upscale_buffer.dim().width as usize != scaled_width
                || self.upscale_buffer.dim().height as usize != scaled_height
            {
                self.upscale_buffer = TheRGBABuffer::new(TheDim::new(
                    0,
                    0,
                    scaled_width as i32,
                    scaled_height as i32,
                ));
            }
            (scaled_width, scaled_height, true)
        } else {
            (full_width, full_height, false)
        };

        if render_buffer {
            scene_handler.vm.render_frame(
                self.upscale_buffer.pixels_mut(),
                width as u32,
                height as u32,
            );
            Self::upscale_buffer_into(
                &self.upscale_buffer,
                &mut self.buffer,
                full_width,
                full_height,
            );
        } else {
            scene_handler
                .vm
                .render_frame(self.buffer.pixels_mut(), width as u32, height as u32);
        }

        let bg = scene_handler
            .settings
            .background_color_2d
            .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
        apply_2d_visibility_mask(
            self.buffer.pixels_mut(),
            full_width,
            full_height,
            &self.mapmini,
            self.grid_size,
            self.top_left,
            self.player_pos,
            scene_handler.settings.visibility_range_2d,
            scene_handler.settings.visibility_alpha_2d,
            bg,
        );
    }

    fn render_prepared_d3(
        &mut self,
        _time: &TheTime,
        _animation_frame: usize,
        scene_handler: &mut SceneHandler,
    ) {
        let full_width = self.buffer.dim().width as usize;
        let full_height = self.buffer.dim().height as usize;
        let (width, height, render_buffer) = if self.upscale > 1.0 {
            let scaled_width = (full_width as f32 / self.upscale).round() as usize;
            let scaled_height = (full_height as f32 / self.upscale).round() as usize;
            if self.upscale_buffer.dim().width as usize != scaled_width
                || self.upscale_buffer.dim().height as usize != scaled_height
            {
                self.upscale_buffer = TheRGBABuffer::new(TheDim::new(
                    0,
                    0,
                    scaled_width as i32,
                    scaled_height as i32,
                ));
            }
            (scaled_width, scaled_height, true)
        } else {
            (full_width, full_height, false)
        };

        if render_buffer {
            scene_handler.vm.render_frame(
                self.upscale_buffer.pixels_mut(),
                width as u32,
                height as u32,
            );
            Self::upscale_buffer_into(
                &self.upscale_buffer,
                &mut self.buffer,
                full_width,
                full_height,
            );
        } else {
            scene_handler
                .vm
                .render_frame(self.buffer.pixels_mut(), width as u32, height as u32);
        }
    }

    fn apply_iso_sector_visibility(
        &mut self,
        map: &Map,
        scene_handler: &mut SceneHandler,
        force_reapply: bool,
    ) {
        const FADE_STEP: f32 = 0.08;
        // Outside ISO mode, always force canonical sector visibility/opacity.
        if self.camera != PlayerCamera::D3Iso {
            scene_handler.vm.set_active_vm(0);
            for sector in &map.sectors {
                let is_dungeon_door_panel = sector
                    .properties
                    .get_str_default("generated_by", String::new())
                    == "dungeon_tool"
                    && sector
                        .properties
                        .get_str_default("dungeon_part", String::new())
                        == "door_panel";
                if is_dungeon_door_panel {
                    scene_handler.vm.execute(scenevm::Atom::SetGeoOpacity {
                        id: scenevm::GeoId::Sector(sector.id),
                        opacity: 0.0,
                    });
                    scene_handler.vm.execute(scenevm::Atom::SetGeoVisible {
                        id: scenevm::GeoId::Sector(sector.id),
                        visible: false,
                    });
                    continue;
                }
                scene_handler.vm.execute(scenevm::Atom::SetGeoOpacity {
                    id: scenevm::GeoId::Sector(sector.id),
                    opacity: 1.0,
                });
                scene_handler.vm.execute(scenevm::Atom::SetGeoVisible {
                    id: scenevm::GeoId::Sector(sector.id),
                    visible: sector.properties.get_bool_default("visible", true),
                });
            }
            self.iso_hidden_sectors.clear();
            self.iso_sector_fade.clear();
            return;
        }

        fn matches_pattern(name: &str, pattern: &str) -> bool {
            let name = name.trim().to_ascii_lowercase();
            let pattern = pattern.trim().to_ascii_lowercase();
            if pattern.is_empty() {
                return false;
            }
            if let Some(prefix) = pattern.strip_suffix('*') {
                name.starts_with(prefix)
            } else {
                name == pattern
            }
        }

        let mut target_hidden: FxHashSet<u32> = FxHashSet::default();

        // Multiple sectors may overlap at player position (e.g. foundations, interiors).
        // Collect hide patterns from all matching sectors instead of only the first one.
        let mut hide_patterns: Vec<String> = Vec::new();
        for sector in map
            .sectors
            .iter()
            .filter(|s| s.layer.is_none() && s.is_inside(map, self.player_pos))
        {
            if let Some(Value::StrArray(patterns)) = sector.properties.get("iso_hide_on_enter") {
                hide_patterns.extend(patterns.iter().cloned());
            }
        }
        if hide_patterns.is_empty()
            && let Some(current_sector) = map
                .sectors
                .iter()
                .find(|sector| sector.name == self.current_sector_name)
            && let Some(Value::StrArray(patterns)) =
                current_sector.properties.get("iso_hide_on_enter")
        {
            hide_patterns.extend(patterns.iter().cloned());
        }

        if !hide_patterns.is_empty() {
            for sector in &map.sectors {
                let roof_name = sector
                    .properties
                    .get_str_default("roof_name", String::new());
                let is_match = hide_patterns.iter().any(|pattern| {
                    matches_pattern(&sector.name, pattern)
                        || (!roof_name.is_empty() && matches_pattern(&roof_name, pattern))
                });
                if is_match {
                    target_hidden.insert(sector.id);
                }
            }
        }

        let unchanged = target_hidden == self.iso_hidden_sectors;
        let mut has_active_fade = false;
        for sector in &map.sectors {
            let target_alpha = if target_hidden.contains(&sector.id) {
                0.0
            } else {
                1.0
            };
            let current = *self
                .iso_sector_fade
                .get(&sector.id)
                .unwrap_or(&target_alpha);
            if (current - target_alpha).abs() > 1e-3 {
                has_active_fade = true;
                break;
            }
        }
        if !force_reapply && unchanged && !has_active_fade {
            return;
        }

        scene_handler.vm.set_active_vm(0);
        for sector in &map.sectors {
            let is_dungeon_door_panel = sector
                .properties
                .get_str_default("generated_by", String::new())
                == "dungeon_tool"
                && sector
                    .properties
                    .get_str_default("dungeon_part", String::new())
                    == "door_panel";
            if is_dungeon_door_panel {
                scene_handler.vm.execute(scenevm::Atom::SetGeoOpacity {
                    id: scenevm::GeoId::Sector(sector.id),
                    opacity: 0.0,
                });
                scene_handler.vm.execute(scenevm::Atom::SetGeoVisible {
                    id: scenevm::GeoId::Sector(sector.id),
                    visible: false,
                });
                self.iso_sector_fade.insert(sector.id, 0.0);
                continue;
            }
            let was_hidden = self.iso_hidden_sectors.contains(&sector.id);
            let should_hide = target_hidden.contains(&sector.id);
            let base_visible = sector.properties.get_bool_default("visible", true);
            let target_alpha = if should_hide { 0.0 } else { 1.0 };

            let current_alpha = self
                .iso_sector_fade
                .get(&sector.id)
                .copied()
                .unwrap_or(if was_hidden { 0.0 } else { 1.0 });

            let next_alpha = if current_alpha < target_alpha {
                (current_alpha + FADE_STEP).min(target_alpha)
            } else if current_alpha > target_alpha {
                (current_alpha - FADE_STEP).max(target_alpha)
            } else {
                current_alpha
            };

            // Ensure geometry is visible while fading in.
            if target_alpha > 0.0 && next_alpha > 0.0 {
                scene_handler.vm.execute(scenevm::Atom::SetGeoVisible {
                    id: scenevm::GeoId::Sector(sector.id),
                    visible: base_visible,
                });
            }

            scene_handler.vm.execute(scenevm::Atom::SetGeoOpacity {
                id: scenevm::GeoId::Sector(sector.id),
                opacity: next_alpha,
            });

            if next_alpha <= 0.001 {
                scene_handler.vm.execute(scenevm::Atom::SetGeoVisible {
                    id: scenevm::GeoId::Sector(sector.id),
                    visible: false,
                });
            }

            self.iso_sector_fade.insert(sector.id, next_alpha);
        }

        self.iso_hidden_sectors = target_hidden;
    }

    /// Upscale the source buffer into the destination buffer using nearest-neighbor sampling.
    fn upscale_buffer_into(
        src: &TheRGBABuffer,
        dst: &mut TheRGBABuffer,
        dst_width: usize,
        dst_height: usize,
    ) {
        let src_width = src.dim().width as usize;
        let src_height = src.dim().height as usize;
        let src_pixels = src.pixels();
        let dst_pixels = dst.pixels_mut();

        let x_ratio = src_width as f32 / dst_width as f32;
        let y_ratio = src_height as f32 / dst_height as f32;

        // Pre-compute source X indices for the row
        let mut src_x_indices: Vec<usize> = Vec::with_capacity(dst_width);
        for x in 0..dst_width {
            src_x_indices.push(((x as f32 * x_ratio) as usize).min(src_width - 1));
        }

        for y in 0..dst_height {
            let src_y = ((y as f32 * y_ratio) as usize).min(src_height - 1);
            let dst_row_start = y * dst_width * 4;
            let src_row_start = src_y * src_width * 4;

            for (x, &src_x) in src_x_indices.iter().enumerate() {
                let dst_idx = dst_row_start + x * 4;
                let src_idx = src_row_start + src_x * 4;

                dst_pixels[dst_idx..dst_idx + 4].copy_from_slice(&src_pixels[src_idx..src_idx + 4]);
            }
        }
    }
}
