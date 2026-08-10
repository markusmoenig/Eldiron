use crate::{
    ParticleEmitter, Texture,
    material_library::{MaterialDefinition, MaterialFamily},
};
use procedural_recipes::{GeometryFeature, GeometryOperation, Recipe, RecipePlacement};
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug, Default)]
pub enum TileRole {
    Character, // #d9ac8b
    Nature,    // #3e6958
    Mountain,  // #b1a58d
    Road,      // #624c3c
    Water,     // #243d5c
    #[default]
    ManMade, // #e0c872
    Dungeon,   // #b03a48
    Effect,    // #d4804d
    Icon,      // #5c8b93
    UI,        // #e3cfb4
}

impl TileRole {
    pub fn to_color(self) -> TheColor {
        match self {
            TileRole::Character => TheColor::from_hex("#d9ac8b"),
            TileRole::Nature => TheColor::from_hex("#3e6958"),
            TileRole::Mountain => TheColor::from_hex("#b1a58d"),
            TileRole::Road => TheColor::from_hex("#624c3c"),
            TileRole::Water => TheColor::from_hex("#243d5c"),
            TileRole::ManMade => TheColor::from_hex("#e0c872"),
            TileRole::Dungeon => TheColor::from_hex("#b03a48"),
            TileRole::Effect => TheColor::from_hex("#d4804d"),
            TileRole::Icon => TheColor::from_hex("#5c8b93"),
            TileRole::UI => TheColor::from_hex("#e3cfb4"),
        }
    }
    pub fn to_string(self) -> &'static str {
        match self {
            TileRole::Character => "Character",
            TileRole::Nature => "Nature",
            TileRole::Mountain => "Mountain",
            TileRole::Road => "Road",
            TileRole::Water => "Water",
            TileRole::ManMade => "Man Made",
            TileRole::Dungeon => "Dungeon",
            TileRole::Effect => "Effect",
            TileRole::Icon => "Icon",
            TileRole::UI => "UI",
        }
    }
    pub fn iterator() -> impl Iterator<Item = TileRole> {
        [
            TileRole::Character,
            TileRole::Nature,
            TileRole::Mountain,
            TileRole::Road,
            TileRole::Water,
            TileRole::ManMade,
            TileRole::Dungeon,
            TileRole::Effect,
            TileRole::Icon,
            TileRole::UI,
        ]
        .iter()
        .copied()
    }
    pub fn from_index(index: u8) -> TileRole {
        match index {
            0 => TileRole::Character,
            1 => TileRole::Nature,
            2 => TileRole::Mountain,
            3 => TileRole::Road,
            4 => TileRole::Water,
            5 => TileRole::ManMade,
            6 => TileRole::Dungeon,
            7 => TileRole::Effect,
            8 => TileRole::Icon,
            9 => TileRole::UI,
            _ => TileRole::ManMade,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub struct TileLightEmitter {
    pub color: [u8; 4],
    pub intensity: f32,
    pub range: f32,
    pub flicker: f32,
    pub lift: f32,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub struct TileAttachment {
    pub name: String,
    pub position: [f32; 3],
    pub direction: [f32; 3],
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub struct TileLightEffect {
    pub name: String,
    pub attachment: String,
    pub emitter: TileLightEmitter,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct TileParticleEffect {
    pub name: String,
    pub attachment: String,
    pub emitter: ParticleEmitter,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct TileProceduralMeta {
    /// Optional procedural generation style selector, e.g. "stone" or "crypt".
    #[serde(default)]
    pub style: String,
    /// Optional procedural generation kind, e.g. "floor", "wall", or "door".
    #[serde(default)]
    pub kind: String,
    /// Weighted random selection weight for procedural generation.
    #[serde(default = "default_proc_weight")]
    pub weight: u32,
    /// Number of map cells covered by one coordinated procedural texture.
    ///
    /// Ordinary tiles use `[1, 1]`. Recipe-backed surfaces may cover several
    /// cells so their UVs remain continuous instead of restarting per cell.
    #[serde(default = "default_proc_coverage")]
    pub coverage: [u32; 2],
}

impl Default for TileProceduralMeta {
    fn default() -> Self {
        Self {
            style: String::new(),
            kind: String::new(),
            weight: default_proc_weight(),
            coverage: default_proc_coverage(),
        }
    }
}

fn default_proc_coverage() -> [u32; 2] {
    [1, 1]
}

/// Placement-time geometry carried by a tile asset.
///
/// The map compiler instantiates these features with local placement context. Keeping the
/// declaration on the tile ensures geometry, collision, and authored surfaces travel together.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum TileGeometryFeature {
    Box(TileBoxGeometry),
    /// Legacy serialized form. New recipes compile to generic Box nodes.
    Niche(TileNicheGeometry),
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug, Default)]
pub enum TileGeometryOperation {
    #[default]
    Add,
    Subtract,
}

/// How procedural geometry/effects compose with the placement host.
#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug, Default)]
pub enum TileRecipePlacement {
    /// The baked Tile owns and replaces the host surface.
    #[default]
    Surface,
    /// Geometry/effects overlay the existing host surface.
    Fixture,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct TileBoxGeometry {
    pub name: String,
    pub operation: TileGeometryOperation,
    pub surface: String,
    pub position: [f32; 3],
    pub size: [f32; 3],
    pub repeat: [u32; 3],
    pub spacing: [f32; 3],
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct TileNicheGeometry {
    pub name: String,
    pub surface: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub depth: f32,
    pub sill: f32,
    #[serde(default)]
    pub frame_width: f32,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct TileMaterialMeta {
    /// High-level material preset used by authoring tools, e.g. "stone" or "wood".
    #[serde(default = "default_material_preset")]
    pub preset: String,
    /// High-level finish modifier, e.g. "matte", "natural", "polished", or "wet".
    #[serde(default = "default_material_finish")]
    pub finish: String,
}

fn default_material_preset() -> String {
    "default".to_string()
}

fn default_material_finish() -> String {
    "natural".to_string()
}

const PALETTE_TILE_UUID_PREFIX: u128 = 0x50414C455454455F0000000000000000u128;
const PALETTE_TILE_UUID_MASK: u128 = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00u128;

impl TileMaterialMeta {
    pub fn is_default(&self) -> bool {
        self.preset.trim().is_empty()
            || (self.preset.eq_ignore_ascii_case("default")
                && self.finish.eq_ignore_ascii_case("natural"))
    }

    pub fn normalized_preset(&self) -> String {
        let preset = self.preset.trim().to_ascii_lowercase();
        if preset.is_empty() {
            "default".to_string()
        } else {
            preset
        }
    }

    pub fn normalized_finish(&self) -> String {
        let finish = self.finish.trim().to_ascii_lowercase();
        if finish.is_empty() {
            "natural".to_string()
        } else {
            finish
        }
    }

    pub fn rmoe_values(&self) -> Option<[f32; 4]> {
        self.base_rmoe_values()
    }

    pub fn material_id(&self) -> Option<u8> {
        if self.is_default() {
            return None;
        }
        Some(
            MaterialDefinition::from_preset_finish(
                &self.normalized_preset(),
                &self.normalized_finish(),
            )
            .id(),
        )
    }

    fn base_rmoe_values(&self) -> Option<[f32; 4]> {
        if self.is_default() {
            return None;
        }

        Some(
            MaterialDefinition::from_preset_finish(
                &self.normalized_preset(),
                &self.normalized_finish(),
            )
            .rmoe_values(),
        )
    }

    fn color_luma_saturation(rgba: [u8; 4]) -> (f32, f32, f32) {
        let r = rgba[0] as f32 / 255.0;
        let g = rgba[1] as f32 / 255.0;
        let b = rgba[2] as f32 / 255.0;
        let a = rgba[3] as f32 / 255.0;
        let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let saturation = if max <= f32::EPSILON {
            0.0
        } else {
            (max - min) / max
        };
        (luma, saturation, a)
    }

    pub fn rmoe_values_for_color(&self, rgba: [u8; 4]) -> Option<[f32; 4]> {
        let [mut roughness, mut metallic, mut opacity, mut emissive] = self.base_rmoe_values()?;
        let preset = self.normalized_preset();
        let (luma, saturation, alpha) = Self::color_luma_saturation(rgba);
        let dark_detail = 0.5 - luma;
        let bright_detail = luma - 0.5;
        let saturated_detail = saturation - 0.35;

        match preset.as_str() {
            "stone" => {
                roughness += dark_detail * 0.18 + saturated_detail * 0.05;
            }
            "wood" => {
                roughness += dark_detail * 0.14 - saturation * 0.04;
            }
            "metal" => {
                roughness += dark_detail * 0.16 - bright_detail.max(0.0) * 0.05;
                metallic *= 0.92 + luma * 0.08;
            }
            "glass" => {
                roughness += dark_detail * 0.04 + saturation * 0.02;
                opacity *= 0.85 + luma * 0.15;
            }
            "water" => {
                roughness += dark_detail * 0.05 + saturation * 0.02;
                opacity *= 0.75 + luma * 0.25;
            }
            "mirror" => {
                roughness += dark_detail * 0.03;
                metallic *= 0.9 + luma * 0.1;
            }
            "emissive" => {
                roughness += dark_detail * 0.08;
                emissive *= (0.25 + luma * 0.75 + saturation * 0.15).clamp(0.0, 1.0);
            }
            "dirt" => {
                roughness += dark_detail * 0.18 + (1.0 - luma) * 0.08;
            }
            "fabric" => {
                roughness += dark_detail * 0.16 + saturation * 0.05;
            }
            "plastic" => {
                roughness += dark_detail * 0.08 - saturation * 0.03;
            }
            _ => {
                roughness += dark_detail * 0.08;
            }
        }

        let family = MaterialFamily::from_preset(&preset);
        if matches!(
            family,
            MaterialFamily::Skin | MaterialFamily::Wax | MaterialFamily::Foliage
        ) {
            roughness += dark_detail * 0.04;
        }

        if alpha < 0.98 {
            opacity *= alpha;
        }

        Some([
            roughness.clamp(0.02, 1.0),
            metallic.clamp(0.0, 1.0),
            opacity.clamp(0.0, 1.0),
            emissive.clamp(0.0, 1.0),
        ])
    }

    pub fn variant_tile_id(base_id: Uuid, preset: &str, finish: &str) -> Uuid {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in base_id
            .as_bytes()
            .iter()
            .copied()
            .chain(preset.trim().to_ascii_lowercase().bytes())
            .chain([0])
            .chain(finish.trim().to_ascii_lowercase().bytes())
        {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }

        Uuid::from_u128(0x4D41544C5F5641520000000000000000u128 | hash as u128)
    }
}

impl Default for TileMaterialMeta {
    fn default() -> Self {
        Self {
            preset: default_material_preset(),
            finish: default_material_finish(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub struct Tile {
    pub id: Uuid,
    pub role: TileRole,
    /// The textures of the tiles. Primary source.
    pub textures: Vec<Texture>,
    /// The module if the textures are shader generated
    pub module: Option<serde_json::Value>,
    /// For top down 2D scenarios
    pub blocking: bool,
    /// The scale of the tile (mostly used for billboard rendering)
    pub scale: f32,
    /// Human-readable alias used for filtering and source lookup.
    #[serde(default, alias = "tags")]
    pub alias: String,
    /// Gameplay-facing tags used by tile transition events such as
    /// `entered_tile` and `left_tile`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gameplay_tags: Vec<String>,
    /// Optional procedural generation hints used by region generators.
    #[serde(default)]
    pub procedural: TileProceduralMeta,
    /// Shared Recipe composition semantics used by Creator, Source, and clients.
    #[serde(default)]
    pub recipe_placement: TileRecipePlacement,
    /// Optional architectural geometry instantiated whenever this tile is placed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub geometry: Vec<TileGeometryFeature>,
    /// Named placement-local points shared by procedural geometry and effects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<TileAttachment>,
    /// Placement-local lights authored by a procedural recipe.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub light_effects: Vec<TileLightEffect>,
    /// Placement-local particle emitters authored by a procedural recipe.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub particle_effects: Vec<TileParticleEffect>,
    /// Optional high-level material metadata used to derive render material values.
    #[serde(default, skip_serializing_if = "TileMaterialMeta::is_default")]
    pub material: TileMaterialMeta,
    /// Source-level procedural material alias, retained for editor/source introspection.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub material_alias: String,
    /// Packed per-pixel R/M/O/E bytes for every texture frame. Texture serialization
    /// intentionally strips these bytes, so generated tiles persist them here.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub baked_material_data: Vec<Vec<u8>>,
    /// Optional particle emitter definition for this tile.
    #[serde(default)]
    pub particle_emitter: Option<ParticleEmitter>,
    /// Optional point light definition for this tile.
    #[serde(default)]
    pub light_emitter: Option<TileLightEmitter>,
}

impl Tile {
    /// Parse a comma, semicolon, or newline separated gameplay tag list.
    /// Tags are stored in a stable, lowercase form so scripts can compare them
    /// without depending on how they were capitalized in Creator.
    pub fn parse_gameplay_tags(value: &str) -> Vec<String> {
        let mut tags = Vec::new();
        for tag in value.split([',', ';', '\n']) {
            let tag = tag.trim().to_ascii_lowercase();
            if !tag.is_empty() && !tags.contains(&tag) {
                tags.push(tag);
            }
        }
        tags
    }

    /// Return normalized gameplay tags, including for project data authored
    /// outside Creator.
    pub fn normalized_gameplay_tags(&self) -> Vec<String> {
        Self::parse_gameplay_tags(&self.gameplay_tags.join(","))
    }

    fn is_synthetic_palette_tile_id(id: Uuid) -> bool {
        id.as_u128() & PALETTE_TILE_UUID_MASK == PALETTE_TILE_UUID_PREFIX
    }

    fn apply_material_meta_to_texture(material: &TileMaterialMeta, texture: &mut Texture) {
        if texture.data_ext.is_none() {
            texture.generate_normals(true);
        }

        for y in 0..texture.height {
            for x in 0..texture.width {
                let idx = (y * texture.width + x) * 4;
                let rgba = [
                    texture.data[idx],
                    texture.data[idx + 1],
                    texture.data[idx + 2],
                    texture.data[idx + 3],
                ];
                if let Some(material_id) = material.material_id() {
                    texture.set_material_id(x as u32, y as u32, material_id);
                } else if let Some([roughness, metallic, opacity, emissive]) =
                    material.rmoe_values_for_color(rgba)
                {
                    texture
                        .set_materials(x as u32, y as u32, roughness, metallic, opacity, emissive);
                }
            }
        }
    }

    /// Apply the host-facing geometry/effect metadata shared by Source,
    /// Creator, and runtime clients for a procedural Tile recipe.
    pub fn apply_procedural_recipe_metadata(&mut self, recipe: &Recipe) {
        self.blocking = recipe.blocking;
        self.procedural.coverage = recipe.coverage;
        self.recipe_placement = match recipe.placement {
            RecipePlacement::Surface => TileRecipePlacement::Surface,
            RecipePlacement::Fixture => TileRecipePlacement::Fixture,
        };
        self.geometry = recipe
            .geometry
            .iter()
            .map(|feature| match feature {
                GeometryFeature::Box(geometry_box) => TileGeometryFeature::Box(TileBoxGeometry {
                    name: geometry_box.name.clone(),
                    operation: match geometry_box.operation {
                        GeometryOperation::Add => TileGeometryOperation::Add,
                        GeometryOperation::Subtract => TileGeometryOperation::Subtract,
                    },
                    surface: geometry_box.surface.clone(),
                    position: geometry_box.position,
                    size: geometry_box.size,
                    repeat: geometry_box.repeat,
                    spacing: geometry_box.spacing,
                }),
            })
            .collect();
        self.attachments = recipe
            .attachments
            .iter()
            .map(|attachment| TileAttachment {
                name: attachment.name.clone(),
                position: attachment.position,
                direction: attachment.direction,
            })
            .collect();
        self.light_effects = recipe
            .lights
            .iter()
            .map(|light| TileLightEffect {
                name: light.name.clone(),
                attachment: light.attachment.clone(),
                emitter: TileLightEmitter {
                    color: light.color,
                    intensity: light.intensity,
                    range: light.range,
                    flicker: light.flicker,
                    lift: light.lift,
                },
            })
            .collect();
        self.particle_effects = recipe
            .particles
            .iter()
            .map(|particles| {
                let mut emitter = ParticleEmitter::new(
                    vek::Vec3::zero(),
                    vek::Vec3::new(
                        particles.direction[0],
                        particles.direction[1],
                        particles.direction[2],
                    ),
                );
                emitter.spread = particles.spread;
                emitter.rate = particles.rate;
                emitter.color = particles.color;
                emitter.color_ramp = particles.color_ramp;
                emitter.color_variation = particles.color_variation;
                emitter.lifetime_range = (particles.lifetime[0], particles.lifetime[1]);
                emitter.radius_range = (particles.radius[0], particles.radius[1]);
                emitter.speed_range = (particles.speed[0], particles.speed[1]);
                emitter.spawn_area = particles.spawn_area;
                emitter.flame_base = particles.flame_base;
                TileParticleEffect {
                    name: particles.name.clone(),
                    attachment: particles.attachment.clone(),
                    emitter,
                }
            })
            .collect();
    }

    /// Create a tile from a single texture.
    pub fn from_texture(texture: Texture) -> Self {
        Self {
            id: Uuid::new_v4(),
            role: TileRole::ManMade,
            textures: vec![texture],
            module: None,
            blocking: false,
            scale: 1.0,
            alias: String::new(),
            gameplay_tags: Vec::new(),
            procedural: TileProceduralMeta::default(),
            recipe_placement: TileRecipePlacement::Surface,
            geometry: Vec::new(),
            attachments: Vec::new(),
            light_effects: Vec::new(),
            particle_effects: Vec::new(),
            material: TileMaterialMeta::default(),
            material_alias: String::new(),
            baked_material_data: Vec::new(),
            particle_emitter: None,
            light_emitter: None,
        }
    }

    /// Create a tile from a vector of textures.
    pub fn from_textures(textures: Vec<Texture>) -> Self {
        Self {
            id: Uuid::new_v4(),
            textures,
            module: None,
            blocking: false,
            scale: 1.0,
            alias: String::new(),
            procedural: TileProceduralMeta::default(),
            material: TileMaterialMeta::default(),
            particle_emitter: None,
            light_emitter: None,
            ..Default::default()
        }
    }

    /// Create an empty tile.
    pub fn empty() -> Self {
        Self {
            id: Uuid::new_v4(),
            textures: vec![],
            module: None,
            blocking: false,
            scale: 1.0,
            alias: String::new(),
            procedural: TileProceduralMeta::default(),
            particle_emitter: None,
            light_emitter: None,
            ..Default::default()
        }
    }

    /// Append a texture to the Tile.
    pub fn append(&mut self, texture: Texture) {
        self.textures.push(texture);
    }

    /// Converts the frames to an array of buffers
    pub fn to_buffer_array(&self) -> Vec<Vec<u8>> {
        let mut b = vec![];
        for t in &self.textures {
            b.push(t.data.to_vec());
        }
        b
    }

    /// Converts the frames to an array of material buffers
    pub fn to_material_array(&self) -> Vec<Vec<u8>> {
        if self.material.rmoe_values().is_some() {
            return self
                .textures
                .iter()
                .map(|texture| {
                    let mut texture = texture.clone();
                    Self::apply_material_meta_to_texture(&self.material, &mut texture);
                    texture.data_ext.unwrap_or_default()
                })
                .collect();
        }

        let preserve_runtime_materials = Self::is_synthetic_palette_tile_id(self.id);
        let mut b = vec![];
        for (frame, texture) in self.textures.iter().enumerate() {
            if texture.data_ext.is_some() {
                let mut texture = texture.clone();
                let restored_baked = self
                    .baked_material_data
                    .get(frame)
                    .is_some_and(|bytes| texture.apply_material_bytes(bytes));
                if !preserve_runtime_materials && !restored_baked {
                    texture.set_materials_all(0.5, 0.0, 1.0, 0.0);
                }
                if let Some(mat) = texture.data_ext {
                    b.push(mat);
                }
            }
        }
        b
    }

    /// Checks if the tile is empty
    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    /// Returns a new Tile with all textures resized to the specified dimensions
    pub fn resized(&self, new_width: usize, new_height: usize) -> Self {
        let resized_textures = self
            .textures
            .iter()
            .map(|t| t.resized(new_width, new_height))
            .collect();

        Self {
            id: self.id,
            role: self.role,
            textures: resized_textures,
            module: self.module.clone(),
            blocking: self.blocking,
            scale: self.scale,
            alias: self.alias.clone(),
            gameplay_tags: self.gameplay_tags.clone(),
            procedural: self.procedural.clone(),
            recipe_placement: self.recipe_placement,
            geometry: self.geometry.clone(),
            attachments: self.attachments.clone(),
            light_effects: self.light_effects.clone(),
            particle_effects: self.particle_effects.clone(),
            material: self.material.clone(),
            material_alias: self.material_alias.clone(),
            baked_material_data: self
                .textures
                .iter()
                .zip(&self.baked_material_data)
                .map(|(texture, bytes)| {
                    let mut texture = texture.clone();
                    texture.apply_material_bytes(bytes);
                    texture.resized(new_width, new_height).material_bytes()
                })
                .collect(),
            particle_emitter: self.particle_emitter.clone(),
            light_emitter: self.light_emitter.clone(),
        }
    }

    /// Sets the number of frames in the textures array.
    /// If frames > current count: duplicates the last texture to fill.
    /// If frames < current count: truncates the array.
    /// Also handles material_map and normal_map within each texture.
    pub fn set_frames(&mut self, frames: usize) {
        if frames == 0 {
            self.textures.clear();
            self.baked_material_data.clear();
            return;
        }

        let current_count = self.textures.len();

        if frames > current_count {
            if current_count > 0 {
                // Duplicate the last texture to reach the desired frame count
                let last_texture = self.textures.last().unwrap().clone();
                for _ in current_count..frames {
                    self.textures.push(last_texture.clone());
                }
                if let Some(last_material) = self.baked_material_data.last().cloned() {
                    while self.baked_material_data.len() < frames {
                        self.baked_material_data.push(last_material.clone());
                    }
                }
            }
        } else if frames < current_count {
            // Truncate to the desired frame count
            self.textures.truncate(frames);
            self.baked_material_data.truncate(frames);
        }
    }

    /// Initialize all textures with default materials and compute normals
    /// Sets roughness=0.5, metallic=0.0, opacity=1.0, emissive=0.0 for all pixels
    /// Then generates normals from the color data for each texture
    pub fn set_default_materials(&mut self) {
        for texture in &mut self.textures {
            texture.set_default_materials();
            texture.generate_normals(true);
        }
    }
}

const fn default_proc_weight() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gameplay_tags_are_normalized_and_deduplicated() {
        assert_eq!(
            Tile::parse_gameplay_tags(" Chair, water;CHAIR\n  trap "),
            vec!["chair", "water", "trap"]
        );

        let mut tile = Tile::empty();
        tile.gameplay_tags = vec!["Chair".into(), " chair ".into(), "TRAP".into()];
        assert_eq!(tile.normalized_gameplay_tags(), vec!["chair", "trap"]);
    }

    #[test]
    fn gameplay_tags_default_when_loading_older_tile_data() {
        let tile: Tile = serde_json::from_value(serde_json::json!({
            "id": Uuid::new_v4(),
            "role": "ManMade",
            "textures": [],
            "module": null,
            "blocking": false,
            "scale": 1.0,
            "alias": "chair"
        }))
        .unwrap();

        assert!(tile.gameplay_tags.is_empty());
    }

    #[test]
    fn material_meta_overrides_material_frames() {
        let mut tile = Tile::from_texture(Texture::from_color([120, 120, 120, 255]));
        tile.material = TileMaterialMeta {
            preset: "metal".to_string(),
            finish: "polished".to_string(),
        };

        let frames = tile.to_material_array();
        assert_eq!(frames.len(), 1);

        let mut material_texture = Texture::from_color([0, 0, 0, 255]);
        material_texture.data_ext = Some(frames[0].clone());
        let (roughness, metallic, opacity, emissive) = material_texture.get_materials(0, 0);

        assert!(roughness <= 0.15);
        assert!(metallic >= 0.85);
        assert_eq!(opacity, 1.0);
        assert_eq!(emissive, 0.0);
    }

    #[test]
    fn baked_procedural_materials_survive_tile_serialization() {
        let mut texture = Texture::from_color([80, 70, 60, 255]);
        texture.set_materials_all(0.2, 0.4, 0.8, 0.6);
        texture.set_normal(0, 0, 0.25, -0.25);
        let baked = texture.material_bytes();
        let mut tile = Tile::from_texture(texture);
        tile.recipe_placement = TileRecipePlacement::Fixture;
        tile.material_alias = "dungeon/test".to_string();
        tile.baked_material_data = vec![baked];

        let serialized = serde_json::to_string(&tile).unwrap();
        let decoded: Tile = serde_json::from_str(&serialized).unwrap();
        let frames = decoded.to_material_array();
        let mut reconstructed = decoded.textures[0].clone();
        reconstructed.data_ext = Some(frames[0].clone());
        let (roughness, metallic, opacity, emissive) = reconstructed.get_materials(0, 0);

        assert_eq!(decoded.material_alias, "dungeon/test");
        assert_eq!(decoded.recipe_placement, TileRecipePlacement::Fixture);
        assert!((roughness - 0.2).abs() < 0.07);
        assert!((metallic - 0.4).abs() < 0.07);
        assert!((opacity - 0.8).abs() < 0.07);
        assert!((emissive - 0.6).abs() < 0.07);
        let (normal_x, normal_y) = reconstructed.get_normal(0, 0);
        assert!((normal_x - 0.25).abs() < 0.01);
        assert!((normal_y + 0.25).abs() < 0.01);
    }

    #[test]
    fn material_meta_writes_semantic_material_id() {
        let texture = Texture::new(
            vec![
                16, 16, 16, 255, //
                240, 240, 240, 255,
            ],
            2,
            1,
        );
        let mut tile = Tile::from_texture(texture);
        tile.material = TileMaterialMeta {
            preset: "emissive".to_string(),
            finish: "natural".to_string(),
        };

        let frames = tile.to_material_array();
        let mut material_texture = Texture::new(vec![0, 0, 0, 255, 0, 0, 0, 255], 2, 1);
        material_texture.data_ext = Some(frames[0].clone());

        let (_, _, _, dark_emissive) = material_texture.get_materials(0, 0);
        let (_, _, _, bright_emissive) = material_texture.get_materials(1, 0);
        let dark_id = material_texture.get_material_id(0, 0);
        let bright_id = material_texture.get_material_id(1, 0);

        assert_eq!(dark_id, tile.material.material_id());
        assert_eq!(bright_id, tile.material.material_id());
        assert_eq!(dark_emissive, bright_emissive);
        assert_eq!(bright_emissive, 1.0);
    }

    #[test]
    fn default_material_meta_resets_legacy_material_pixels() {
        let mut texture = Texture::from_color([120, 120, 120, 255]);
        texture.set_materials_all(0.0, 1.0, 0.2, 1.0);
        texture.set_normal(0, 0, 0.25, -0.5);
        let tile = Tile::from_texture(texture);

        let frames = tile.to_material_array();
        assert_eq!(frames.len(), 1);

        let mut material_texture = Texture::from_color([0, 0, 0, 255]);
        material_texture.data_ext = Some(frames[0].clone());
        let (roughness, metallic, opacity, emissive) = material_texture.get_materials(0, 0);
        let (normal_x, normal_y) = material_texture.get_normal(0, 0);

        assert_eq!(roughness, 0.53333336);
        assert_eq!(metallic, 0.0);
        assert_eq!(opacity, 1.0);
        assert_eq!(emissive, 0.0);
        assert!((normal_x - 0.25).abs() < 0.01);
        assert!((normal_y + 0.5).abs() < 0.01);
    }

    #[test]
    fn synthetic_palette_tiles_preserve_runtime_material_pixels() {
        let mut texture = Texture::from_color([120, 120, 120, 255]);
        texture.set_materials_all(0.02, 1.0, 1.0, 0.0);
        let mut tile = Tile::from_texture(texture);
        tile.id = Uuid::from_u128(PALETTE_TILE_UUID_PREFIX | 7);

        let frames = tile.to_material_array();
        assert_eq!(frames.len(), 1);

        let mut material_texture = Texture::from_color([0, 0, 0, 255]);
        material_texture.data_ext = Some(frames[0].clone());
        let (roughness, metallic, opacity, emissive) = material_texture.get_materials(0, 0);

        assert!(roughness <= 0.1);
        assert_eq!(metallic, 1.0);
        assert_eq!(opacity, 1.0);
        assert_eq!(emissive, 0.0);
    }
}
