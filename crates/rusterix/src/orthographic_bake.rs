use crate::map::{OrthographicBakeAsset, OrthographicBakeTile};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
use scenevm::{Camera3D, CameraKind};
use std::collections::HashMap;
use uuid::Uuid;
use vek::Vec3;

const BAKE_VERSION: u32 = 4;
const DEFAULT_TILE_SIZE: u32 = 256;
const MAX_TILE_COUNT: usize = 4096;

#[derive(Clone, Copy, Debug)]
pub struct OrthographicBakeLighting {
    pub sun_direction: Vec3<f32>,
    pub sun_color: Vec3<f32>,
    pub sun_intensity: f32,
    pub sun_enabled: bool,
}

#[derive(Clone, Debug, Default)]
struct DecodedBakeTile {
    color: Vec<u8>,
    depth: Vec<f32>,
    albedo: Vec<u8>,
    normal: Vec<u8>,
    material: Vec<u8>,
}

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
    staging_tiles: HashMap<(i32, i32), DecodedBakeTile>,
    tile_plan: Vec<(i32, i32)>,
    current_tile: usize,
    current_sample: u32,
    committed_region: Option<Uuid>,
    committed: Option<OrthographicBakeAsset>,
    committed_tiles: HashMap<(i32, i32), DecodedBakeTile>,
    loaded_signature: Option<(Uuid, u32, usize, usize)>,
    pending_persisted_update: Option<Option<OrthographicBakeAsset>>,
}

impl Default for OrthographicBakeController {
    fn default() -> Self {
        Self {
            status: OrthographicBakeStatus::Missing,
            visible: false,
            requested_region: None,
            requested_samples: 32,
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
                    .map(|tile| {
                        tile.color_png_base64.len()
                            + tile.depth_base64.as_ref().map_or(0, String::len)
                            + tile.albedo_png_base64.as_ref().map_or(0, String::len)
                            + tile.normal_png_base64.as_ref().map_or(0, String::len)
                            + tile.material_png_base64.as_ref().map_or(0, String::len)
                    })
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
            camera_forward_origin: camera.pos.dot(camera.forward.normalized()),
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

    /// Publish the current accumulated beauty sample to the staging preview.
    /// It remains transient until the tile's surface channels are committed.
    pub fn preview_current_tile(&mut self, rgba: &[u8]) -> Result<(), String> {
        let Some(asset) = self.staging_asset.as_ref() else {
            return Err("The bake has no staging asset.".into());
        };
        let Some(&(x, y)) = self.tile_plan.get(self.current_tile) else {
            return Err("The bake has no current tile.".into());
        };
        let needed = asset.tile_size as usize * asset.tile_size as usize * 4;
        if rgba.len() != needed {
            return Err("The progressive bake preview has an invalid tile size.".into());
        }
        self.staging_tiles.entry((x, y)).or_default().color = rgba.to_vec();
        Ok(())
    }

    pub fn commit_current_tile(
        &mut self,
        rgba: Vec<u8>,
        depth: Vec<f32>,
        albedo: Vec<u8>,
        normal: Vec<u8>,
        material: Vec<u8>,
    ) -> Result<bool, String> {
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
        if depth.len() != needed / 4
            || albedo.len() != needed
            || normal.len() != needed
            || material.len() != needed
        {
            return Err("The bake returned an incomplete surface-data tile.".into());
        }
        let png = encode_png(asset.tile_size, asset.tile_size, &rgba)?;
        let albedo_png = encode_png(asset.tile_size, asset.tile_size, &albedo)?;
        let normal_png = encode_png(asset.tile_size, asset.tile_size, &normal)?;
        let material_png = encode_png(asset.tile_size, asset.tile_size, &material)?;
        asset.tiles.push(OrthographicBakeTile {
            x,
            y,
            color_png_base64: BASE64.encode(png),
            depth_base64: Some(BASE64.encode(encode_depth(&depth))),
            albedo_png_base64: Some(BASE64.encode(albedo_png)),
            normal_png_base64: Some(BASE64.encode(normal_png)),
            material_png_base64: Some(BASE64.encode(material_png)),
        });
        self.staging_tiles.insert(
            (x, y),
            DecodedBakeTile {
                color: rgba,
                depth,
                albedo,
                normal,
                material,
            },
        );
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
                    .map(|tile| {
                        tile.color_png_base64.len()
                            + tile.depth_base64.as_ref().map_or(0, String::len)
                            + tile.albedo_png_base64.as_ref().map_or(0, String::len)
                            + tile.normal_png_base64.as_ref().map_or(0, String::len)
                            + tile.material_png_base64.as_ref().map_or(0, String::len)
                    })
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
        self.compose_rgba_lit(region_id, width, height, camera, None)
    }

    pub fn compose_rgba_lit(
        &self,
        region_id: Uuid,
        width: u32,
        height: u32,
        camera: &Camera3D,
        lighting: Option<OrthographicBakeLighting>,
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
            let mut current_tile_data: Option<&DecodedBakeTile> = None;
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
                output[dst..dst + 4]
                    .copy_from_slice(&shade_baked_pixel(tile, src, camera, lighting));
            }
        }
        Some(output)
    }

