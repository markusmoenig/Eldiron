//! Immutable minimap asset previews. Assets stay at source resolution; fitting
//! and alpha composition are performed by the common GPU painter.
use crate::prelude::*;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::Arc,
};
use theframework::{
    paint::{PaintOp, Rect},
    theui::{
        paint::{UiPaintError, UiPaintResult, WidgetPainter},
        thewidget::therenderview::RenderViewPaintSource,
    },
};

struct Image {
    key: String,
    pixels: TheRGBABuffer,
}
struct CachedImage {
    image: Arc<Image>,
    used: u64,
}
struct Preview {
    size: Vec2<i32>,
    image: Option<Arc<Image>>,
    error: Option<String>,
}
impl RenderViewPaintSource for Preview {
    fn paint(&self, painter: &mut WidgetPainter<'_>) -> UiPaintResult {
        if let Some(error) = &self.error {
            return Err(UiPaintError(error.clone()));
        }
        painter.list.ops.push(PaintOp::PixelRects {
            rects: vec![Rect::new(0., 0., self.size.x as f32, self.size.y as f32)].into(),
            color: [0., 0., 0., 1.],
        });
        if let Some(source) = &self.image {
            let width = source.pixels.dim().width as f32;
            let height = source.pixels.dim().height as f32;
            let scale = (self.size.x as f32 / width).min(self.size.y as f32 / height);
            let extent = [width * scale, height * scale];
            let image = painter.images.image(&source.key, &source.pixels, false)?;
            painter.list.ops.push(PaintOp::SpriteImage {
                image,
                origin: [
                    ((self.size.x as f32 - extent[0]) * 0.5).round(),
                    ((self.size.y as f32 - extent[1]) * 0.5).round(),
                ],
                scale: [extent[0].round() / width, extent[1].round() / height],
                tint: [1.; 4],
            });
        }
        Ok(())
    }
}

/// Sidebar-owned, bounded source cache. A published preview keeps its immutable
/// image alive independently of cache eviction or a subsequent document edit.
#[derive(Default)]
pub struct Sources {
    images: HashMap<u64, CachedImage>,
    bytes: usize,
    clock: u64,
    active: Option<(Vec2<i32>, Option<u64>, Arc<dyn RenderViewPaintSource>)>,
}
impl Sources {
    pub fn clear_active(&mut self) {
        self.active = None;
    }
    pub fn sprite(
        &mut self,
        size: Vec2<i32>,
        texture: Option<&rusterix::Texture>,
    ) -> Arc<dyn RenderViewPaintSource> {
        let size = size.map(|n| n.max(0));
        let key = texture.map(|texture| {
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            texture.width.hash(&mut hash);
            texture.height.hash(&mut hash);
            texture.data.hash(&mut hash);
            hash.finish()
        });
        if let Some((old_size, old_key, source)) = &self.active
            && *old_size == size
            && *old_key == key
        {
            return source.clone();
        }
        self.clock = self.clock.wrapping_add(1);
        let mut error = None;
        let image = texture.zip(key).and_then(|(texture, key)| {
            if texture.width == 0
                || texture.height == 0
                || texture.width > i32::MAX as usize
                || texture.height > i32::MAX as usize
                || texture
                    .width
                    .checked_mul(texture.height)
                    .and_then(|n| n.checked_mul(4))
                    != Some(texture.data.len())
            {
                error = Some("Invalid minimap preview source image".into());
                return None;
            }
            if let Some(cached) = self.images.get_mut(&key) {
                cached.used = self.clock;
                return Some(cached.image.clone());
            }
            let image = Arc::new(Image {
                key: format!("minimap-source:{}", Uuid::new_v4()),
                pixels: TheRGBABuffer::from(
                    texture.data.clone(),
                    texture.width as u32,
                    texture.height as u32,
                ),
            });
            self.bytes += texture.data.len();
            self.images.insert(
                key,
                CachedImage {
                    image: image.clone(),
                    used: self.clock,
                },
            );
            Some(image)
        });
        // Bound both pixels and metadata. Published/in-flight frames retain their
        // own Arcs; the host image registry independently budgets GPU residency.
        while self.bytes > 32 * 1024 * 1024 || self.images.len() > 128 {
            let Some(oldest) = self
                .images
                .iter()
                .min_by_key(|(_, image)| image.used)
                .map(|(key, _)| *key)
            else {
                break;
            };
            let old = self.images.remove(&oldest).unwrap();
            self.bytes -= old.image.pixels.pixels().len();
        }
        let source: Arc<dyn RenderViewPaintSource> = Arc::new(Preview { size, image, error });
        self.active = Some((size, key, source.clone()));
        source
    }
}

