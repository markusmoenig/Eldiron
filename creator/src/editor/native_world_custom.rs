//! Custom authoring maps retain their ordered 2D base on the application GPU.
//! The shared preview builder supplies geometry, never destination pixels.
use super::*;
use std::hash::{Hash, Hasher};
use vek::Mat3;

#[derive(Clone, PartialEq)]
pub(super) struct Description {
    clip: Option<[u32; 4]>,
}
impl Description {
    pub(super) fn current(map: &Map, server: &CreatorContext) -> Option<Self> {
        if server.shared.editor_view_mode != EditorViewMode::D2 || map.camera == MapCamera::TwoDIso {
            return None;
        }
        let clip = match server.shared.get_map_context() {
            MapContext::Screen => {
                let grid = server.config.grid_size.max(1) as f32;
                let width = server.config.viewport.x as f32 / grid;
                let height = server.config.viewport.y as f32 / grid;
                Some([-width * 0.5, -height * 0.5, width, height])
            }
            MapContext::Character | MapContext::Item => Some([-5., -5., 10., 10.]),
            MapContext::Region
                if scope::uses_region(server) && server.shared.editing_surface.is_some() =>
            {
                None
            }
            _ => return None,
        };
        Some(Self {
            clip: clip.map(|clip| clip.map(f32::to_bits)),
        })
    }
    pub(super) fn clip(&self) -> Option<rusterix::Rect> {
        let [x, y, width, height] = self.clip?.map(f32::from_bits);
        Some(rusterix::Rect {
            x,
            y,
            width,
            height,
        })
    }
}

// Draft geometry can change before a committed revision. Keep this visual
// content stamp separate from the view identity used to retain pointer capture.
pub(super) fn content(map: &Map) -> u64 {
    let mut content = std::collections::hash_map::DefaultHasher::new();
    serde_json::to_vec(map)
        .expect("serializable custom map")
        .hash(&mut content);
    content.finish()
}

