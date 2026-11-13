//! Immutable tile-board paint. Only original/cropped asset pixels are retained;
//! viewport scaling, backgrounds, borders and ghost composition happen on GPU.
use super::*;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
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
    _parent: Option<Arc<Image>>,
}
#[derive(Default)]
pub(super) struct Images {
    textures: HashMap<u64, Weak<Image>>,
    crops: HashMap<(String, [usize; 4]), Weak<Image>>,
}
impl Images {
    fn prune(&mut self) {
        self.textures.retain(|_, image| image.strong_count() > 0);
        self.crops.retain(|_, image| image.strong_count() > 0);
    }
    fn texture(&mut self, texture: &rusterix::Texture, bitmap: bool) -> Result<Arc<Image>, String> {
        let length = texture
            .width
            .checked_mul(texture.height)
            .and_then(|n| n.checked_mul(4));
        if texture.width == 0
            || texture.height == 0
            || length != Some(texture.data.len())
            || texture.width > i32::MAX as usize
            || texture.height > i32::MAX as usize
        {
            return Err("Invalid tile-picker source image".into());
        }
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        texture.width.hash(&mut hash);
        texture.height.hash(&mut hash);
        bitmap.hash(&mut hash);
        texture.data.hash(&mut hash);
        let key = hash.finish();
        if let Some(image) = self.textures.get(&key).and_then(Weak::upgrade) {
            return Ok(image);
        }
        let mut crop = [0, 0, texture.width, texture.height];
        if bitmap {
            let mut bounds = [texture.width, texture.height, 0, 0];
            for y in 0..texture.height {
                for x in 0..texture.width {
                    if texture.data[(y * texture.width + x) * 4 + 3] > 0 {
                        bounds[0] = bounds[0].min(x);
                        bounds[1] = bounds[1].min(y);
                        bounds[2] = bounds[2].max(x + 1);
                        bounds[3] = bounds[3].max(y + 1);
                    }
                }
            }
            if bounds[2] > bounds[0] && bounds[3] > bounds[1] {
                crop = [
                    bounds[0],
                    bounds[1],
                    bounds[2] - bounds[0],
                    bounds[3] - bounds[1],
                ];
            } else {
                crop = [0, 0, 1, 1];
            }
        }
        let bytes = TilesDock::crop_texture_rgba(
            &texture.data,
            (texture.width, texture.height),
            (crop[0], crop[1], crop[2], crop[3]),
        );
        let image = Arc::new(Image {
            key: format!("tile-board-source:{}", Uuid::new_v4()),
            pixels: TheRGBABuffer::from(bytes, crop[2] as u32, crop[3] as u32),
            _parent: None,
        });
        self.textures.insert(key, Arc::downgrade(&image));
        Ok(image)
    }
    fn crop(&mut self, image: Arc<Image>, crop: [usize; 4]) -> Arc<Image> {
        let width = image.pixels.dim().width as usize;
        let height = image.pixels.dim().height as usize;
        if crop == [0, 0, width, height] {
            return image;
        }
        let key = (image.key.clone(), crop);
        if let Some(image) = self.crops.get(&key).and_then(Weak::upgrade) {
            return image;
        }
        let bytes = TilesDock::crop_texture_rgba(
            image.pixels.pixels(),
            (width, height),
            (crop[0], crop[1], crop[2], crop[3]),
        );
        let cropped = Arc::new(Image {
            key: format!("tile-board-crop:{}", Uuid::new_v4()),
            pixels: TheRGBABuffer::from(bytes, crop[2] as u32, crop[3] as u32),
            _parent: Some(image),
        });
        self.crops.insert(key, Arc::downgrade(&cropped));
        cropped
    }
}

