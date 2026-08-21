use crate::map::{OrthographicBakeAsset, OrthographicBakeTile};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
use scenevm::{Camera3D, CameraKind};
use std::collections::HashMap;
use uuid::Uuid;
use vek::Vec3;

const BAKE_VERSION: u32 = 2;
const DEFAULT_TILE_SIZE: u32 = 256;
const MAX_TILE_COUNT: usize = 4096;

#[derive(Clone, Copy, Debug)]
pub struct OrthographicBakeWork {
    pub sample: u32,
    pub tile_size: u32,
    pub camera: Camera3D,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrthographicBakeStatus {
    Missing,
    Requested,
    Rendering { sample: u32, total: u32 },
    Ready,
    Failed(String),
}

#[derive(Debug)]
pub struct OrthographicBakeController {
    status: OrthographicBakeStatus,
    visible: bool,
    requested_region: Option<Uuid>,
    requested_samples: u32,
    staging_camera: Option<Camera3D>,
    staging_asset: Option<OrthographicBakeAsset>,
    staging_tiles: HashMap<(i32, i32), Vec<u8>>,
    tile_plan: Vec<(i32, i32)>,
    current_tile: usize,
    current_sample: u32,
    committed_region: Option<Uuid>,
    committed: Option<OrthographicBakeAsset>,
    committed_tiles: HashMap<(i32, i32), Vec<u8>>,
    loaded_signature: Option<(Uuid, u32, usize, usize)>,
    pending_persisted_update: Option<Option<OrthographicBakeAsset>>,
}

impl Default for OrthographicBakeController {
    fn default() -> Self {
        Self {
            status: OrthographicBakeStatus::Missing,
            visible: false,
            requested_region: None,
            requested_samples: 16,
            staging_camera: None,
            staging_asset: None,
            staging_tiles: HashMap::new(),
            tile_plan: Vec::new(),
            current_tile: 0,
            current_sample: 0,
            committed_region: None,
            committed: None,
            committed_tiles: HashMap::new(),
            loaded_signature: None,
            pending_persisted_update: None,
        }
    }
}

impl OrthographicBakeController {
    pub fn request_render(&mut self, region_id: Uuid, samples: u32) {
        self.requested_region = Some(region_id);
        self.requested_samples = samples.clamp(1, 256);
        self.staging_camera = None;
        self.staging_asset = None;
        self.staging_tiles.clear();
        self.tile_plan.clear();
        self.current_tile = 0;
        self.current_sample = 0;
        self.status = OrthographicBakeStatus::Requested;
        self.visible = true;
    }

    pub fn sync_persisted_asset(
        &mut self,
        region_id: Uuid,
        persisted: Option<&OrthographicBakeAsset>,
    ) {
        if self.is_rendering() && self.requested_region == Some(region_id) {
            return;
        }
        let signature = persisted.map(|asset| {
            (
                region_id,
                asset.version,
                asset.tiles.len(),
                asset
                    .tiles
                    .iter()
                    .map(|tile| tile.color_png_base64.len())
                    .sum(),
            )
        });
        if self.loaded_signature == signature && self.committed_region == Some(region_id) {
            return;
        }

        self.loaded_signature = signature;
        self.committed_region = persisted.map(|_| region_id);
        self.committed = persisted.cloned();
        self.committed_tiles = persisted.map(decode_tiles).unwrap_or_default();
        self.status = if self.committed.is_some() {
            OrthographicBakeStatus::Ready
        } else {
            OrthographicBakeStatus::Missing
        };
        self.visible = self.committed.is_some();
    }