    pub fn compose_depth(
        &self,
        region_id: Uuid,
        width: u32,
        height: u32,
        camera: &Camera3D,
    ) -> Option<Vec<f32>> {
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

        // Version-three tiles stored camera-linear depth but did not store the bake camera's
        // forward-axis origin. Comparing that directly with a live camera can hide every
        // character. Keep old bakes visible without occlusion; rebaking upgrades them.
        if asset.version < 4 {
            return None;
        }

        let width = width.max(1);
        let height = height.max(1);
        let mut output = vec![f32::INFINITY; width as usize * height as usize];
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
        let depth_rebase =
            asset.camera_forward_origin - camera.pos.dot(vec3(asset.camera_forward).normalized());

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
            for x in 0..width {
                let u = center_u + ((x as f32 + 0.5) * 2.0 / width as f32 - 1.0) * half_w;
                if has_exact_bounds && (u < min_u || u > max_u) {
                    continue;
                }
                let tile_x = ((u - origin_u) / span).floor() as i32;
                let Some(tile) = tiles.get(&(tile_x, tile_y)) else {
                    continue;
                };
                let local_x = ((u - (origin_u + tile_x as f32 * span)) * ppu)
                    .floor()
                    .clamp(0.0, asset.tile_size.saturating_sub(1) as f32)
                    as u32;
                let src = local_y as usize * asset.tile_size as usize + local_x as usize;
                if let Some(depth) = tile.depth.get(src).copied().filter(|v| v.is_finite()) {
                    output[y as usize * width as usize + x as usize] = depth + depth_rebase;
                }
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

fn shade_baked_pixel(
    tile: &DecodedBakeTile,
    src: usize,
    camera: &Camera3D,
    lighting: Option<OrthographicBakeLighting>,
) -> [u8; 4] {
    let fallback = tile
        .color
        .get(src..src + 4)
        .and_then(|pixel| pixel.try_into().ok())
        .unwrap_or([0, 0, 0, 255]);
    let Some(lighting) = lighting.filter(|lighting| lighting.sun_enabled) else {
        return fallback;
    };
    if tile.albedo.len() < src + 4 || tile.normal.len() < src + 4 || tile.material.len() < src + 4 {
        return fallback;
    }
    if tile.albedo[src + 3] == 0 {
        return fallback;
    }

    let decode = |bytes: &[u8]| {
        Vec3::new(
            bytes[src] as f32 / 255.0,
            bytes[src + 1] as f32 / 255.0,
            bytes[src + 2] as f32 / 255.0,
        )
    };
    let encoded = decode(&tile.color);
    let mapped = Vec3::new(
        encoded.x.powf(2.2),
        encoded.y.powf(2.2),
        encoded.z.powf(2.2),
    );
    let indirect = Vec3::new(
        mapped.x / (1.0 - mapped.x).max(1.0e-4),
        mapped.y / (1.0 - mapped.y).max(1.0e-4),
        mapped.z / (1.0 - mapped.z).max(1.0e-4),
    );
    let albedo = decode(&tile.albedo);
    let normal_encoded = decode(&tile.normal);
    let normal_raw = normal_encoded * 2.0 - Vec3::broadcast(1.0);
    let normal = if normal_raw.magnitude_squared() > 1.0e-6 {
        normal_raw.normalized()
    } else {
        Vec3::unit_y()
    };
    let roughness = (tile.material[src] as f32 / 255.0).clamp(0.04, 1.0);
    let metallic = (tile.material[src + 1] as f32 / 255.0).clamp(0.0, 1.0);
    let light = -lighting.sun_direction.normalized();
    let view = -camera.forward.normalized();
    let ndotl = normal.dot(light).max(0.0);
    let ndotv = normal.dot(view).max(0.0);

    let mut direct = Vec3::zero();
    if ndotl > 0.0 && ndotv > 0.0 {
        let half_raw = view + light;
        let half = if half_raw.magnitude_squared() > 1.0e-6 {
            half_raw.normalized()
        } else {
            normal
        };
        let ndoth = normal.dot(half).max(0.0);
        let vdoth = view.dot(half).max(0.0);
        let alpha = roughness * roughness;
        let alpha2 = alpha * alpha;
        let denom = ndoth * ndoth * (alpha2 - 1.0) + 1.0;
        let distribution = alpha2 / (std::f32::consts::PI * denom * denom + 1.0e-6);
        let k = (roughness + 1.0).powi(2) / 8.0;
        let geometry_v = ndotv / (ndotv * (1.0 - k) + k + 1.0e-6);
        let geometry_l = ndotl / (ndotl * (1.0 - k) + k + 1.0e-6);
        let f0 = Vec3::broadcast(0.04) * (1.0 - metallic) + albedo * metallic;
        let fresnel_weight = (1.0 - vdoth).powi(5);
        let fresnel = f0 + (Vec3::broadcast(1.0) - f0) * fresnel_weight;
        let specular =
            fresnel * (distribution * geometry_v * geometry_l / (4.0 * ndotv * ndotl + 1.0e-6));
        let diffuse = albedo * ((1.0 - metallic) / std::f32::consts::PI);
        let brdf = diffuse + specular;
        direct = Vec3::new(
            brdf.x * lighting.sun_color.x,
            brdf.y * lighting.sun_color.y,
            brdf.z * lighting.sun_color.z,
        ) * (lighting.sun_intensity.max(0.0) * ndotl);
    }

    let linear = indirect + direct;
    let mapped = Vec3::new(
        linear.x / (linear.x + 1.0),
        linear.y / (linear.y + 1.0),
        linear.z / (linear.z + 1.0),
    );
    [
        (mapped.x.powf(1.0 / 2.2) * 255.0).round().clamp(0.0, 255.0) as u8,
        (mapped.y.powf(1.0 / 2.2) * 255.0).round().clamp(0.0, 255.0) as u8,
        (mapped.z.powf(1.0 / 2.2) * 255.0).round().clamp(0.0, 255.0) as u8,
        fallback[3],
    ]
}

fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(rgba, width, height, ExtendedColorType::Rgba8)
        .map_err(|error| format!("Could not compress bake tile: {error}"))?;
    Ok(bytes)
}

fn encode_depth(depth: &[f32]) -> Vec<u8> {
    depth.iter().flat_map(|value| value.to_le_bytes()).collect()
}

fn decode_depth(encoded: &str, pixel_count: usize) -> Option<Vec<f32>> {
    let bytes = BASE64.decode(encoded).ok()?;
    if bytes.len() != pixel_count * 4 {
        return None;
    }
    Some(
        bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect(),
    )
}

fn decode_png(encoded: &str, tile_size: u32) -> Option<Vec<u8>> {
    let bytes = BASE64.decode(encoded).ok()?;
    let decoded = image::load_from_memory(&bytes).ok()?.into_rgba8();
    (decoded.width() == tile_size && decoded.height() == tile_size).then(|| decoded.into_raw())
}

fn decode_tiles(asset: &OrthographicBakeAsset) -> HashMap<(i32, i32), DecodedBakeTile> {
    asset
        .tiles
        .iter()
        .filter_map(|tile| {
            let pixel_count = asset.tile_size as usize * asset.tile_size as usize;
            let color = decode_png(&tile.color_png_base64, asset.tile_size)?;
            let depth = tile
                .depth_base64
                .as_deref()
                .and_then(|encoded| decode_depth(encoded, pixel_count))
                .unwrap_or_default();
            let albedo = tile
                .albedo_png_base64
                .as_deref()
                .and_then(|encoded| decode_png(encoded, asset.tile_size))
                .unwrap_or_default();
            let normal = tile
                .normal_png_base64
                .as_deref()
                .and_then(|encoded| decode_png(encoded, asset.tile_size))
                .unwrap_or_default();
            let material = tile
                .material_png_base64
                .as_deref()
                .and_then(|encoded| decode_png(encoded, asset.tile_size))
                .unwrap_or_default();
            Some((
                (tile.x, tile.y),
                DecodedBakeTile {
                    color,
                    depth,
                    albedo,
                    normal,
                    material,
                },
            ))
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
    fn default_bake_uses_32_samples() {
        assert_eq!(OrthographicBakeController::default().requested_samples, 32);
    }

    #[test]
    fn tile_binary_is_png_base64_serializable() {
        let rgba = vec![127; 4 * 4 * 4];
        let encoded = BASE64.encode(encode_png(4, 4, &rgba).unwrap());
        let tile = OrthographicBakeTile {
            x: 0,
            y: 0,
            color_png_base64: encoded,
            depth_base64: None,
            albedo_png_base64: None,
            normal_png_base64: None,
            material_png_base64: None,
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
    fn depth_binary_round_trips_camera_linear_values() {
        let depth = vec![0.25, 2.5, f32::INFINITY, 91.125];
        let encoded = BASE64.encode(encode_depth(&depth));
        assert_eq!(decode_depth(&encoded, depth.len()).unwrap(), depth);
    }

    #[test]
    fn composed_depth_is_rebased_to_the_live_camera() {
        let region = Uuid::new_v4();
        let mut camera = Camera3D::iso();
        camera.ortho_half_h = 0.5;
        let bake_origin = camera.pos.dot(camera.forward.normalized());
        let rgba = vec![255, 255, 255, 255];
        let asset = OrthographicBakeAsset {
            version: BAKE_VERSION,
            tile_size: 1,
            pixels_per_world_unit: 1.0,
            projected_bounds: [-0.5, 0.5, -0.5, 0.5],
            camera_forward: camera.forward.into_array(),
            camera_right: camera.right.into_array(),
            camera_up: camera.up.into_array(),
            camera_forward_origin: bake_origin,
            samples: 1,
            tiles: vec![OrthographicBakeTile {
                x: 0,
                y: 0,
                color_png_base64: BASE64.encode(encode_png(1, 1, &rgba).unwrap()),
                depth_base64: Some(BASE64.encode(encode_depth(&[2.0]))),
                albedo_png_base64: None,
                normal_png_base64: None,
                material_png_base64: None,
            }],
        };
        let mut bake = OrthographicBakeController::default();
        bake.sync_persisted_asset(region, Some(&asset));

        assert_eq!(bake.compose_depth(region, 1, 1, &camera).unwrap(), [2.0]);
        camera.pos -= camera.forward.normalized() * 3.0;
        let moved_depth = bake.compose_depth(region, 1, 1, &camera).unwrap()[0];
        assert!((moved_depth - 5.0).abs() < 1.0e-5);

        let mut legacy = asset;
        legacy.version = 3;
        bake.sync_persisted_asset(region, Some(&legacy));
        assert!(bake.compose_rgba(region, 1, 1, &camera).is_some());
        assert!(bake.compose_depth(region, 1, 1, &camera).is_none());
    }

    #[test]
    fn current_sun_is_resolved_after_the_bake() {
        let tile = DecodedBakeTile {
            color: vec![0, 0, 0, 255],
            depth: vec![1.0],
            albedo: vec![255, 255, 255, 255],
            normal: vec![128, 255, 128, 255],
            material: vec![255, 0, 255, 0],
        };
        let camera = Camera3D::iso();
        let dark = shade_baked_pixel(&tile, 0, &camera, None);
        let lit = shade_baked_pixel(
            &tile,
            0,
            &camera,
            Some(OrthographicBakeLighting {
                sun_direction: Vec3::new(0.0, -1.0, 0.0),
                sun_color: Vec3::broadcast(1.0),
                sun_intensity: 2.0,
                sun_enabled: true,
            }),
        );
        assert_eq!(dark, [0, 0, 0, 255]);
        assert!(lit[0] > dark[0] && lit[1] > dark[1] && lit[2] > dark[2]);
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
            camera_forward_origin: camera.pos.dot(camera.forward.normalized()),
            samples: 1,
            tiles: vec![OrthographicBakeTile {
                x: 0,
                y: 0,
                color_png_base64: BASE64.encode(encode_png(4, 4, &rgba).unwrap()),
                depth_base64: None,
                albedo_png_base64: None,
                normal_png_base64: None,
                material_png_base64: None,
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
            let pixels = work.tile_size as usize * work.tile_size as usize;
            bake.commit_current_tile(
                vec![64; pixels * 4],
                vec![1.0; pixels],
                vec![64; pixels * 4],
                vec![127; pixels * 4],
                vec![64; pixels * 4],
            )
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
            camera_forward_origin: camera.pos.dot(camera.forward.normalized()),
            samples: 1,
            tiles: vec![OrthographicBakeTile {
                x: 0,
                y: 0,
                color_png_base64: BASE64.encode(encode_png(4, 4, &rgba).unwrap()),
                depth_base64: None,
                albedo_png_base64: None,
                normal_png_base64: None,
                material_png_base64: None,
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