/// The existing assembled Creator checkpoint compares the retained CPU oracle
/// with the actual GPU compositor; it also goes through Sidebar's real event
/// route to prove that these docks no longer allocate a display bitmap.
#[cfg(test)]
pub(crate) fn reference_cases() -> Vec<(&'static str, Arc<dyn RenderViewPaintSource>, TheRGBABuffer)>
{
    use crate::{
        dockmanager::DockManagerState,
        docks::{tilemap::TilemapDock, tiles::TilesDock, tiles_editor::TilesEditorDock},
        editor::DOCKMANAGER,
        sidebar::Sidebar,
    };
    let mut project = Project::default();
    let mut tile = rusterix::Tile::default();
    for frame in 0..2 {
        let (width, height) = if frame == 0 { (19, 31) } else { (37, 13) };
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                pixels.extend([
                    (x * 23 % 256) as u8,
                    (y * 31 % 256) as u8,
                    ((x + y + frame) * 17 % 256) as u8,
                    [0, 64, 128, 192, 255][(x + y) % 5],
                ]);
            }
        }
        tile.textures
            .push(rusterix::Texture::new(pixels, width, height));
    }
    tile.isometric = Some(rusterix::isometric::SpriteLayout {
        frames: tile.textures.iter().rev().cloned().collect(),
        ..Default::default()
    });
    let id = tile.id;
    project.tiles.insert(id, tile.clone());
    let mut server = ServerContext::new();
    server.editing_ctx = PixelEditingContext::Tile(id, 0);
    let mut region = Region::new();
    server.curr_region = region.id;
    region.map.camera = MapCamera::TwoDIso;
    project.regions.push(region);
    let mut sources = Sources::default();
    let mut cases = Vec::new();
    for frame in 0..2 {
        server.animation_counter = frame;
        for (label, size, dock) in [
            (
                "minimap pixel editor",
                Vec2::new(143, 83),
                Box::new(TilesEditorDock::native_minimap_fixture()) as Box<dyn Dock>,
            ),
            (
                "minimap tilemap",
                Vec2::new(83, 143),
                Box::new(TilemapDock::native_minimap_fixture(tile.clone())) as Box<dyn Dock>,
            ),
            (
                "minimap iso hover",
                Vec2::new(127, 91),
                Box::new(TilesDock::native_minimap_fixture(id)) as Box<dyn Dock>,
            ),
        ] {
            let source = dock
                .native_minimap_source(size, &project, &server, &mut sources)
                .unwrap();
            let same = dock
                .native_minimap_source(size, &project, &server, &mut sources)
                .unwrap();
            assert!(
                Arc::ptr_eq(&source, &same),
                "unchanged previews must not invalidate the root"
            );
            let mut context = TheContext::new(size.x as usize, size.y as usize, 1.);
            let mut reference = TheRGBABuffer::new(TheDim::sized(size.x, size.y));
            assert!(dock.draw_minimap(&mut reference, &project, &mut context, &server));
            for pixel in reference.pixels_mut().chunks_exact_mut(4) {
                pixel[3] = 255;
            }

            const KEY: &str = "Native Minimap Checkpoint";
            struct Restore(DockManagerState, usize, String);
            impl Drop for Restore {
                fn drop(&mut self) {
                    let mut manager = DOCKMANAGER.write().unwrap();
                    manager.docks.shift_remove(KEY);
                    manager.state = self.0;
                    manager.index = self.1;
                    manager.dock = self.2.clone();
                }
            }
            let restore = {
                let mut manager = DOCKMANAGER.write().unwrap();
                assert!(!manager.docks.contains_key(KEY));
                let saved = Restore(manager.state, manager.index, manager.dock.clone());
                manager.docks.insert(KEY.into(), dock);
                manager.index = manager.docks.get_index_of(KEY).unwrap();
                manager.state = DockManagerState::Minimized;
                saved
            };
            let mut ui = TheUI::new();
            let mut view = TheRenderView::new(TheId::named("MiniMap"));
            view.set_dim(TheDim::sized(size.x, size.y), &mut context);
            // Start with the old adapter, proving that publication releases it.
            view.render_buffer_mut().resize(size.x, size.y);
            ui.canvas.set_widget(view);
            context.ui.command_paint = true;
            let mut sidebar = Sidebar::new();
            assert!(sidebar.handle_event(
                &TheEvent::Custom(TheId::named("Soft Update Minimap"), TheValue::Empty),
                &mut ui,
                &mut context,
                &mut project,
                &mut server
            ));
            assert!(
                !ui.get_render_view("MiniMap")
                    .unwrap()
                    .has_cpu_display_storage()
            );
            ui.get_render_view("MiniMap")
                .unwrap()
                .set_needs_redraw(false);
            assert!(
                !sidebar.handle_event(
                    &TheEvent::Custom(TheId::named("Soft Update Minimap"), TheValue::Empty),
                    &mut ui,
                    &mut context,
                    &mut project,
                    &mut server,
                ),
                "a cached static preview must not schedule another presentation"
            );
            drop(restore);
            cases.push((label, source, reference));
        }
    }
    assert_eq!(
        sources.images.len(),
        2,
        "all animation views reuse the original frame images"
    );
    // Same persisted texture identity, edited pixels: old published sources
    // remain immutable while the editor's next frame uses the new revision.
    project.tiles.get_mut(&id).unwrap().textures[0].data[0..4]
        .copy_from_slice(&[255, 80, 120, 255]);
    server.animation_counter = 0;
    let size = Vec2::new(143, 83);
    let dock = TilesEditorDock::native_minimap_fixture();
    let source = dock
        .native_minimap_source(size, &project, &server, &mut sources)
        .unwrap();
    let mut context = TheContext::new(size.x as usize, size.y as usize, 1.);
    let mut reference = TheRGBABuffer::new(TheDim::sized(size.x, size.y));
    assert!(dock.draw_minimap(&mut reference, &project, &mut context, &server));
    for pixel in reference.pixels_mut().chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    cases.push(("minimap edited pixels", source, reference));
    cases
}