    pub fn prepare_work(
        &mut self,
        region_id: Uuid,
        viewport_width: u32,
        viewport_height: u32,
        camera: Camera3D,
        projected_scene_bounds: Option<(f32, f32, f32, f32)>,
    ) -> Result<Option<OrthographicBakeWork>, String> {
        if self.requested_region != Some(region_id) || !self.is_rendering() {
            return Ok(None);
        }
        if !matches!(camera.kind, CameraKind::OrthoIso) {
            return Err("Bake rendering requires an orthographic camera.".into());
        }

        if matches!(self.status, OrthographicBakeStatus::Requested) {
            self.initialize_plan(
                viewport_width,
                viewport_height,
                camera,
                projected_scene_bounds,
            )?;
        } else if !same_orientation(self.staging_camera.as_ref(), &camera) {
            return Err("The orthographic camera orientation changed while baking.".into());
        }

        let Some(asset) = self.staging_asset.as_ref() else {
            return Ok(None);
        };
        let Some(&(tile_x, tile_y)) = self.tile_plan.get(self.current_tile) else {
            return Ok(None);
        };
        let span = asset.tile_size as f32 / asset.pixels_per_world_unit;
        let right = vec3(asset.camera_right);
        let up = vec3(asset.camera_up);
        let [min_u, _, min_v, _] = asset.projected_bounds;
        let mut tile_camera = self.staging_camera.unwrap_or(camera);
        let target_u = min_u + (tile_x as f32 + 0.5) * span;
        let target_v = min_v + (tile_y as f32 + 0.5) * span;
        tile_camera.pos += right * (target_u - tile_camera.pos.dot(right));
        tile_camera.pos += up * (target_v - tile_camera.pos.dot(up));
        tile_camera.ortho_half_h = span * 0.5;

        Ok(Some(OrthographicBakeWork {
            sample: self.current_sample,
            tile_size: asset.tile_size,
            camera: tile_camera,
        }))
    }

    fn initialize_plan(
        &mut self,
        viewport_width: u32,
        viewport_height: u32,
        camera: Camera3D,
        projected_scene_bounds: Option<(f32, f32, f32, f32)>,
    ) -> Result<(), String> {
        let height = viewport_height.max(1) as f32;
        let pixels_per_world_unit =
            (height / (2.0 * camera.ortho_half_h.max(0.0001))).clamp(1.0, 256.0);
        let tile_size = DEFAULT_TILE_SIZE;
        let span = tile_size as f32 / pixels_per_world_unit;
        let right = camera.right.normalized();
        let up = camera.up.normalized();

        let (min_u, max_u, min_v, max_v) = if let Some(bounds) = projected_scene_bounds {
            bounds
        } else {
            let half_h = camera.ortho_half_h.max(0.0001);
            let half_w = half_h * viewport_width.max(1) as f32 / viewport_height.max(1) as f32;
            let center_u = camera.pos.dot(right);
            let center_v = camera.pos.dot(up);
            (
                center_u - half_w,
                center_u + half_w,
                center_v - half_h,
                center_v + half_h,
            )
        };

        let tile_columns = ((max_u - min_u).max(0.0001) / span).ceil().max(1.0) as i32;
        let tile_rows = ((max_v - min_v).max(0.0001) / span).ceil().max(1.0) as i32;
        let mut plan = Vec::new();
        for y in 0..tile_rows {
            for x in 0..tile_columns {
                plan.push((x, y));
            }
        }
        if plan.len() > MAX_TILE_COUNT {
            return Err(format!(
                "The bake would require {} tiles; the current safety limit is {MAX_TILE_COUNT}.",
                plan.len()
            ));
        }
        let center_x = ((camera.pos.dot(right) - min_u) / span).floor() as i32;
        let center_y = ((camera.pos.dot(up) - min_v) / span).floor() as i32;
        plan.sort_by_key(|(x, y)| {
            let dx = i64::from(*x - center_x);
            let dy = i64::from(*y - center_y);
            dx * dx + dy * dy
        });

        self.staging_camera = Some(camera);
        self.staging_asset = Some(OrthographicBakeAsset {
            version: BAKE_VERSION,
            tile_size,
            pixels_per_world_unit,
            projected_bounds: [min_u, max_u, min_v, max_v],
            camera_forward: camera.forward.into_array(),
            camera_right: right.into_array(),
            camera_up: up.into_array(),
            samples: self.requested_samples,
            tiles: Vec::with_capacity(plan.len()),
        });
        self.tile_plan = plan;
        self.current_tile = 0;
        self.current_sample = 0;
        self.update_rendering_status();
        Ok(())
    }

    /// Advance one progressive sample. A true result means the current tile should be captured.
    pub fn finish_sample(&mut self) -> bool {
        if !matches!(self.status, OrthographicBakeStatus::Rendering { .. }) {
            return false;
        }
        if self.current_sample + 1 >= self.requested_samples {
            true
        } else {
            self.current_sample += 1;
            self.update_rendering_status();
            false
        }
    }

