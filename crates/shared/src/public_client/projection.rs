use super::*;
use crate::{
    asset::Asset, character::Character, item::Item, public_geometry::RegionGeometry,
    region::Region, screen::Screen,
};
use indexmap::IndexMap;
use rusterix::{Map, Texture, Tile, Value, ValueContainer};
use std::collections::BTreeMap;
use theframework_core::{Uuid, prelude::ThePalette};
#[path = "policy.rs"]
pub(super) mod policy;
/// Explicit public fields: adding authoring fields to Project cannot publish them.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicPresentation {
    pub name: String,
    pub config: String,
    pub locales: String,
    pub audio_fx: String,
    pub regions: Vec<PublicRegion>,
    pub screens: Vec<PublicScreen>,
    pub characters: Vec<PublicTemplate>,
    pub items: Vec<PublicTemplate>,
    pub assets: Vec<Asset>,
    pub prefabs: Vec<rusterix::BlockPropAsset>,
    pub prefab_paint: IndexMap<Uuid, crate::iso_paint::IsoPaintLayer>,
    pub palette: ThePalette,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicLayout {
    pub geometry: RegionGeometry,
    pub widgets: BTreeMap<u32, String>,
    pub screen_style: Option<String>,
    pub widget_names: BTreeMap<u32, String>,
    pub orthographic_bake: Option<rusterix::map::OrthographicBakeAsset>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicRegion {
    pub id: Uuid,
    pub name: String,
    pub config: String,
    pub layout: PublicLayout,
    pub paint: crate::iso_paint::IsoPaintLayer,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicScreen {
    pub id: Uuid,
    pub name: String,
    pub settings: String,
    pub layout: PublicLayout,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicTemplate {
    pub id: Uuid,
    pub name: String,
    pub data: String,
    pub layout: PublicLayout,
}
impl PublicLayout {
    fn capture(map: &Map, ui: bool) -> Result<Self, String> {
        let widgets = if ui {
            map.sectors
                .iter()
                .filter_map(|s| s.properties.get_str("data").map(|v| (s.id, v)))
                .map(|(id, v)| Ok((id, policy::ui_data(v)?)))
                .collect::<Result<_, String>>()?
        } else {
            BTreeMap::new()
        };
        let screen_style = if ui {
            map.properties
                .get_str("data")
                .map(policy::ui_data)
                .transpose()?
        } else {
            None
        };
        let mut bake = map.orthographic_bake.clone();
        if let Some(bake) = &mut bake {
            for tile in &mut bake.tiles {
                tile.color_png_path = None;
                tile.depth_path = None;
                tile.albedo_png_path = None;
                tile.normal_png_path = None;
                tile.material_png_path = None;
            }
        }
        Ok(Self {
            geometry: RegionGeometry::capture(map)?,
            widgets,
            screen_style,
            widget_names: if ui {
                map.sectors.iter().map(|s| (s.id, s.name.clone())).collect()
            } else {
                BTreeMap::new()
            },
            orthographic_bake: bake,
        })
    }
    pub(super) fn map(&self, name: &str) -> Result<Map, String> {
        self.geometry.validate_public()?;
        if let Some(bake) = &self.orthographic_bake {
            if bake.tiles.len() > 16384
                || bake.tiles.iter().any(|t| {
                    t.color_png_path.is_some()
                        || t.depth_path.is_some()
                        || t.albedo_png_path.is_some()
                        || t.normal_png_path.is_some()
                        || t.material_png_path.is_some()
                })
            {
                return Err("Invalid/external public bake references".into());
            }
        }

        let mut map = Map::default();
        self.geometry.apply(&mut map);
        map.name = name.into();
        for sector in &mut map.sectors {
            if let Some(name) = self.widget_names.get(&sector.id) {
                sector.name = name.clone();
            }
        }
        for (id, data) in &self.widgets {
            if policy::ui_data(data)? != *data {
                return Err("Unsupported public widget fields".into());
            }
            let sector = map
                .sectors
                .iter_mut()
                .find(|s| s.id == *id)
                .ok_or("Unknown widget sector")?;
            sector.properties.set("data", Value::Str(data.clone()));
        }
        if let Some(style) = &self.screen_style {
            if policy::ui_data(style)? != *style {
                return Err("Unsupported public screen style".into());
            }
            map.properties.set("data", Value::Str(style.clone()));
        }
        map.orthographic_bake = self.orthographic_bake.clone();
        Ok(map)
    }
}
impl PublicPresentation {
    pub fn capture(p: &Project) -> Result<Self, String> {
        Ok(Self {
            name: p.name.clone(),
            config: policy::config(&p.config)?,
            locales: p.locales.clone(),
            audio_fx: p.audio_fx.clone(),
            regions: p
                .regions
                .iter()
                .map(|r| {
                    Ok(PublicRegion {
                        id: r.id,
                        name: r.name.clone(),
                        config: policy::region_config(&r.config)?,
                        layout: PublicLayout::capture(&r.map, false)?,
                        paint: paint(&r.iso_paint),
                    })
                })
                .collect::<Result<_, String>>()?,
            screens: p
                .screens
                .values()
                .map(|s| {
                    Ok(PublicScreen {
                        id: s.id,
                        name: s.name.clone(),
                        settings: policy::screen_settings(&s.settings)?,
                        layout: PublicLayout::capture(&s.map, true)?,
                    })
                })
                .collect::<Result<_, String>>()?,
            characters: p
                .characters
                .values()
                .map(|c| {
                    Ok(PublicTemplate {
                        id: c.id,
                        name: c.name.clone(),
                        data: policy::template_data(&c.data)?,
                        layout: PublicLayout::capture(&c.map, false)?,
                    })
                })
                .collect::<Result<_, String>>()?,
            items: p
                .items
                .values()
                .map(|i| {
                    Ok(PublicTemplate {
                        id: i.id,
                        name: i.name.clone(),
                        data: policy::template_data(&i.data)?,
                        layout: PublicLayout::capture(&i.map, false)?,
                    })
                })
                .collect::<Result<_, String>>()?,
            assets: p.assets.values().cloned().collect(),
            prefabs: p
                .block_props
                .values()
                .map(prefab)
                .collect::<Result<_, String>>()?,
            prefab_paint: p
                .block_prop_paint
                .iter()
                .map(|(id, layer)| (*id, paint(layer)))
                .collect(),
            palette: p.palette.clone(),
        })
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.regions.len() > 1024
            || self.screens.len() > 1024
            || self.items.len() > 16384
            || self.characters.len() > 16384
            || self.assets.len() > 16384
        {
            return Err("Public release object budget exceeded".into());
        }
        fn unique<'a>(mut ids: impl Iterator<Item = &'a Uuid>) -> bool {
            let mut seen = std::collections::BTreeSet::new();
            ids.all(|id| seen.insert(*id))
        }
        if !unique(self.regions.iter().map(|v| &v.id))
            || !unique(self.screens.iter().map(|v| &v.id))
            || !unique(self.characters.iter().map(|v| &v.id))
            || !unique(self.items.iter().map(|v| &v.id))
            || !unique(self.assets.iter().map(|v| &v.id))
            || !unique(self.prefabs.iter().map(|v| &v.id))
        {
            return Err("Duplicate public object identity".into());
        }
        if policy::config(&self.config)? != self.config {
            return Err("Unsupported public configuration fields".into());
        }
        let settings = crate::project_play::ProjectPlaySettings::parse(&self.config)?;
        if settings.mode != crate::project_play::ProjectMode::Multiplayer {
            return Err("Public releases must be multiplayer".into());
        }
        settings.launch(crate::project_play::LaunchPurpose::Player)?;
        for c in self.characters.iter().chain(&self.items) {
            if policy::template_data(&c.data)? != c.data {
                return Err("Server attributes in public template".into());
            }
            c.layout.map(&c.name)?;
        }
        for r in &self.regions {
            if policy::region_config(&r.config)? != r.config {
                return Err("Private region configuration".into());
            }
            r.layout.map(&r.name)?;
        }
        for s in &self.screens {
            if policy::screen_settings(&s.settings)? != s.settings {
                return Err("Unsupported public screen settings".into());
            }
            s.layout.map(&s.name)?;
        }
        for layer in self
            .prefab_paint
            .values()
            .chain(self.regions.iter().map(|r| &r.paint))
        {
            for tile in layer
                .chunks
                .values()
                .flat_map(|c| &c.stamps)
                .filter_map(|s| s.source_image.as_ref())
            {
                validate_tile(tile)?;
            }
        }
        for p in &self.prefabs {
            if serde_json::to_value(prefab(p)?).map_err(|e| e.to_string())?
                != serde_json::to_value(p).map_err(|e| e.to_string())?
            {
                return Err("Private prefab metadata in public release".into());
            }
        }
        Ok(())
    }
    pub fn into_project(self) -> Result<Project, String> {
        let mut p = Project::new();
        p.name = self.name;
        p.config = self.config;
        p.locales = self.locales;
        p.audio_fx = self.audio_fx;
        p.regions = self
            .regions
            .into_iter()
            .map(|r| {
                let mut region = Region::new();
                region.id = r.id;
                region.name = r.name;
                region.config = r.config;
                region.map = r.layout.map(&region.name)?;
                region.iso_paint = restore_paint(r.paint);
                Ok(region)
            })
            .collect::<Result<_, String>>()?;
        p.screens = self
            .screens
            .into_iter()
            .map(|s| {
                let mut screen = Screen::new();
                screen.id = s.id;
                screen.name = s.name;
                screen.settings = s.settings;
                screen.map = s.layout.map(&screen.name)?;
                Ok((screen.id, screen))
            })
            .collect::<Result<_, String>>()?;
        for c in self.characters {
            let mut character = Character::new();
            character.id = c.id;
            character.name = c.name;
            character.data = c.data;
            character.map = c.layout.map(&character.name)?;
            p.characters.insert(character.id, character);
        }
        for i in self.items {
            let mut item = Item::new();
            item.id = i.id;
            item.name = i.name;
            item.data = i.data;
            item.map = i.layout.map(&item.name)?;
            p.items.insert(item.id, item);
        }
        p.assets = self.assets.into_iter().map(|a| (a.id, a)).collect();
        p.block_props = self.prefabs.into_iter().map(|a| (a.id, a)).collect();
        p.block_prop_paint = self
            .prefab_paint
            .into_iter()
            .map(|(id, layer)| (id, restore_paint(layer)))
            .collect();
        p.palette = self.palette;
        Ok(p)
    }
}
fn paint(source: &crate::iso_paint::IsoPaintLayer) -> crate::iso_paint::IsoPaintLayer {
    let mut layer = source.clone();
    for chunk in layer.chunks.values_mut() {
        for stamp in chunk.stamps.iter_mut() {
            if let Some(tile) = &stamp.source_image {
                stamp.source_image = Some(std::sync::Arc::new(tile_public(tile)));
            }
        }
    }
    layer
}
fn prefab(source: &rusterix::BlockPropAsset) -> Result<rusterix::BlockPropAsset, String> {
    use rusterix::map::block_prop::BlockPropGeometrySource;
    let mut asset = rusterix::BlockPropAsset::new(source.name.clone());
    asset.id = source.id;
    asset.origin = source.origin;
    for part in &source.parts {
        let mut p = part.clone();
        let mut map = Map::default();
        map.geometry_objects = part.geometry_source.geometry_objects().to_vec();
        let mut geometry = RegionGeometry::capture(&map)?;
        for (public, original) in geometry
            .objects
            .iter_mut()
            .zip(part.geometry_source.geometry_objects())
        {
            if let Some(slot) = original.properties.get_str("prefab_material_slot") {
                public
                    .properties
                    .set("prefab_material_slot", Value::Str(slot.into()));
            }
        }
        p.geometry_source = BlockPropGeometrySource::Authored {
            geometry_objects: geometry.objects,
        };
        asset.parts.push(p);
    }
    asset.interaction_targets = source.interaction_targets.clone();
    asset.support_surfaces = source.support_surfaces.clone();
    asset.particle_effects = source.particle_effects.clone();
    asset.light_effects = source.light_effects.clone();
    for component in &source.components {
        if component.kind != "Door" {
            continue;
        }
        let mut c = component.clone();
        c.properties = ValueContainer::default();
        for key in [
            "part_id",
            "secondary_part_id",
            "motion",
            "slide_axis",
            "slide_distance",
            "angle_degrees",
        ] {
            if let Some(v) = component.properties.get(key) {
                c.properties.set(key, v.clone());
            }
        }
        asset.components.push(c);
    }
    for c in &asset.components {
        let key = format!("door_{}_open", c.id);
        if let Some(Value::Bool(open)) = source.default_state.get(&key) {
            asset.default_state.set(&key, Value::Bool(*open));
        }
    }
    Ok(asset)
}
fn tile_public(source: &Tile) -> Tile {
    let mut tile = Tile::from_texture(Texture::white());
    tile.id = source.id;
    tile.role = source.role;
    tile.textures = source.textures.clone();
    tile.uv_pages = source.uv_pages.clone();
    tile.paint = source.paint.clone();
    tile.isometric = source.isometric.clone();
    tile.blocking = source.blocking;
    tile.scale = source.scale;
    tile.alias = source.alias.clone();
    tile.geometry = source.geometry.clone();
    tile.attachments = source.attachments.clone();
    tile.light_effects = source.light_effects.clone();
    tile.particle_effects = source.particle_effects.clone();
    tile.material = source.material.clone();
    tile.baked_material_data = source
        .textures
        .iter()
        .enumerate()
        .map(|(frame, texture)| {
            source
                .baked_material_data
                .get(frame)
                .filter(|b| !b.is_empty())
                .cloned()
                .unwrap_or_else(|| texture.material_bytes())
        })
        .collect();
    tile.particle_emitter = source.particle_emitter.clone();
    tile.light_emitter = source.light_emitter.clone();
    tile
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicTexture {
    pub texture: Texture,
    pub materials: Vec<u8>,
}
impl PublicTexture {
    fn capture(t: &Texture) -> Self {
        Self {
            texture: t.clone(),
            materials: t.material_bytes().into(),
        }
    }
    fn texture(&self) -> Result<Texture, String> {
        validate_texture(&self.texture)?;
        let mut t = self.texture.clone();
        if !self.materials.is_empty() && !t.apply_material_bytes(&self.materials) {
            return Err("Invalid public material pixels".into());
        }
        Ok(t)
    }
}
fn validate_texture(t: &Texture) -> Result<(), String> {
    if t.width == 0
        || t.height == 0
        || t.width > 8192
        || t.height > 8192
        || t.width.checked_mul(t.height).and_then(|n| n.checked_mul(4)) != Some(t.data.len())
        || t.data_ext.as_ref().is_some_and(|v| v.len() != t.data.len())
    {
        return Err("Invalid/oversized public texture".into());
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicAppearance {
    pub source: String,
    pub equipment: Vec<(String, String, String)>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicVisuals {
    pub rules: String,
    pub tiles: IndexMap<Uuid, Tile>,
    pub materials: BTreeMap<Uuid, Tile>,
    pub textures: BTreeMap<String, PublicTexture>,
    pub ruleset_materials: BTreeMap<String, PublicTexture>,
    pub animations: BTreeMap<String, (Vec<PublicTexture>, f32)>,
    pub item_icons: BTreeMap<String, Vec<PublicTexture>>,
    pub item_models: BTreeMap<String, String>,
    pub appearances: BTreeMap<String, PublicAppearance>,
    pub avatars: BTreeMap<String, PublicAvatar>,
    pub palette_materials: Vec<[f32; 4]>,
    pub palette_material_ids: Vec<u8>,
}
fn geometry_source(source: &str) -> Result<String, String> {
    if source.len() > 4 * 1024 * 1024 {
        return Err("Public geometry recipe exceeds budget".into());
    }
    let doc = denrim_materials::geometry::Document::load(source).map_err(|e| e.to_string())?;
    doc.save().map_err(|e| e.to_string())
}
impl PublicVisuals {
    pub fn capture(a: &rusterix::Assets) -> Result<Self, String> {
        Ok(Self {
            rules: policy::rules(&a.rules)?,
            tiles: a
                .tiles
                .iter()
                .map(|(id, t)| (*id, tile_public(t)))
                .collect(),
            materials: a
                .materials
                .iter()
                .map(|(id, t)| (*id, tile_public(t)))
                .collect(),
            textures: a
                .textures
                .iter()
                .map(|(id, t)| (id.clone(), PublicTexture::capture(t)))
                .collect(),
            ruleset_materials: a
                .ruleset_materials
                .iter()
                .map(|(id, t)| (id.clone(), PublicTexture::capture(t)))
                .collect(),
            animations: a
                .material_animations
                .iter()
                .map(|(id, (frames, duration))| {
                    (
                        id.clone(),
                        (
                            frames.iter().map(PublicTexture::capture).collect(),
                            *duration,
                        ),
                    )
                })
                .collect(),
            item_icons: a
                .item_icons
                .iter()
                .map(|(id, frames)| {
                    (
                        id.clone(),
                        frames.iter().map(PublicTexture::capture).collect(),
                    )
                })
                .collect(),
            item_models: a
                .item_models
                .iter()
                .map(|(id, source)| Ok((id.clone(), model_reference(source)?)))
                .collect::<Result<_, String>>()?,
            appearances: a
                .procedural_avatars
                .iter()
                .map(|(id, value)| {
                    Ok((
                        id.clone(),
                        PublicAppearance {
                            source: geometry_source(&value.source)?,
                            equipment: value.public_equipment_bindings().to_vec(),
                        },
                    ))
                })
                .collect::<Result<_, String>>()?,
            avatars: a
                .avatars
                .iter()
                .map(|(name, avatar)| (name.clone(), PublicAvatar::capture(avatar)))
                .collect(),
            palette_materials: a.palette_materials.clone(),
            palette_material_ids: a.palette_material_ids.clone(),
        })
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.tiles.len() > 16384 || self.appearances.len() > 4096 || self.avatars.len() > 4096 {
            return Err("Public visual asset budget exceeded".into());
        }
        if policy::rules(&self.rules)? != self.rules {
            return Err("Private/unsupported rule fields in public client".into());
        }
        for tile in self.tiles.values().chain(self.materials.values()) {
            validate_tile(tile)?;
        }
        for (frames, duration) in self.animations.values() {
            if frames.len() > 4096 || !duration.is_finite() || *duration <= 0. {
                return Err("Invalid public animation".into());
            }
        }
        for a in self.appearances.values() {
            if a.equipment.len() > 64
                || a.equipment
                    .iter()
                    .any(|(x, y, z)| x.len() > 256 || y.len() > 256 || z.len() > 256)
            {
                return Err("Invalid public appearance bindings".into());
            }
        }
        for a in self.avatars.values() {
            a.avatar()?;
        }
        for t in self
            .textures
            .values()
            .chain(self.ruleset_materials.values())
            .chain(self.animations.values().flat_map(|(f, _)| f.iter()))
            .chain(self.item_icons.values().flat_map(|f| f.iter()))
        {
            t.texture()?;
        }
        for reference in self.item_models.values() {
            model_reference(reference)?;
        }
        for appearance in self.appearances.values() {
            geometry_source(&appearance.source)?;
        }
        Ok(())
    }
    pub fn install(&self, a: &mut rusterix::Assets) -> Result<(), String> {
        let mut tiles = self.tiles.clone();
        for tile in tiles.values_mut() {
            restore_tile(tile);
        }
        // Publish restored pixels to both the UUID registry and indexed list.
        a.set_tiles(tiles);
        a.materials = self
            .materials
            .iter()
            .map(|(id, t)| (*id, t.clone()))
            .collect();
        for tile in a.materials.values_mut() {
            for (t, data) in tile.textures.iter_mut().zip(&tile.baked_material_data) {
                t.apply_material_bytes(data);
            }
        }
        a.textures = self
            .textures
            .iter()
            .map(|(id, t)| Ok((id.clone(), t.texture()?)))
            .collect::<Result<_, String>>()?;
        a.ruleset_materials = self
            .ruleset_materials
            .iter()
            .map(|(id, t)| Ok((id.clone(), t.texture()?)))
            .collect::<Result<_, String>>()?;
        a.material_animations = self
            .animations
            .iter()
            .map(|(id, (frames, d))| {
                Ok((
                    id.clone(),
                    (
                        frames
                            .iter()
                            .map(PublicTexture::texture)
                            .collect::<Result<_, _>>()?,
                        *d,
                    ),
                ))
            })
            .collect::<Result<_, String>>()?;
        a.item_icons = self
            .item_icons
            .iter()
            .map(|(id, frames)| {
                Ok((
                    id.clone(),
                    frames
                        .iter()
                        .map(PublicTexture::texture)
                        .collect::<Result<_, _>>()?,
                ))
            })
            .collect::<Result<_, String>>()?;
        a.item_models = self
            .item_models
            .iter()
            .map(|(id, s)| (id.clone(), s.clone()))
            .collect();
        a.palette_materials = self.palette_materials.clone();
        a.palette_material_ids = self.palette_material_ids.clone();
        a.avatars.clear();
        for (id, avatar) in &self.avatars {
            a.avatars.insert(id.clone(), avatar.avatar()?);
        }
        a.procedural_avatars.clear();
        for (id, value) in &self.appearances {
            let equipment = value
                .equipment
                .iter()
                .map(|(x, y, z)| (x.as_str(), y.as_str(), z.as_str()))
                .collect::<Vec<_>>();
            let appearance = rusterix::procedural_avatar::Appearance::new(
                &value.source,
                &equipment,
                rusterix::procedural_avatar::fingerprint(&(id, &value.source)),
            )?;
            a.procedural_avatars
                .insert(id.clone(), std::sync::Arc::new(appearance));
        }
        Ok(())
    }
}

/// Preserve packed material pixels in bitmap avatar frames as well as normals.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicAvatar {
    pub avatar: rusterix::Avatar,
    pub materials: Vec<Vec<u8>>,
}
impl PublicAvatar {
    fn capture(avatar: &rusterix::Avatar) -> Self {
        Self {
            avatar: avatar.clone(),
            materials: avatar
                .animations
                .iter()
                .flat_map(|a| a.perspectives.iter())
                .flat_map(|p| p.frames.iter())
                .map(|f| f.texture.material_bytes().into())
                .collect(),
        }
    }
    fn avatar(&self) -> Result<rusterix::Avatar, String> {
        let mut avatar = self.avatar.clone();
        let mut pixels = self.materials.iter();
        for frame in avatar
            .animations
            .iter_mut()
            .flat_map(|a| a.perspectives.iter_mut())
            .flat_map(|p| p.frames.iter_mut())
        {
            validate_texture(&frame.texture)?;
            let data = pixels.next().ok_or("Missing avatar material pixels")?;
            if !data.is_empty() && !frame.texture.apply_material_bytes(data) {
                return Err("Invalid avatar material pixels".into());
            }
        }
        if pixels.next().is_some() {
            return Err("Unexpected avatar material pixels".into());
        }
        Ok(avatar)
    }
}

fn validate_tile(tile: &Tile) -> Result<(), String> {
    if tile.module.is_some() || !tile.gameplay_tags.is_empty() {
        return Err("Shader/gameplay module in public tile".into());
    }
    if tile.textures.len() != tile.baked_material_data.len() || tile.textures.len() > 4096 {
        return Err("Invalid public tile frames".into());
    }
    for (t, data) in tile.textures.iter().zip(&tile.baked_material_data) {
        validate_texture(t)?;
        if !data.is_empty() && data.len() != t.width * t.height * 2 {
            return Err("Invalid public tile material pixels".into());
        }
    }
    Ok(())
}

fn model_reference(reference: &str) -> Result<String, String> {
    if reference.is_empty() || reference.len() > 256 || reference.chars().any(char::is_control) {
        return Err("Invalid public model reference".into());
    }
    Ok(reference.into())
}

fn restore_tile(tile: &mut Tile) {
    for (texture, pixels) in tile.textures.iter_mut().zip(&tile.baked_material_data) {
        if !pixels.is_empty() {
            texture.apply_material_bytes(pixels);
        }
    }
}
fn restore_paint(mut layer: crate::iso_paint::IsoPaintLayer) -> crate::iso_paint::IsoPaintLayer {
    for stamp in layer.chunks.values_mut().flat_map(|c| c.stamps.iter_mut()) {
        if let Some(tile) = &mut stamp.source_image {
            restore_tile(std::sync::Arc::make_mut(tile));
        }
    }
    layer
}

impl PublicRegion {
    pub fn into_region(self) -> Result<Region, String> {
        if policy::region_config(&self.config)? != self.config {
            return Err("Private region configuration".into());
        }
        for tile in self
            .paint
            .chunks
            .values()
            .flat_map(|c| &c.stamps)
            .filter_map(|s| s.source_image.as_ref())
        {
            validate_tile(tile)?;
        }
        let mut region = Region::new();
        region.id = self.id;
        region.name = self.name;
        region.config = self.config;
        region.map = self.layout.map(&region.name)?;
        region.iso_paint = restore_paint(self.paint);
        Ok(region)
    }
}