#[derive(Default)]
pub(super) struct Gpu {
    renderer: Option<eldiron_presentation::overview2d::Overview2DRenderer>,
    assets: Option<AssetKey>,
    asset_epoch: u64,
    key: Option<(Description, u64, usize, (usize, usize), ValueContainer)>,
    pub(super) image: Option<theframework_core::ImageHandle>,
}
impl Gpu {
    pub(super) fn clear(&mut self, images: &mut renderkit::ImageRegistry) {
        if let Some(image) = self.image {
            images.remove(image);
        }
        *self = Self::default();
    }
    pub(super) fn prepare(
        &mut self,
        description: &Description,
        content: u64,
        map: &Map,
        assets: &rusterix::Assets,
        binding: &Binding,
        scene: &rusterix::Scene,
        images: &mut renderkit::ImageRegistry,
    ) -> Result<(), String> {
        let assets_changed = self.assets.as_ref().is_none_or(|old| {
            old.source != binding.assets.source
                || old.prefabs != binding.assets.prefabs
                || !old.same_atlas(&binding.assets)
        });
        let key = (
            description.clone(),
            content,
            scene.animation_frame,
            binding.size,
            binding.guide_properties.clone(),
        );
        if !assets_changed && self.key.as_ref() == Some(&key) {
            return Ok(());
        }
        if assets_changed {
            self.asset_epoch = self
                .asset_epoch
                .checked_add(1)
                .ok_or("custom asset epoch exhausted")?;
            self.assets = Some(binding.assets.clone());
        }
        let size = (binding.size.0 as u32, binding.size.1 as u32);
        let transform = Mat3::<f32>::translation_2d(Vec2::new(
            map.offset.x + size.0 as f32 * 0.5,
            -map.offset.y + size.1 as f32 * 0.5,
        )) * Mat3::new(map.grid_size, 0., 0., 0., map.grid_size, 0., 0., 0., 1.);
        if self.renderer.is_none() {
            self.renderer = Some(
                eldiron_presentation::overview2d::Overview2DRenderer::new(images.context().clone())
                    .map_err(|e| e.to_string())?,
            );
        }
        let image = self
            .renderer
            .as_mut()
            .unwrap()
            .render_with_background(
                scene,
                assets,
                size,
                transform,
                binding.document,
                self.asset_epoch,
                scene.mapmini.occlusion_regions(),
                [0; 3],
            )
            .map_err(|e| e.to_string())?;
        if !self
            .image
            .and_then(|handle| images.get(handle))
            .is_some_and(|old| Arc::ptr_eq(&old, &image))
        {
            if let Some(old) = self.image.take() {
                images.remove(old);
            }
            self.image = Some(images.insert(image).map_err(|e| e.to_string())?);
        }
        self.key = Some(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_map_keys_track_uncommitted_geometry_and_context_clip() {
        let mut server = CreatorContext::default();
        server.shared.editor_view_mode = EditorViewMode::D2;
        server.shared.pc = ProjectContext::Screen(Uuid::new_v4());
        let mut map = Map::new();
        map.add_vertex_at(1., 2.);
        let initial = Description::current(&map, &server).unwrap();
        let initial_content = content(&map);
        let revision = map.changed;
        map.vertices[0].x += 0.25;
        assert_eq!(map.changed, revision);
        assert_ne!(
            content(&map),
            initial_content,
            "live geometry invalidates the base"
        );
        assert!(
            Description::current(&map, &server).unwrap() == initial,
            "live geometry retains the view identity of its gesture"
        );
        map.vertices[0].x -= 0.25;
        assert!(Description::current(&map, &server).unwrap() == initial);
        let screen_clip = initial.clip().unwrap();
        assert_eq!(
            screen_clip.width,
            server.config.viewport.x as f32 / server.config.grid_size.max(1) as f32
        );
        server.shared.pc = ProjectContext::Character(Uuid::new_v4());
        assert!(
            Description::current(&map, &server).is_none(),
            "character selection currently maps to the region preview, not its dormant custom map"
        );
        server.shared.editor_view_mode = EditorViewMode::Orbit;
        assert!(Description::current(&map, &server).is_none());
        server.shared.editor_view_mode = EditorViewMode::D2;
        server.shared.pc = ProjectContext::Region(Uuid::new_v4());
        assert!(Description::current(&map, &server).is_none());
        server.shared.pc = ProjectContext::Screen(Uuid::new_v4());
        map.camera = MapCamera::ThreeDIso;
        assert!(
            Description::current(&map, &server).is_some(),
            "stored game camera does not change the Screen editing projection"
        );
        map.camera = MapCamera::TwoDIso;
        assert!(
            Description::current(&map, &server).is_none(),
            "isometric maps use the dedicated producer"
        );
        map.camera = MapCamera::TwoD;
        server.shared.pc = ProjectContext::Region(Uuid::new_v4());
        server.shared.editing_surface = Some(rusterix::Surface::new(0));
        assert!(
            Description::current(&map, &server)
                .unwrap()
                .clip()
                .is_none()
        );
    }
}

#[cfg(test)]
pub(crate) struct Fixture {
    project: Project,
    assets: rusterix::Assets,
    history: crate::undo::UndoManager,
    context: ProjectContext,
    mode: EditorViewMode,
    character: ContentContext,
    item: ContentContext,
    viewport: Vec2<i32>,
    grid: i32,
    authored: Vec<u8>,
    clipboard: Map,
    animation: usize,
    editor_canvas: bool,
    editing: Option<rusterix::Surface>,
    active: Option<rusterix::Surface>,
    detail_mode: GeometryEditMode,
    parent: Option<(Uuid, Vec<u8>)>,
}

#[cfg(test)]
impl Editor {
    pub(crate) fn begin_native_custom_for_test(
        &mut self,
        kind: usize,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> Fixture {
        let mut fixture = Fixture {
            project: self.documents.project.clone(),
            assets: self.server_ctx.reference.read().assets.clone(),
            history: self.server_ctx.history.read().clone(),
            context: self.server_ctx.shared.pc,
            mode: self.server_ctx.shared.editor_view_mode,
            character: self.server_ctx.shared.curr_character,
            item: self.server_ctx.shared.curr_item,
            viewport: self.server_ctx.config.viewport,
            grid: self.server_ctx.config.grid_size,
            authored: Vec::new(),
            clipboard: self.server_ctx.shared.clipboard.clone(),
            animation: self.server_ctx.reference.read().client.animation_frame,
            editor_canvas: self.server_ctx.reference.read().client.use_editor_canvas,
            editing: self.server_ctx.shared.editing_surface.clone(),
            active: self.server_ctx.shared.active_detail_surface.clone(),
            detail_mode: self.server_ctx.shared.geometry_edit_mode,
            parent: None,
        };
        let mut pixels = Vec::new();
        for i in 0..16 {
            pixels.extend_from_slice(&if i % 3 == 0 {
                [240, 60, 120, 112]
            } else {
                [30, 160, 220, 255]
            });
        }
        let alternate = pixels
            .chunks_exact(4)
            .flat_map(|p| [p[2], p[0], p[1], p[3]])
            .collect();
        let mut tile = rusterix::Tile::from_texture(rusterix::Texture::new(pixels, 4, 4));
        tile.textures.push(rusterix::Texture::new(alternate, 4, 4));
        let tile_id = tile.id;
        self.documents.project.tiles.insert(tile.id, tile.clone());
        let mut tiles = fixture.assets.tiles.clone();
        tiles.insert(tile.id, tile);
        self.server_ctx.reference.write().assets.set_tiles(tiles);
        let mut map = Map::new();
        map.subdivisions = 4.;
        if kind == 2 || kind == 5 {
            map.camera = MapCamera::ThreeDIso;
        }
        if kind == 3 || kind == 4 {
            map.camera = MapCamera::TwoDIso;
        }
        map.grid_size = 32.25;
        map.offset = Vec2::new(0.25, -0.5);
        for (x, y, width, height) in [(-2., -1., 4., 2.), (-0.5, -0.5, 2., 2.)] {
            let sector = crate::project_templates::rectangle(&mut map, x, y, width, height);
            let sector = map.find_sector_mut(sector).unwrap();
            sector.properties.set(
                "source",
                Value::Source(rusterix::PixelSource::TileId(tile_id)),
            );
            sector.properties.set("tile_mode", Value::Int(0));
            sector.properties.set(
                "floor_source",
                Value::Source(rusterix::PixelSource::TileId(tile_id)),
            );
        }
        map.clear_selection();
        map.clear_temp();
        let context = match kind {
            0 | 2 | 3 => {
                let mut screen = shared::screen::Screen::new();
                screen.map = map;
                let id = screen.id;
                self.documents.project.screens.insert(id, screen);
                ProjectContext::Screen(id)
            }
            1 | 4 | 5 => {
                let region = self
                    .documents
                    .project
                    .get_region_ctx_mut(&self.server_ctx.shared)
                    .unwrap();
                let host = crate::project_templates::rectangle(&mut region.map, 3., 3., 3., 2.);
                let mut surface = rusterix::Surface::new(host);
                surface.calculate_geometry(&region.map);
                surface.world_vertices = region
                    .map
                    .find_sector(host)
                    .unwrap()
                    .vertices_world(&region.map)
                    .unwrap();
                assert_eq!(surface.world_vertices.len(), 4);
                surface.profile = Some(map.id);
                region.map.profiles.insert(map.id, map);
                region.map.surfaces.insert(surface.id, surface.clone());
                self.server_ctx.shared.editing_surface = Some(surface);
                self.server_ctx.shared.active_detail_surface = None;
                self.server_ctx.shared.geometry_edit_mode = GeometryEditMode::Geometry;
                fixture.parent = Some((region.map.id, serde_json::to_vec(&region.map).unwrap()));
                ProjectContext::Region(region.id)
            }
            _ => unreachable!("custom fixture kind"),
        };
        self.server_ctx.shared.editor_view_mode = EditorViewMode::D2;
        self.server_ctx.config.viewport = Vec2::new(320, 240);
        self.server_ctx.config.grid_size = 32;
        crate::undo::project_helper::set_project_context(
            ctx,
            ui,
            &self.documents.project,
            &mut self.server_ctx,
            context,
        );
        fixture.authored =
            serde_json::to_vec(scope::map(&self.documents.project, &self.server_ctx).unwrap())
                .unwrap();
        fixture
    }

    pub(crate) fn native_custom_plan_for_test(
        &self,
        fixture: &Fixture,
    ) -> Option<(PaintList, (u32, u32))> {
        let world = &self.native_viewport.world;
        let frame = world.presented.as_ref().filter(|frame| {
            frame.ready && (frame.binding.custom.is_some() || frame.isometric.is_some())
        })?;
        let map = scope::map(&self.documents.project, &self.server_ctx).unwrap();
        assert_eq!(serde_json::to_vec(map).unwrap(), fixture.authored);
        if let Some((id, authored)) = &fixture.parent {
            let parent = &self
                .documents
                .project
                .get_region_ctx(&self.server_ctx.shared)
                .unwrap()
                .map;
            assert_eq!(parent.id, *id);
            assert_eq!(
                &serde_json::to_vec(parent).unwrap(),
                authored,
                "profile preparation must preserve its parent"
            );
            assert_ne!(
                frame.binding.map, parent.id,
                "2D profile displays its child map"
            );
        }
        assert!(
            world.source.as_ref().unwrap().source.is_none(),
            "custom view cannot need a hidden scene builder"
        );
        assert!(world.gpu.as_ref().unwrap().custom.image.is_some() || frame.isometric.is_some());
        let mut list = PaintList::default();
        list.ops.push(PaintOp::DisplayCompositing(
            theframework::paint::DisplayCompositing::LegacyOpaque,
        ));
        let mut source_ops = world.published.as_ref().unwrap().1.ops.iter().peekable();
        // The source's explicit isometric canvas clear is part of its visual
        // contract. Include precisely that prefix, without consuming tool HUD
        // rectangles after the scene/guides.
        if source_ops.peek().is_some_and(|op| matches!(op,
            PaintOp::Rect { rect, radius, brush: theframework::paint::Brush::Solid(color) }
            if *radius == 0. && *color == [0.,0.,0.,1.]
                && *rect == Rect::new(0.,0.,frame.binding.size.0 as f32,frame.binding.size.1 as f32)))
        {
            list.ops.push(source_ops.next().unwrap().clone());
        }
        list.ops.extend(source_ops.take_while(|op| {
                    matches!(
                        op,
                        PaintOp::SpriteImage { .. } | PaintOp::SampledLine { .. }
                    )
                })
                .cloned(),
        );
        Some((
            list,
            (frame.binding.size.0 as u32, frame.binding.size.1 as u32),
        ))
    }

    pub(crate) fn native_custom_reference_for_test(
        &self,
        gpu: &Arc<renderkit::GpuContext>,
        size: (u32, u32),
    ) -> Vec<u8> {
        let bridge = self.server_ctx.reference.read();
        let mut reference =
            rusterix::Rusterix::with_scene_handler(rusterix::SceneHandler::with_vm(
                scenevm::SceneVM::new_with_context(gpu.clone(), size.0, size.1),
            ));
        reference.assets = bridge.assets.clone();
        reference
            .scene_handler
            .sync_base_render_settings(&reference.assets.config);
        reference.scene_handler.clear_overlay();
        reference
            .scene_handler
            .build_atlas(&reference.assets.tiles, true);
        reference.client.editor_canvas_colors = bridge.client.editor_canvas_colors;
        reference.client.use_editor_canvas = bridge.client.use_editor_canvas;
        reference.client.server_time = bridge.client.server_time;
        reference.client.animation_frame = bridge.client.animation_frame;
        reference
            .client
            .builder_d2
            .set_map_tool_type(self.server_ctx.shared.curr_map_tool_type);
        reference
            .client
            .builder_d2
            .set_map_hover_info(self.server_ctx.shared.hover, self.server_ctx.shared.hover_cursor);
        reference.client.builder_d2.draw_grid = self.server_ctx.shared.show_editor_grid;
        reference
            .client
            .builder_d2
            .set_properties(&self.build_values);
        let map = scope::map(&self.documents.project, &self.server_ctx).unwrap();
        reference.client.builder_d2.set_clip_rect(
            Description::current(map, &self.server_ctx).and_then(|description| description.clip()),
        );
        reference.build_custom_scene_d2(
            Vec2::new(size.0 as f32, size.1 as f32),
            map,
            &self.build_values,
            if scope::uses_region(&self.server_ctx) {
                &self.server_ctx.shared.editing_surface
            } else {
                &None
            },
            true,
        );
        let mut pixels = vec![0; size.0 as usize * size.1 as usize * 4];
        reference.scene_handler.runtime_render_state = bridge.render_state_for_view(map, true);
        reference.client.draw_custom_d2(
            map,
            &mut pixels,
            size.0 as usize,
            size.1 as usize,
            &reference.assets,
            &mut reference.scene_handler,
        );
        pixels
    }

    pub(crate) fn set_native_custom_phase_for_test(&mut self, phase: usize) {
        let mut bridge = self.server_ctx.reference.write();
        bridge.client.animation_frame = usize::from(phase != 0);
        if phase == 2 {
            bridge.client.use_editor_canvas = !bridge.client.use_editor_canvas;
        }
        if phase == 3 {
            let surface = self.server_ctx.shared.editing_surface.as_mut().unwrap();
            let shift = surface.edit_uv.right * 0.25;
            surface.world_vertices[0] += shift;
        }
    }

    pub(crate) fn finish_native_custom_for_test(
        &mut self,
        fixture: Fixture,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        self.documents.project = fixture.project;
        self.server_ctx.reference.write().assets = fixture.assets;
        self.server_ctx.reference.write().client.animation_frame = fixture.animation;
        self.server_ctx.reference.write().client.use_editor_canvas = fixture.editor_canvas;
        *self.server_ctx.history.write() = fixture.history;
        self.server_ctx.shared.clipboard = fixture.clipboard;
        self.server_ctx.shared.hover = (None, None, None);
        self.server_ctx.shared.hover_cursor = None;
        self.server_ctx.shared.geo_hit = None;
        self.server_ctx.shared.curr_character = fixture.character;
        self.server_ctx.shared.curr_item = fixture.item;
        self.server_ctx.shared.editing_surface = fixture.editing;
        self.server_ctx.shared.active_detail_surface = fixture.active;
        self.server_ctx.shared.geometry_edit_mode = fixture.detail_mode;
        self.server_ctx.config.viewport = fixture.viewport;
        self.server_ctx.config.grid_size = fixture.grid;
        self.server_ctx.shared.editor_view_mode = fixture.mode;
        crate::undo::project_helper::set_project_context(
            ctx,
            ui,
            &self.documents.project,
            &mut self.server_ctx,
            fixture.context,
        );
        self.server_ctx.shared.editor_view_mode = fixture.mode;
        self.server_ctx.history.read().set_undo_state_to_ui(ctx);
        self.invalidate_native_world_viewport(ui);
    }
}