    pub fn commit_current_tile(&mut self, rgba: Vec<u8>) -> Result<bool, String> {
        let Some(asset) = self.staging_asset.as_mut() else {
            return Err("The bake has no staging asset.".into());
        };
        let Some(&(x, y)) = self.tile_plan.get(self.current_tile) else {
            return Err("The bake has no current tile.".into());
        };
        let needed = asset.tile_size as usize * asset.tile_size as usize * 4;
        if rgba.len() != needed {
            return Err(format!(
                "The bake returned {} RGBA bytes for a tile, expected {needed}.",
                rgba.len()
            ));
        }
        let png = encode_png(asset.tile_size, asset.tile_size, &rgba)?;
        asset.tiles.push(OrthographicBakeTile {
            x,
            y,
            color_png_base64: BASE64.encode(png),
            depth_base64: None,
        });
        self.staging_tiles.insert((x, y), rgba);
        self.current_tile += 1;
        self.current_sample = 0;

        if self.current_tile >= self.tile_plan.len() {
            let completed = self.staging_asset.take().unwrap();
            self.committed_region = self.requested_region;
            self.committed = Some(completed.clone());
            self.committed_tiles = std::mem::take(&mut self.staging_tiles);
            self.loaded_signature = Some((
                self.committed_region.unwrap(),
                completed.version,
                completed.tiles.len(),
                completed
                    .tiles
                    .iter()
                    .map(|tile| tile.color_png_base64.len())
                    .sum(),
            ));
            self.pending_persisted_update = Some(Some(completed));
            self.status = OrthographicBakeStatus::Ready;
            self.staging_camera = None;
            self.tile_plan.clear();
            Ok(true)
        } else {
            self.update_rendering_status();
            Ok(false)
        }
    }

    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = OrthographicBakeStatus::Failed(error.into());
        self.staging_camera = None;
        self.staging_asset = None;
        self.staging_tiles.clear();
        self.tile_plan.clear();
        self.visible = self.committed.is_some();
    }

    pub fn toggle_visibility(&mut self) -> Option<bool> {
        if self.committed.is_none() && !self.is_rendering() {
            return None;
        }
        self.visible = !self.visible;
        Some(self.visible)
    }

    pub fn clear(&mut self) {
        *self = Self::default();
        self.pending_persisted_update = Some(None);
    }

    pub fn take_persisted_update(&mut self) -> Option<Option<OrthographicBakeAsset>> {
        self.pending_persisted_update.take()
    }

    pub fn status(&self) -> &OrthographicBakeStatus {
        &self.status
    }

    pub fn is_rendering(&self) -> bool {
        matches!(
            self.status,
            OrthographicBakeStatus::Requested | OrthographicBakeStatus::Rendering { .. }
        )
    }

    pub fn can_compose(&self, region_id: Uuid, camera: &Camera3D) -> bool {
        let active_region = if self.is_rendering() {
            self.requested_region
        } else {
            self.committed_region
        };
        if !self.visible || active_region != Some(region_id) {
            return false;
        }
        let (asset, has_tiles) = if self.is_rendering() {
            (self.staging_asset.as_ref(), !self.staging_tiles.is_empty())
        } else {
            (self.committed.as_ref(), !self.committed_tiles.is_empty())
        };
        has_tiles && asset.is_some_and(|asset| asset_matches_camera(asset, camera))
    }

    pub fn compose_rgba(
        &self,
        region_id: Uuid,
        width: u32,
        height: u32,
        camera: &Camera3D,
    ) -> Option<Vec<u8>> {
        let active_region = if self.is_rendering() {
            self.requested_region
        } else {
            self.committed_region
        };
        if !self.visible || active_region != Some(region_id) {
            return None;
        }
        let (asset, tiles) = if self.is_rendering() {
            (self.staging_asset.as_ref()?, &self.staging_tiles)
        } else {
            (self.committed.as_ref()?, &self.committed_tiles)
        };
        if !asset_matches_camera(asset, camera) || tiles.is_empty() {
            return None;
        }

        let width = width.max(1);
        let height = height.max(1);
        let mut output = vec![0u8; width as usize * height as usize * 4];
        for pixel in output.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        let right = vec3(asset.camera_right);
        let up = vec3(asset.camera_up);
        let ppu = asset.pixels_per_world_unit.max(0.0001);
        let span = asset.tile_size as f32 / ppu;
        let [min_u, max_u, min_v, max_v] = asset.projected_bounds;
        let has_exact_bounds = asset.version >= 2 && max_u > min_u && max_v > min_v;
        let origin_u = if has_exact_bounds { min_u } else { 0.0 };
        let origin_v = if has_exact_bounds { min_v } else { 0.0 };
        let half_h = camera.ortho_half_h.max(0.0001);
        let half_w = half_h * width as f32 / height as f32;
        let center_u = camera.pos.dot(right);
        let center_v = camera.pos.dot(up);

        for y in 0..height {
            let v = center_v + (1.0 - (y as f32 + 0.5) * 2.0 / height as f32) * half_h;
            if has_exact_bounds && (v < min_v || v > max_v) {
                continue;
            }
            let tile_y = ((v - origin_v) / span).floor() as i32;
            let local_y = ((origin_v + (tile_y + 1) as f32 * span - v) * ppu)
                .floor()
                .clamp(0.0, asset.tile_size.saturating_sub(1) as f32)
                as u32;
            let mut current_tile_x = i32::MIN;
            let mut current_tile_data: Option<&Vec<u8>> = None;
            for x in 0..width {
                let u = center_u + ((x as f32 + 0.5) * 2.0 / width as f32 - 1.0) * half_w;
                if has_exact_bounds && (u < min_u || u > max_u) {
                    continue;
                }
                let tile_x = ((u - origin_u) / span).floor() as i32;
                if tile_x != current_tile_x {
                    current_tile_x = tile_x;
                    current_tile_data = tiles.get(&(tile_x, tile_y));
                }
                let Some(tile) = current_tile_data else {
                    continue;
                };
                let local_x = ((u - (origin_u + tile_x as f32 * span)) * ppu)
                    .floor()
                    .clamp(0.0, asset.tile_size.saturating_sub(1) as f32)
                    as u32;
                let src = (local_y as usize * asset.tile_size as usize + local_x as usize) * 4;
                let dst = (y as usize * width as usize + x as usize) * 4;
                output[dst..dst + 4].copy_from_slice(&tile[src..src + 4]);
            }
        }
        Some(output)
    }