enum Node {
    Rects {
        rects: Arc<[Rect]>,
        color: [f32; 4],
    },
    Radials {
        layer: Arc<theframework::paint::PixelRadialLayer>,
        rect: Rect,
        alpha: f32,
    },
    Image {
        source: Arc<Image>,
        rect: Rect,
        alpha: f32,
    },
}
pub(super) struct Board {
    nodes: Vec<Node>,
    error: Option<String>,
}
impl RenderViewPaintSource for Board {
    fn paint(&self, p: &mut WidgetPainter<'_>, _viewport: [f32; 2]) -> UiPaintResult {
        if let Some(error) = &self.error {
            return Err(UiPaintError(error.clone()));
        }
        for node in &self.nodes {
            match node {
                Node::Rects { rects, color } => p.list.ops.push(PaintOp::PixelRects {
                    rects: rects.clone(),
                    color: *color,
                }),
                Node::Radials { layer, rect, alpha } => {
                    if *alpha < 1. {
                        p.list.ops.push(PaintOp::PushOpacity(*alpha));
                    }
                    p.list.ops.push(PaintOp::PixelRadialLayer {
                        rect: *rect,
                        layer: layer.clone(),
                    });
                    if *alpha < 1. {
                        p.list.ops.push(PaintOp::PopOpacity);
                    }
                }
                Node::Image {
                    source,
                    rect,
                    alpha,
                } => {
                    let image = p.images.image(&source.key, &source.pixels, false)?;
                    p.list.ops.push(PaintOp::SpriteImage {
                        image,
                        origin: [rect.x, rect.y],
                        scale: [
                            rect.width / source.pixels.dim().width as f32,
                            rect.height / source.pixels.dim().height as f32,
                        ],
                        tint: [1., 1., 1., *alpha],
                    });
                }
            }
        }
        Ok(())
    }
}
fn clip(size: Vec2<i32>, rect: Vec4<i32>, inset: i32) -> Option<Rect> {
    let x0 = (rect.x + inset).clamp(0, size.x);
    let y0 = (rect.y + inset).clamp(0, size.y);
    let x1 = (rect.x + rect.z - inset).clamp(0, size.x);
    let y1 = (rect.y + rect.w - inset).clamp(0, size.y);
    (x1 > x0 && y1 > y0)
        .then(|| Rect::new(x0 as f32, y0 as f32, (x1 - x0) as f32, (y1 - y0) as f32))
}
fn outline(rect: Rect, out: &mut Vec<Rect>) {
    out.extend([
        Rect::new(rect.x, rect.y, rect.width, 1.),
        Rect::new(rect.x, rect.y + rect.height - 1., rect.width, 1.),
        Rect::new(rect.x, rect.y, 1., rect.height),
        Rect::new(rect.x + rect.width - 1., rect.y, 1., rect.height),
    ]);
}
impl Board {
    fn rects(&mut self, rects: Vec<Rect>, color: [u8; 4]) {
        if !rects.is_empty() {
            self.nodes.push(Node::Rects {
                rects: rects.into(),
                color: color.map(|v| v as f32 / 255.),
            });
        }
    }
    fn border(&mut self, rect: Option<Rect>, color: [u8; 4]) {
        if let Some(rect) = rect {
            let mut edges = Vec::new();
            outline(rect, &mut edges);
            self.rects(edges, color);
        }
    }
}
impl TilesDock {
    pub(super) fn native_board(&mut self, project: &Project, tab: usize, size: Vec2<i32>) -> Board {
        self.native_images.prune();
        let TileBoardLayout {
            cell,
            width,
            height,
            board_cols,
            offset,
            total_rows,
            visible_rows,
            occupied,
            mut placements,
        } = self.layout_tab(project, tab, size);
        let mut board = Board {
            nodes: Vec::new(),
            error: None,
        };
        board.rects(vec![Rect::new(0., 0., size.x as f32, size.y as f32)], BLACK);
        let mut fills = Vec::new();
        let mut borders = Vec::new();
        let mut selected = Vec::new();
        let first = (offset.y / cell).max(0);
        let end = visible_rows
            .max(total_rows)
            .min(((height + offset.y) / cell + 2).max(0));
        for row in first..end {
            for col in 0..board_cols {
                let pos = Vec2::new(col, row);
                let reserved = self.is_reserved_slot(project, tab, pos);
                if occupied.contains(&(col, row)) && !reserved {
                    continue;
                }
                let rect = Vec4::new(col * cell - offset.x, row * cell - offset.y, cell, cell);
                if rect.x >= width || rect.x + cell <= 0 || rect.y >= height || rect.y + cell <= 0 {
                    continue;
                }
                if let Some(rect) = clip(size, rect, 2) {
                    if !reserved {
                        fills.push(rect);
                        outline(rect, &mut borders);
                    }
                    if self.selected_empty_cell == Some((tab, pos)) {
                        outline(rect, &mut selected);
                    }
                }
            }
        }
        board.rects(fills, [62, 62, 62, 255]);
        board.rects(borders, [74, 74, 74, 255]);
        board.rects(selected, WHITE);
        for placement in &mut placements {
            let rect = Vec4::new(
                placement.rect.x - offset.x,
                placement.rect.y - offset.y,
                placement.rect.z,
                placement.rect.w,
            );
            placement.rect = rect;
            if rect.x >= width || rect.x + rect.z <= 0 || rect.y >= height || rect.y + rect.w <= 0 {
                continue;
            }
            self.native_source(&mut board, project, size, rect, placement.source, 1.);
        }
        if tab == self.active_tab
            && let Some((preview_tab, pos)) = self.drag_drop_cell
            && preview_tab == tab
            && let Some((_, source, _, _)) = self.drag_item
        {
            let span = self.source_span(project, source);
            let rect = Vec4::new(
                pos.x * cell - offset.x,
                pos.y * cell - offset.y,
                span.x * cell,
                span.y * cell,
            );
            board.border(clip(size, rect, 2), [220, 220, 220, 255]);
            self.native_source(&mut board, project, size, rect, source, 0.55);
        }
        self.placements[tab] = placements;
        board
    }
    fn native_source(
        &mut self,
        board: &mut Board,
        project: &Project,
        size: Vec2<i32>,
        rect: Vec4<i32>,
        source: TileSource,
        alpha: f32,
    ) {
        let TileSource::SingleTile(id) = source;
        if let Some(tile) = project.tiles.get(&id)
            && !tile.textures.is_empty()
        {
            let mut full = Vec4::new(
                rect.x + 2,
                rect.y + 2,
                (rect.z - 4).max(1),
                (rect.w - 4).max(1),
            );
            if let Some(emitter) = &tile.particle_emitter {
                if let Some(draw) = clip(size, full, 0) {
                    let frame = crate::docks::particle_preview::prepare_particle_emitter_preview(
                        emitter, full.z, full.w, 0.68,
                    );
                    board.nodes.push(Node::Radials {
                        layer: frame.native_layer(false),
                        rect: draw,
                        alpha,
                    });
                }
            } else {
                let image = {
                    let iso = self.iso_mode && !project.procedural_materials.contains_key(&id);
                    let texture = if iso {
                        tile.isometric
                            .as_ref()
                            .and_then(|s| s.frames.first())
                            .unwrap_or(&tile.textures[0])
                    } else {
                        &tile.textures[0]
                    };
                    let bitmap = tile.isometric.as_ref().is_some_and(|s| s.bitmap);
                    self.native_images
                        .texture(texture, bitmap)
                        .inspect(|image| {
                            if bitmap || (iso && tile.isometric.is_some()) {
                                let dim = image.pixels.dim();
                                let scale = (full.z as f32 / dim.width as f32)
                                    .min(full.w as f32 / dim.height as f32);
                                let w = (dim.width as f32 * scale).round().max(1.) as i32;
                                let h = (dim.height as f32 * scale).round().max(1.) as i32;
                                full.x += (full.z - w) / 2;
                                full.y += (full.w - h) / 2;
                                full.z = w;
                                full.w = h;
                            }
                        })
                };
                match image {
                    Err(error) => {
                        board.error.get_or_insert(error);
                    }
                    Ok(image) => {
                        if let Some(draw) = clip(size, full, 0) {
                            let width = image.pixels.dim().width as usize;
                            let height = image.pixels.dim().height as usize;
                            let u0 = ((draw.x - full.x as f32) / full.z as f32).clamp(0., 1.);
                            let v0 = ((draw.y - full.y as f32) / full.w as f32).clamp(0., 1.);
                            let u1 = ((draw.x + draw.width - full.x as f32) / full.z as f32)
                                .clamp(0., 1.);
                            let v1 = ((draw.y + draw.height - full.y as f32) / full.w as f32)
                                .clamp(0., 1.);
                            let x0 = ((u0 * width as f32).round() as usize).min(width - 1);
                            let y0 = ((v0 * height as f32).round() as usize).min(height - 1);
                            let x1 = ((u1 * width as f32).round() as usize).clamp(x0 + 1, width);
                            let y1 = ((v1 * height as f32).round() as usize).clamp(y0 + 1, height);
                            let source = self.native_images.crop(image, [x0, y0, x1 - x0, y1 - y0]);
                            board.nodes.push(Node::Image {
                                source,
                                rect: draw,
                                alpha,
                            });
                        }
                    }
                }
            }
        }
        let hovered = Some(source) == self.tile_hover_source;
        if hovered || Some(source) == self.current_source() {
            board.border(
                clip(size, rect, 1),
                if hovered { [210, 210, 210, 255] } else { WHITE },
            );
        }
    }
}

