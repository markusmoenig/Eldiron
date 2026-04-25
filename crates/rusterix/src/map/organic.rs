use crate::PixelSource;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vek::Vec2;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicChannelBinding {
    pub channel: i32,
    pub name: String,
    pub source: Option<PixelSource>,
    pub roughness: f32,
    pub metallic: f32,
    pub opacity: f32,
    pub emissive: f32,
}

impl OrganicChannelBinding {
    pub fn defaults() -> Vec<Self> {
        vec![
            Self {
                channel: 0,
                name: "Foliage".to_string(),
                source: None,
                roughness: 0.5,
                metallic: 0.0,
                opacity: 1.0,
                emissive: 0.0,
            },
            Self {
                channel: 1,
                name: "Soil".to_string(),
                source: None,
                roughness: 0.5,
                metallic: 0.0,
                opacity: 1.0,
                emissive: 0.0,
            },
            Self {
                channel: 2,
                name: "Stone".to_string(),
                source: None,
                roughness: 0.5,
                metallic: 0.0,
                opacity: 1.0,
                emissive: 0.0,
            },
            Self {
                channel: 3,
                name: "Accent".to_string(),
                source: None,
                roughness: 0.5,
                metallic: 0.0,
                opacity: 1.0,
                emissive: 0.0,
            },
        ]
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicSpan {
    pub channel: i32,
    pub source: Option<PixelSource>,
    pub offset: f32,
    pub height: f32,
    pub density: f32,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicColumn {
    pub x: i32,
    pub y: i32,
    pub spans: Vec<OrganicSpan>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicVineStroke {
    pub stroke_id: i32,
    pub seq: i32,
    pub start: Vec2<f32>,
    pub end: Vec2<f32>,
    pub anchor_offset: f32,
    pub width: f32,
    pub depth: f32,
    pub channel: i32,
    pub source: Option<PixelSource>,
    pub grow_positive: bool,
    pub cap_start: bool,
    pub cap_end: bool,
}

pub fn default_organic_vine_strokes() -> Vec<OrganicVineStroke> {
    Vec::new()
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum OrganicGrowthShape {
    Bush,
    Tree,
}

fn default_growth_shape() -> OrganicGrowthShape {
    OrganicGrowthShape::Bush
}

fn default_growth_lobes() -> i32 {
    0
}

fn default_growth_spread() -> f32 {
    0.0
}

fn default_growth_trunk_height() -> f32 {
    0.0
}

fn default_growth_trunk_radius() -> f32 {
    0.0
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicBushCluster {
    pub center: Vec2<f32>,
    pub anchor_offset: f32,
    pub radius: f32,
    pub height: f32,
    pub layers: i32,
    pub taper: f32,
    pub breakup: f32,
    pub channel: i32,
    pub source: Option<PixelSource>,
    pub grow_positive: bool,
    #[serde(default = "default_growth_shape")]
    pub shape: OrganicGrowthShape,
    #[serde(default = "default_growth_lobes")]
    pub canopy_lobes: i32,
    #[serde(default = "default_growth_spread")]
    pub canopy_spread: f32,
    #[serde(default = "default_growth_trunk_height")]
    pub trunk_height: f32,
    #[serde(default = "default_growth_trunk_radius")]
    pub trunk_radius: f32,
}

pub fn default_organic_bush_clusters() -> Vec<OrganicBushCluster> {
    Vec::new()
}

pub fn terrain_organic_detail_id(tile_x: i32, tile_y: i32) -> Uuid {
    let mut bytes = [0u8; 16];
    bytes[..4].copy_from_slice(b"trgn");
    bytes[4..8].copy_from_slice(&tile_x.to_le_bytes());
    bytes[8..12].copy_from_slice(&tile_y.to_le_bytes());
    Uuid::from_bytes(bytes)
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicVolumeLayer {
    pub id: Uuid,
    pub name: String,
    pub cell_size: f32,
    pub page_size: i32,
    pub pages: IndexMap<i64, OrganicDetailPage>,
    pub channel_bindings: Vec<OrganicChannelBinding>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicDetailCell {
    pub palette_index: u8,
    pub coverage: u8,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicDetailPage {
    pub page_x: i32,
    pub page_y: i32,
    pub cells: Vec<OrganicDetailCell>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrganicBatchDetail {
    pub anchor_uv: Vec2<f32>,
    pub flip_x: bool,
    pub layers: Vec<OrganicVolumeLayer>,
}

impl Default for OrganicVolumeLayer {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Main Organic Layer".to_string(),
            cell_size: 0.05,
            page_size: 16,
            pages: IndexMap::default(),
            channel_bindings: OrganicChannelBinding::defaults(),
        }
    }
}

impl OrganicVolumeLayer {
    fn page_key(page_x: i32, page_y: i32) -> i64 {
        ((page_x as i64) << 32) ^ (page_y as u32 as i64)
    }

    fn palette_index_from_source(source: Option<PixelSource>) -> Option<u8> {
        match source {
            Some(PixelSource::PaletteIndex(index)) => u8::try_from(index).ok(),
            _ => None,
        }
    }

    fn local_to_cell(&self, local: Vec2<f32>) -> (i32, i32) {
        let inv = 1.0 / self.cell_size.max(0.01);
        (
            (local.x * inv).floor() as i32,
            (local.y * inv).floor() as i32,
        )
    }

    fn cell_center(&self, cell_x: i32, cell_y: i32) -> Vec2<f32> {
        Vec2::new(
            (cell_x as f32 + 0.5) * self.cell_size,
            (cell_y as f32 + 0.5) * self.cell_size,
        )
    }

    fn page_slot(&self, cell_x: i32, cell_y: i32) -> (i32, i32, usize) {
        let page_size = self.page_size.max(1);
        let page_x = cell_x.div_euclid(page_size);
        let page_y = cell_y.div_euclid(page_size);
        let local_x = cell_x.rem_euclid(page_size) as usize;
        let local_y = cell_y.rem_euclid(page_size) as usize;
        let slot = local_y * page_size as usize + local_x;
        (page_x, page_y, slot)
    }

    fn page_mut(&mut self, page_x: i32, page_y: i32) -> &mut OrganicDetailPage {
        let key = Self::page_key(page_x, page_y);
        let page_size = self.page_size.max(1);
        self.pages.entry(key).or_insert_with(|| {
            let len = (page_size * page_size) as usize;
            OrganicDetailPage {
                page_x,
                page_y,
                cells: vec![
                    OrganicDetailCell {
                        palette_index: 0,
                        coverage: 0,
                    };
                    len
                ],
            }
        })
    }

    fn cleanup_page_if_empty(&mut self, page_x: i32, page_y: i32) {
        let key = Self::page_key(page_x, page_y);
        let remove = self
            .pages
            .get(&key)
            .map(|page| page.cells.iter().all(|cell| cell.coverage == 0))
            .unwrap_or(false);
        if remove {
            self.pages.shift_remove(&key);
        }
    }

    fn blend_cell(
        &mut self,
        cell_x: i32,
        cell_y: i32,
        palette_index: u8,
        amount: u8,
        erase: bool,
    ) -> bool {
        if amount == 0 {
            return false;
        }
        let (page_x, page_y, slot) = self.page_slot(cell_x, cell_y);
        let page = self.page_mut(page_x, page_y);
        let cell = &mut page.cells[slot];
        let before = cell.clone();
        if erase {
            cell.coverage = cell.coverage.saturating_sub(amount);
            if cell.coverage == 0 {
                cell.palette_index = 0;
            }
        } else {
            cell.coverage = cell.coverage.saturating_add(amount);
            if amount > 0 {
                cell.palette_index = palette_index;
            }
        }
        let changed = *cell != before;
        if erase {
            self.cleanup_page_if_empty(page_x, page_y);
        }
        changed
    }

    fn radial_mask(radius: f32, softness: f32, dist: f32) -> f32 {
        if radius <= 0.0001 || dist >= radius {
            return 0.0;
        }
        let feather = softness.clamp(0.0, 1.0) * 0.85 + 0.05;
        let inner = radius * (1.0 - feather);
        if dist <= inner {
            1.0
        } else {
            1.0 - ((dist - inner) / (radius - inner).max(0.0001))
        }
    }

    fn apply_blob(
        &mut self,
        center: Vec2<f32>,
        radius: f32,
        softness: f32,
        density: f32,
        palette_index: u8,
        erase: bool,
    ) -> bool {
        let radius = radius.max(self.cell_size * 0.5);
        let density = density.clamp(0.0, 1.0);
        let min = center - Vec2::broadcast(radius);
        let max = center + Vec2::broadcast(radius);
        let (min_x, min_y) = self.local_to_cell(min);
        let (max_x, max_y) = self.local_to_cell(max);
        let mut changed = false;
        for cell_y in min_y..=max_y {
            for cell_x in min_x..=max_x {
                let cell_center = self.cell_center(cell_x, cell_y);
                let mask = Self::radial_mask(radius, softness, (cell_center - center).magnitude());
                let amount = (mask * density * 255.0).round().clamp(0.0, 255.0) as u8;
                changed |= self.blend_cell(cell_x, cell_y, palette_index, amount, erase);
            }
        }
        changed
    }

    fn apply_capsule(
        &mut self,
        start: Vec2<f32>,
        end: Vec2<f32>,
        radius: f32,
        softness: f32,
        density: f32,
        palette_index: u8,
        erase: bool,
    ) -> bool {
        let radius = radius.max(self.cell_size * 0.5);
        let density = density.clamp(0.0, 1.0);
        let min = Vec2::new(start.x.min(end.x), start.y.min(end.y)) - Vec2::broadcast(radius);
        let max = Vec2::new(start.x.max(end.x), start.y.max(end.y)) + Vec2::broadcast(radius);
        let (min_x, min_y) = self.local_to_cell(min);
        let (max_x, max_y) = self.local_to_cell(max);
        let mut changed = false;
        for cell_y in min_y..=max_y {
            for cell_x in min_x..=max_x {
                let cell_center = self.cell_center(cell_x, cell_y);
                let dist = Self::point_segment_distance(cell_center, start, end);
                let mask = Self::radial_mask(radius, softness, dist);
                let amount = (mask * density * 255.0).round().clamp(0.0, 255.0) as u8;
                changed |= self.blend_cell(cell_x, cell_y, palette_index, amount, erase);
            }
        }
        changed
    }

    pub fn set_channel_source(&mut self, channel: i32, source: Option<PixelSource>) {
        if let Some(binding) = self
            .channel_bindings
            .iter_mut()
            .find(|binding| binding.channel == channel)
        {
            binding.source = source;
        } else {
            self.channel_bindings.push(OrganicChannelBinding {
                channel,
                name: format!("Channel {}", channel),
                source,
                roughness: 0.5,
                metallic: 0.0,
                opacity: 1.0,
                emissive: 0.0,
            });
        }
    }

    pub fn set_channel_material(
        &mut self,
        channel: i32,
        roughness: f32,
        metallic: f32,
        opacity: f32,
        emissive: f32,
    ) {
        if let Some(binding) = self
            .channel_bindings
            .iter_mut()
            .find(|binding| binding.channel == channel)
        {
            binding.roughness = roughness.clamp(0.0, 1.0);
            binding.metallic = metallic.clamp(0.0, 1.0);
            binding.opacity = opacity.clamp(0.0, 1.0);
            binding.emissive = emissive.clamp(0.0, 1.0);
        } else {
            self.channel_bindings.push(OrganicChannelBinding {
                channel,
                name: format!("Channel {}", channel),
                source: None,
                roughness: roughness.clamp(0.0, 1.0),
                metallic: metallic.clamp(0.0, 1.0),
                opacity: opacity.clamp(0.0, 1.0),
                emissive: emissive.clamp(0.0, 1.0),
            });
        }
    }

    pub fn source_for_channel(&self, channel: i32) -> Option<&PixelSource> {
        self.channel_bindings
            .iter()
            .find(|binding| binding.channel == channel)
            .and_then(|binding| binding.source.as_ref())
    }

    pub fn binding_for_channel(&self, channel: i32) -> Option<&OrganicChannelBinding> {
        self.channel_bindings
            .iter()
            .find(|binding| binding.channel == channel)
    }

    pub fn paint_sphere(
        &mut self,
        center: Vec2<f32>,
        radius: f32,
        anchor_offset: f32,
        max_height: f32,
        edge_softness: f32,
        height_falloff: f32,
        density: f32,
        channel: i32,
        source: Option<PixelSource>,
        grow_positive: bool,
    ) -> bool {
        let _ = (
            anchor_offset,
            max_height,
            height_falloff,
            channel,
            grow_positive,
        );
        if density <= 0.001 {
            return false;
        }
        let Some(palette_index) = Self::palette_index_from_source(source) else {
            return false;
        };
        self.apply_blob(center, radius, edge_softness, density, palette_index, false)
    }

    pub fn paint_capsule(
        &mut self,
        start: Vec2<f32>,
        end: Vec2<f32>,
        radius: f32,
        anchor_offset: f32,
        max_height: f32,
        edge_softness: f32,
        height_falloff: f32,
        density: f32,
        channel: i32,
        source: Option<PixelSource>,
        grow_positive: bool,
    ) -> bool {
        let _ = (
            anchor_offset,
            max_height,
            height_falloff,
            channel,
            grow_positive,
        );
        if density <= 0.001 {
            return false;
        }
        let Some(palette_index) = Self::palette_index_from_source(source) else {
            return false;
        };
        self.apply_capsule(
            start,
            end,
            radius,
            edge_softness,
            density,
            palette_index,
            false,
        )
    }

    pub fn erase_sphere(
        &mut self,
        center: Vec2<f32>,
        radius: f32,
        anchor_offset: f32,
        max_height: f32,
        edge_softness: f32,
        height_falloff: f32,
        grow_positive: bool,
    ) -> bool {
        let _ = (anchor_offset, max_height, height_falloff, grow_positive);
        self.apply_blob(center, radius, edge_softness, 1.0, 0, true)
    }

    pub fn erase_capsule(
        &mut self,
        start: Vec2<f32>,
        end: Vec2<f32>,
        radius: f32,
        anchor_offset: f32,
        max_height: f32,
        edge_softness: f32,
        height_falloff: f32,
        grow_positive: bool,
    ) -> bool {
        let _ = (anchor_offset, max_height, height_falloff, grow_positive);
        self.apply_capsule(start, end, radius, edge_softness, 1.0, 0, true)
    }

    pub fn sample(&self, local: Vec2<f32>) -> Option<OrganicDetailCell> {
        let (cell_x, cell_y) = self.local_to_cell(local);
        let (page_x, page_y, slot) = self.page_slot(cell_x, cell_y);
        let key = Self::page_key(page_x, page_y);
        let page = self.pages.get(&key)?;
        let cell = page.cells.get(slot)?;
        if cell.coverage == 0 {
            None
        } else {
            Some(cell.clone())
        }
    }

    pub fn painted_local_bounds(&self) -> Option<(Vec2<f32>, Vec2<f32>)> {
        let mut min = Vec2::new(f32::INFINITY, f32::INFINITY);
        let mut max = Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
        let mut found = false;
        let page_size = self.page_size.max(1);
        for page in self.pages.values() {
            for (slot, cell) in page.cells.iter().enumerate() {
                if cell.coverage == 0 {
                    continue;
                }
                let local_x = (slot as i32).rem_euclid(page_size);
                let local_y = (slot as i32).div_euclid(page_size);
                let cell_x = page.page_x * page_size + local_x;
                let cell_y = page.page_y * page_size + local_y;
                let cell_min = Vec2::new(
                    cell_x as f32 * self.cell_size,
                    cell_y as f32 * self.cell_size,
                );
                let cell_max = cell_min + Vec2::broadcast(self.cell_size);
                min.x = min.x.min(cell_min.x);
                min.y = min.y.min(cell_min.y);
                max.x = max.x.max(cell_max.x);
                max.y = max.y.max(cell_max.y);
                found = true;
            }
        }
        if found { Some((min, max)) } else { None }
    }

    fn point_segment_distance(p: Vec2<f32>, a: Vec2<f32>, b: Vec2<f32>) -> f32 {
        let ab = b - a;
        let len_sq = ab.magnitude_squared();
        if len_sq <= f32::EPSILON {
            return (p - a).magnitude();
        }
        let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
        (p - (a + ab * t)).magnitude()
    }
}

pub fn default_organic_layers() -> IndexMap<Uuid, OrganicVolumeLayer> {
    IndexMap::default()
}