    pub fn progress_text(&self) -> Option<String> {
        match &self.status {
            OrthographicBakeStatus::Requested => Some("Preparing tiled orthographic bake…".into()),
            OrthographicBakeStatus::Rendering { sample, total } => {
                Some(format!("Bake work {}/{}", (sample + 1).min(*total), total))
            }
            _ => None,
        }
    }

    fn update_rendering_status(&mut self) {
        let total = (self.tile_plan.len() as u32).saturating_mul(self.requested_samples);
        let sample = (self.current_tile as u32)
            .saturating_mul(self.requested_samples)
            .saturating_add(self.current_sample);
        self.status = OrthographicBakeStatus::Rendering { sample, total };
    }
}

fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(rgba, width, height, ExtendedColorType::Rgba8)
        .map_err(|error| format!("Could not compress bake tile: {error}"))?;
    Ok(bytes)
}

fn decode_tiles(asset: &OrthographicBakeAsset) -> HashMap<(i32, i32), Vec<u8>> {
    asset
        .tiles
        .iter()
        .filter_map(|tile| {
            let encoded = BASE64.decode(&tile.color_png_base64).ok()?;
            let decoded = image::load_from_memory(&encoded).ok()?.into_rgba8();
            if decoded.width() != asset.tile_size || decoded.height() != asset.tile_size {
                return None;
            }
            Some(((tile.x, tile.y), decoded.into_raw()))
        })
        .collect()
}

fn asset_matches_camera(asset: &OrthographicBakeAsset, camera: &Camera3D) -> bool {
    matches!(camera.kind, CameraKind::OrthoIso)
        && approx3(vec3(asset.camera_forward), camera.forward.normalized())
        && approx3(vec3(asset.camera_right), camera.right.normalized())
        && approx3(vec3(asset.camera_up), camera.up.normalized())
}

fn same_orientation(snapshot: Option<&Camera3D>, current: &Camera3D) -> bool {
    snapshot.is_some_and(|snapshot| {
        matches!(snapshot.kind, CameraKind::OrthoIso)
            && matches!(current.kind, CameraKind::OrthoIso)
            && approx3(snapshot.forward.normalized(), current.forward.normalized())
            && approx3(snapshot.right.normalized(), current.right.normalized())
            && approx3(snapshot.up.normalized(), current.up.normalized())
    })
}

fn vec3(value: [f32; 3]) -> Vec3<f32> {
    Vec3::new(value[0], value[1], value[2])
}