/// Populated visual cases in the existing assembled Creator checkpoint. The
/// reference uses the original draw_tab path, not a mirror of GPU paint logic.
#[cfg(test)]
pub(super) fn reference_cases() -> Vec<(&'static str, Arc<dyn RenderViewPaintSource>, TheRGBABuffer)>
{
    let mut project = Project::default();
    let mut ids = Vec::new();
    for index in 0..4 {
        let (width, height) = (16 + index * 3, 20 + index * 2);
        let mut bytes = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let alpha = if index == 2 && (x < 2 || y < 3 || x >= width - 2 || y >= height - 3) {
                    0
                } else {
                    [64, 128, 192, 255][(x + y) % 4]
                };
                bytes.extend([
                    (x * 19 % 256) as u8,
                    (y * 31 % 256) as u8,
                    ((x + y) * 11 % 256) as u8,
                    alpha,
                ]);
            }
        }
        let texture = rusterix::Texture::new(bytes, width, height);
        let mut tile = rusterix::Tile::from_texture(texture.clone());
        tile.alias = format!("Native source {index}");
        if index == 2 {
            tile.isometric = Some(rusterix::isometric::SpriteLayout {
                bitmap: true,
                frames: vec![texture],
                ..Default::default()
            });
        }
        if index == 3 {
            let mut emitter =
                rusterix::ParticleEmitter::new(vek::Vec3::zero(), vek::Vec3::unit_y());
            emitter.rate = 0.;
            emitter.flame_base = true;
            tile.particle_emitter = Some(emitter);
        }
        let id = tile.id;
        ids.push(id);
        project.tiles.insert(id, tile);
        project.set_tile_board_position(TileSource::SingleTile(id), Vec2::new(index as i32, 0));
    }
    project.tile_board_empty_slots.push(Vec2::new(0, 1));
    let mut cases = Vec::new();
    for (label, size, offset, ghost) in [
        ("populated", Vec2::new(230, 140), Vec2::zero(), false),
        ("clipped ghost", Vec2::new(143, 83), Vec2::new(17, 9), true),
        ("particle ghost", Vec2::new(143, 83), Vec2::new(17, 9), true),
    ] {
        let mut dock = <TilesDock as Dock>::new();
        dock.iso_mode = true;
        dock.ensure_tab_state(dock.tab_specs(&project).len());
        dock.tab_offset[0] = offset;
        dock.curr_source = Some(TileSource::SingleTile(ids[0]));
        dock.tile_hover_source = Some(TileSource::SingleTile(ids[1]));
        dock.selected_empty_cell = Some((0, Vec2::new(0, 1)));
        if ghost {
            let ghost_id = if label == "particle ghost" {
                ids[3]
            } else {
                ids[1]
            };
            dock.drag_item = Some((
                0,
                TileSource::SingleTile(ghost_id),
                Vec2::zero(),
                Vec2::zero(),
            ));
            dock.drag_drop_cell = Some((0, Vec2::new(1, 1)));
        }
        let source: Arc<dyn RenderViewPaintSource> = Arc::new(dock.native_board(&project, 0, size));
        let mut reference = TheRGBABuffer::new(TheDim::sized(size.x, size.y));
        reference.fill(BLACK);
        let mut ctx = TheContext::new(size.x as usize, size.y as usize, 1.);
        dock.draw_tab(&mut reference, &mut ctx, &project, 0);
        // RenderView's old copied-framebuffer display ignored stored alpha.
        // Compare the actual opaque presentation, keeping CPU RGB untouched.
        for pixel in reference.pixels_mut().chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        cases.push((label, source, reference));
    }
    cases
}
