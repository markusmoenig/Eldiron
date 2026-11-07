use super::*;
use denrim_materials::{geometry::Texture, integration::EditableMaterialAsset};
use std::collections::HashMap;
pub fn asset(
    spec: &Texture,
    palette: &ThePalette,
    project: Option<&Project>,
) -> Result<EditableMaterialAsset, String> {
    let mut asset = if let Some(bundle) = &spec.bundle {
        EditableMaterialAsset::reopen(bundle).map_err(|e| e.to_string())?
    } else if spec.source.starts_with("ruleset:") {
        ruleset_asset(&spec.source, palette, project)?
    } else if let Some(path) = spec.source.strip_prefix("embedded:") {
        let (folder, variant) = path
            .rsplit_once('/')
            .ok_or("Invalid embedded material source")?;
        let kind = folder.rsplit('/').next().unwrap();
        let source = crate::Embedded::get(&format!("{folder}/{kind}.material"))
            .ok_or("Embedded material is missing")?;
        let document = denrim_materials::format::parse(
            std::str::from_utf8(source.data.as_ref()).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let selected = document
            .result
            .as_deref()
            .or_else(|| document.materials.first().map(|m| m.id.as_str()))
            .ok_or("Material has no output")?;
        let mut a = EditableMaterialAsset::parse(
            &spec.source,
            std::str::from_utf8(source.data.as_ref()).unwrap(),
            selected,
        )
        .map_err(|e| e.to_string())?;
        crate::procedural_material::palette::ensure_color_bindings(&mut a, palette)?;
        let colors = crate::Embedded::get(&format!("{folder}/{variant}.palette.json"))
            .ok_or("Embedded palette bindings are missing")?;
        let data: serde_json::Value =
            serde_json::from_slice(colors.data.as_ref()).map_err(|e| e.to_string())?;
        for (op, params) in data["bindings"]
            .as_object()
            .ok_or("Missing color bindings")?
        {
            for (parameter, index) in params.as_object().ok_or("Invalid color binding")? {
                let index = index.as_u64().ok_or("Invalid palette index")? as u16;
                if palette
                    .colors
                    .get(index as usize)
                    .is_some_and(Option::is_some)
                {
                    a = crate::procedural_material::palette::bind_parameter(
                        &a, op, parameter, index, palette,
                    )?;
                }
            }
        }
        a
    } else {
        let id = Uuid::parse_str(&spec.source).map_err(|_| "Choose a procedural material")?;
        let bundle = project
            .and_then(|p| p.procedural_materials.get(&id))
            .ok_or("Texture material is missing")?;
        EditableMaterialAsset::reopen(bundle).map_err(|e| e.to_string())?
    };
    crate::procedural_material::palette::resolve(&mut asset, palette)?;
    Ok(asset)
}
/// Geometry refers to a ruleset resource, never to an appearance-owned copy.
/// Existing bitmap resources pass through MZ's image source; a linked MZ asset
/// takes precedence as ruleset artwork is migrated.
pub fn ruleset_choices(project: &Project) -> Vec<(String, String)> {
    let selection = shared::rulesets::selected_ruleset_config(&project.config);
    let mut choices =
        shared::rulesets::bundled_texture_assets_for_ruleset(&selection.id, &selection.version)
            .into_iter()
            .map(|a| (format!("ruleset:icon/{}/on", a.id), a.id.replace('_', " ")))
            .collect::<Vec<_>>();
    for a in
        shared::rulesets::bundled_icon_state_assets_for_ruleset(&selection.id, &selection.version)
    {
        let uri = format!("ruleset:icon/{}/{}", a.item_id, a.state);
        if !choices.iter().any(|(id, _)| id == &uri) {
            choices.push((
                uri,
                format!("{} · {}", a.item_id.replace('_', " "), a.state),
            ));
        }
    }
    for def in shared::ruleset_materials::definitions(&project.config) {
        choices.insert(
            0,
            (
                format!("ruleset:material/{}", def.id),
                format!("Material · {}", def.id.replace('-', " ")),
            ),
        );
    }
    choices
}
fn ruleset_asset(
    source: &str,
    palette: &ThePalette,
    project: Option<&Project>,
) -> Result<EditableMaterialAsset, String> {
    if let Some(name) = source.strip_prefix("ruleset:material/") {
        if let Some(project) = project {
            return shared::ruleset_materials::editable(project, name);
        }
        let selection =
            shared::rulesets::selected_ruleset_config(shared::rulesets::DEFAULT_RULESET_CONFIG);
        let def = shared::rulesets::bundled_material_assets_for_ruleset(
            &selection.id,
            &selection.version,
        )
        .into_iter()
        .find(|d| d.id == name)
        .ok_or("Ruleset material is missing")?;
        return rusterix::ruleset_material::editable(&def, palette);
    }
    let path = source
        .strip_prefix("ruleset:icon/")
        .ok_or("Unknown ruleset surface resource")?;
    let (item, state) = path
        .rsplit_once('/')
        .ok_or("Ruleset surface needs an item and state")?;
    let config = project.map_or(shared::rulesets::DEFAULT_RULESET_CONFIG, |p| {
        p.config.as_str()
    });
    let selection = shared::rulesets::selected_ruleset_config(config);
    let native_name = format!("icon/{path}");
    if let Some(def) = shared::ruleset_materials::definitions(config)
        .into_iter()
        .find(|d| d.id == native_name)
    {
        return if let Some(project) = project {
            shared::ruleset_materials::editable(project, def.id)
        } else {
            rusterix::ruleset_material::editable(&def, palette)
        };
    }
    let resource =
        shared::rulesets::bundled_icon_state_assets_for_ruleset(&selection.id, &selection.version)
            .into_iter()
            .find(|a| a.item_id == item && a.state == state);
    let still =
        shared::rulesets::bundled_texture_assets_for_ruleset(&selection.id, &selection.version)
            .into_iter()
            .find(|a| a.id == item && state == "on");
    if resource.is_none() && still.is_none() {
        return Err("Ruleset surface resource is missing".into());
    }
    let tile_id = resource
        .and_then(|a| a.tile_id)
        .and_then(|id| Uuid::parse_str(id).ok());
    if let Some(bundle) =
        project.and_then(|p| tile_id.and_then(|id| p.procedural_materials.get(&id)))
    {
        return EditableMaterialAsset::reopen(bundle).map_err(|e| e.to_string());
    }
    let mut frames =
        if let Some(tile) = project.and_then(|p| tile_id.and_then(|id| p.tiles.get(&id))) {
            tile.textures.clone()
        } else {
            let bytes = resource
                .map(|a| a.frames.to_vec())
                .unwrap_or_else(|| vec![still.unwrap().source]);
            bytes
                .into_iter()
                .map(|b| rusterix::Texture::from_image_safe(b).ok_or("Invalid ruleset image"))
                .collect::<Result<Vec<_>, _>>()?
        };
    for frame in &mut frames {
        shared::palette::fit_rgba(&mut frame.data, palette);
    }
    let first = frames.first().ok_or("Ruleset surface has no frames")?;
    let image = denrim_materials::image_source::ImageSource::rgba8(
        first.width,
        first.height,
        &frames.iter().map(|t| t.data.clone()).collect::<Vec<_>>(),
    )
    .map_err(|e| e.to_string())?;
    let mut a=EditableMaterialAsset::parse(source,"project 1\nmaterial \"surface\" called \"Ruleset surface\"\n  mode pbr\n  default roughness to 0.6\n  row \"image\"\n    use \"Image / Tile Source\" id \"ruleset-image\"\n  output base color from row \"image\" xyz\n  output opacity from row \"image\" w\n","surface").map_err(|e|e.to_string())?;
    a.root.project.materials[0].rows[0].tiles[0].image = Some(image);
    a.validate().map_err(|e| e.to_string())?;
    Ok(a)
}
pub struct Images {
    pub color: Vec<u8>,
    pub data: Vec<u8>,
    pub emission: Vec<u8>,
}
#[derive(Default)]
pub struct Cache {
    images: HashMap<String, Images>,
    prepared: HashMap<u64, EditableMaterialAsset>,
    #[cfg(not(target_arch = "wasm32"))]
    runtime: Option<denrim_materials::gpu::MaterialZ>,
    #[cfg(not(target_arch = "wasm32"))]
    instances: HashMap<String, denrim_materials::gpu::MaterialInstance>,
}
impl Cache {
    fn prepared(
        &mut self,
        spec: &Texture,
        palette: &ThePalette,
    ) -> Result<&EditableMaterialAsset, String> {
        let key = super::fingerprint(&format!("{spec:?}:{:?}", palette.colors));
        if !self.prepared.contains_key(&key) {
            let a = asset(spec, palette, None)?;
            if self.prepared.len() >= 64 {
                self.prepared.clear();
            }
            self.prepared.insert(key, a);
        }
        Ok(&self.prepared[&key])
    }
    pub fn frame(
        &mut self,
        spec: &Texture,
        palette: &ThePalette,
        seconds: f32,
    ) -> Result<(u32, f32), String> {
        let a = self.prepared(spec, palette)?;
        let animation = a
            .root
            .project
            .materials
            .iter()
            .find(|m| m.id == a.selected_material)
            .ok_or("Missing texture output")?
            .animation;
        let frame = if seconds.is_finite() {
            (seconds.max(0.).rem_euclid(animation.duration) / animation.duration
                * animation.frames as f32)
                .floor() as u32
                % animation.frames
        } else {
            0
        };
        Ok((frame, frame as f32 / animation.frames as f32))
    }
    pub fn image(
        &mut self,
        spec: &Texture,
        palette: &ThePalette,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        seconds: f32,
    ) -> Result<&Images, String> {
        let (frame, phase) = self.frame(spec, palette, seconds)?;
        let key = format!(
            "{}:{}:{:?}:{}:{frame}",
            spec.source,
            spec.size,
            palette.colors,
            spec.bundle.as_deref().unwrap_or("")
        );
        let library_size = Uuid::parse_str(&spec.source).ok().and_then(|id| {
            shared::ruleset_materials::resolution(shared::rulesets::DEFAULT_RULESET_CONFIG, &id)
        });
        if !self.images.contains_key(&key)
            && (spec.source.starts_with("ruleset:material/")
                || spec.source.starts_with("ruleset:icon/")
                || library_size.is_some())
        {
            let material = self.prepared(spec, palette)?.clone();
            let size = library_size.unwrap_or(if spec.source.starts_with("ruleset:icon/") {
                32
            } else {
                spec.size
            });
            let texture =
                rusterix::ruleset_material::bake_at_time(&material, palette, size, phase)?
                    .resized(spec.size as usize, spec.size as usize);
            let mut data = Vec::with_capacity(texture.data.len());
            let mut emission = Vec::with_capacity(texture.data.len());
            for i in 0..spec.size * spec.size {
                let (roughness, metallic, _, light) =
                    texture.get_materials(i % spec.size, i / spec.size);
                data.extend([
                    (roughness * 255.).round() as u8,
                    (metallic * 255.).round() as u8,
                    0,
                    255,
                ]);
                // SceneVM's packed emission is a scalar applied to the albedo.
                for c in &texture.data[i as usize * 4..i as usize * 4 + 3] {
                    let c = *c as f32 / 255.;
                    let linear = if c <= 0.04045 {
                        c / 12.92
                    } else {
                        ((c + 0.055) / 1.055).powf(2.4)
                    } * light;
                    let srgb = if linear <= 0.0031308 {
                        linear * 12.92
                    } else {
                        1.055 * linear.powf(1. / 2.4) - 0.055
                    };
                    emission.push((srgb.clamp(0., 1.) * 255.).round() as u8);
                }
                emission.push(255);
            }
            if self.images.len() >= 256 {
                self.images.clear();
            }
            self.images.insert(
                key.clone(),
                Images {
                    color: texture.data,
                    data,
                    emission,
                },
            );
        }
        if !self.images.contains_key(&key) {
            #[cfg(not(target_arch = "wasm32"))]
            {
                use denrim_materials::{gpu::MaterialZ, integration::InstanceRecord};
                if self.runtime.is_none() {
                    self.runtime = Some(
                        pollster::block_on(MaterialZ::new(device, queue))
                            .map_err(|e| e.to_string())?,
                    );
                }
                let a = self.prepared(spec, palette)?.clone();
                let runtime = self.runtime.as_ref().unwrap();
                let reuse = self
                    .instances
                    .get_mut(&spec.source)
                    .is_some_and(|instance| instance.update_asset(&a).is_ok());
                if !reuse {
                    if self.instances.len() >= 16 {
                        self.instances.clear();
                    }
                    let compiled = pollster::block_on(
                        runtime.compile(&a, InstanceRecord::new("geometry-texture")),
                    )
                    .map_err(|e| e.to_string())?;
                    self.instances.insert(spec.source.clone(), compiled);
                }
                let compiled = &self.instances[&spec.source];
                let maps = crate::procedural_material::gpu::render_grid_time(
                    runtime,
                    compiled,
                    device,
                    queue,
                    [spec.size as usize; 2],
                    [1.; 2],
                    [-0.5; 2],
                    phase,
                )?;
                let images = crate::procedural_material::gpu::preview_images(&maps);
                let mut data = vec![];
                for i in 0..(spec.size * spec.size) as usize {
                    data.extend([images[2][i * 4], images[3][i * 4], 0, 255]);
                }
                if self.images.len() >= 256 {
                    self.images.clear();
                }
                self.images.insert(
                    key.clone(),
                    Images {
                        color: images[0].clone(),
                        data,
                        emission: images[4].clone(),
                    },
                );
            }
            #[cfg(target_arch = "wasm32")]
            return Err("Material textures require the desktop GPU renderer".into());
        }
        self.images.get(&key).ok_or("Texture bake failed".into())
    }
}
pub fn resolve_evaluation(e: &mut Evaluation, project: &Project) -> Result<(), String> {
    let mut bundles = HashMap::<String, String>::new();
    let mut resolve = |mesh: &mut denrim_materials::geometry::Mesh| -> Result<(), String> {
        for p in &mut mesh.parts {
            if let Some(t) = &mut p.texture {
                if t.source.starts_with("ruleset:") {
                    if !bundles.contains_key(&t.source) {
                        bundles.insert(
                            t.source.clone(),
                            ruleset_asset(&t.source, &project.palette, Some(project))?
                                .save()
                                .map_err(|e| e.to_string())?,
                        );
                    }
                    t.bundle = Some(bundles[&t.source].clone());
                } else if t.bundle.is_none() && !t.source.starts_with("embedded:") {
                    let id = Uuid::parse_str(&t.source).map_err(|_| "Invalid texture source")?;
                    t.bundle = Some(
                        project
                            .procedural_materials
                            .get(&id)
                            .cloned()
                            .ok_or("Texture material is missing")?,
                    );
                }
            }
        }
        Ok(())
    };
    resolve(&mut e.mesh)?;
    for v in e.operations.values_mut().chain(e.rows.values_mut()) {
        if let Value::Mesh(m) = v {
            resolve(m)?;
        }
    }
    Ok(())
}

pub fn input_key(project: &Project, source: &str) -> u64 {
    let mut keys = Vec::new();
    for a in shared::rulesets::bundled_icon_state_assets() {
        if source.contains(&format!("ruleset:icon/{}/{}", a.item_id, a.state)) {
            if let Some(id) = a.tile_id.and_then(|id| Uuid::parse_str(id).ok()) {
                let bundle = project.procedural_materials.get(&id);
                let images = project.tiles.get(&id).map(|t| {
                    t.textures
                        .iter()
                        .map(|t| (&t.data, &t.data_ext))
                        .collect::<Vec<_>>()
                });
                keys.push(super::fingerprint(&(id, bundle, images)));
            }
        }
    }
    for d in shared::ruleset_materials::definitions(&project.config) {
        if source.contains(&format!("ruleset:material/{}", d.id))
            || (d.id.starts_with("icon/") && source.contains(&format!("ruleset:{}", d.id)))
        {
            let id = shared::rulesets::material_asset_id(d.id);
            keys.push(super::fingerprint(&(
                id,
                project.procedural_materials.get(&id),
            )));
        }
    }
    super::fingerprint(&keys)
}