fn approx3(a: Vec3<f32>, b: Vec3<f32>) -> bool {
    (a - b).magnitude_squared() <= 1e-6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_binary_is_png_base64_serializable() {
        let rgba = vec![127; 4 * 4 * 4];
        let encoded = BASE64.encode(encode_png(4, 4, &rgba).unwrap());
        let tile = OrthographicBakeTile {
            x: 0,
            y: 0,
            color_png_base64: encoded,
            depth_base64: None,
        };
        let json = serde_json::to_string(&tile).unwrap();
        let restored: OrthographicBakeTile = serde_json::from_str(&json).unwrap();
        let bytes = BASE64.decode(restored.color_png_base64).unwrap();
        assert_eq!(
            image::load_from_memory(&bytes).unwrap().to_rgba8().as_raw(),
            &rgba
        );
    }

    #[test]
    fn persisted_tile_composes_after_pan_and_resize() {
        let region = Uuid::new_v4();
        let mut camera = Camera3D::iso();
        camera.ortho_half_h = 0.75;
        camera.pos = camera.right * 2.0 + camera.up * 2.0;
        let rgba = vec![200; 4 * 4 * 4];
        let asset = OrthographicBakeAsset {
            version: BAKE_VERSION,
            tile_size: 4,
            pixels_per_world_unit: 1.0,
            projected_bounds: [0.0, 4.0, 0.0, 4.0],
            camera_forward: camera.forward.into_array(),
            camera_right: camera.right.into_array(),
            camera_up: camera.up.into_array(),
            samples: 1,
            tiles: vec![OrthographicBakeTile {
                x: 0,
                y: 0,
                color_png_base64: BASE64.encode(encode_png(4, 4, &rgba).unwrap()),
                depth_base64: None,
            }],
        };
        let mut bake = OrthographicBakeController::default();
        bake.sync_persisted_asset(region, Some(&asset));

        let first = bake.compose_rgba(region, 3, 2, &camera).unwrap();
        camera.pos += camera.right * 0.25;
        let panned = bake.compose_rgba(region, 2, 3, &camera).unwrap();
        assert!(first.chunks_exact(4).all(|pixel| pixel == [200; 4]));
        assert!(panned.chunks_exact(4).all(|pixel| pixel == [200; 4]));

        let mut map = crate::Map::new();
        map.orthographic_bake = Some(asset);
        let json = serde_json::to_string(&map).unwrap();
        let restored: crate::Map = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.orthographic_bake.unwrap().tiles.len(), 1);
    }

    #[test]
    fn planner_finishes_all_world_tiles_and_emits_persisted_asset() {
        let region = Uuid::new_v4();
        let camera = Camera3D::iso();
        let mut bake = OrthographicBakeController::default();
        bake.request_render(region, 1);
        let bounds = Some((0.0, 1.0, 0.0, 1.0));

        while bake.is_rendering() {
            let work = bake
                .prepare_work(region, 64, 64, camera, bounds)
                .unwrap()
                .unwrap();
            assert!(bake.finish_sample());
            bake.commit_current_tile(vec![
                64;
                work.tile_size as usize * work.tile_size as usize * 4
            ])
            .unwrap();
        }

        assert!(matches!(bake.status(), OrthographicBakeStatus::Ready));
        let persisted = bake.take_persisted_update().unwrap().unwrap();
        assert_eq!(persisted.tiles.len(), 1);
        assert_eq!(persisted.projected_bounds, [0.0, 1.0, 0.0, 1.0]);
        assert!(
            persisted
                .tiles
                .iter()
                .all(|tile| !tile.color_png_base64.is_empty())
        );
    }

    #[test]
    fn replacement_bake_does_not_preview_old_committed_tiles() {
        let region = Uuid::new_v4();
        let mut camera = Camera3D::iso();
        camera.ortho_half_h = 2.0;
        let rgba = vec![180; 4 * 4 * 4];
        let asset = OrthographicBakeAsset {
            version: BAKE_VERSION,
            tile_size: 4,
            pixels_per_world_unit: 1.0,
            projected_bounds: [-2.0, 2.0, -2.0, 2.0],
            camera_forward: camera.forward.into_array(),
            camera_right: camera.right.into_array(),
            camera_up: camera.up.into_array(),
            samples: 1,
            tiles: vec![OrthographicBakeTile {
                x: 0,
                y: 0,
                color_png_base64: BASE64.encode(encode_png(4, 4, &rgba).unwrap()),
                depth_base64: None,
            }],
        };
        let mut bake = OrthographicBakeController::default();
        bake.sync_persisted_asset(region, Some(&asset));
        assert!(bake.compose_rgba(region, 2, 2, &camera).is_some());

        bake.request_render(region, 1);
        bake.prepare_work(region, 4, 4, camera, Some((-2.0, 2.0, -2.0, 2.0)))
            .unwrap()
            .unwrap();
        assert!(bake.compose_rgba(region, 2, 2, &camera).is_none());
    }
}
