//! Live geometry-to-image sources. The portable MZ tile keeps its last rendered
//! pixels; Eldiron owns the model reference and refreshes it off the UI thread.
use super::*;
use denrim_materials::{
    geometry::{Document, Value},
    image_source::ImageSource,
};
use std::sync::{Arc, mpsc};

pub const SOURCE: &str = "eldiron:model-render";
const PREFIX: &str = "eldiron:model:";

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub model: Uuid,
    pub yaw: f32,
    pub pitch: f32,
    pub phase: f32,
    pub clip: String,
    pub size: u32,
    pub frames: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendered_key: Option<u64>,
}
impl Spec {
    pub fn new(model: Uuid) -> Self {
        Self {
            model,
            yaw: -25.,
            pitch: 15.,
            phase: 0.,
            clip: String::new(),
            size: 32,
            frames: 1,
            rendered_key: None,
        }
    }
    fn reference(&self) -> Result<String, String> {
        if !(16..=256).contains(&self.size)
            || !(1..=16).contains(&self.frames)
            || !self.yaw.is_finite()
            || !self.pitch.is_finite()
            || !self.phase.is_finite()
            || self.yaw.abs() > 180.
            || self.pitch.abs() > 90.
            || !(0. ..=1.).contains(&self.phase)
        {
            return Err("Invalid model render camera, phase or image size".into());
        }
        Ok(format!(
            "{PREFIX}{}",
            serde_json::to_string(self).map_err(|e| e.to_string())?
        ))
    }
}
pub fn spec(image: &ImageSource) -> Option<Spec> {
    image
        .reference
        .as_deref()
        .and_then(|r| r.strip_prefix(PREFIX))
        .and_then(|r| serde_json::from_str(r).ok())
}
pub fn has_source(bundle: &str) -> bool {
    bundle.contains(PREFIX)
}
fn image(asset: &EditableMaterialAsset, operation: &str) -> Option<ImageSource> {
    asset
        .root
        .project
        .materials
        .iter()
        .flat_map(|m| &m.rows)
        .flat_map(|r| &r.tiles)
        .find(|t| t.id == operation)
        .and_then(|t| t.image.clone())
}
pub fn assign(
    asset: &EditableMaterialAsset,
    operation: &str,
    mut settings: Spec,
) -> Result<EditableMaterialAsset, String> {
    settings.rendered_key = None;
    let mut source = image(asset, operation)
        .unwrap_or(ImageSource::rgba8(1, 1, &[vec![0; 4]]).map_err(|e| e.to_string())?);
    source.reference = Some(settings.reference()?);
    let mut next = image_source::set(asset, operation, Some(source))?;
    if let Some(m) = next
        .root
        .project
        .materials
        .iter_mut()
        .find(|m| m.id == next.selected_material)
    {
        m.animation.frames = m.animation.frames.max(settings.frames);
    }
    next.validate().map_err(|e| e.to_string())?;
    Ok(next)
}
pub fn choices(project: &Project) -> Vec<(String, String)> {
    #[derive(serde::Deserialize)]
    struct Label {
        label: String,
    }
    let mut choices = project
        .procedural_appearances
        .iter()
        .filter_map(|(id, source)| {
            serde_json::from_str::<Label>(source)
                .ok()
                .map(|m| (format!("@model:{id}"), format!("Model · {}", m.label)))
        })
        .collect::<Vec<_>>();
    choices.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    choices
}
fn clips(project: &Project, model: Uuid) -> Vec<(String, String)> {
    let mut result = vec![(String::new(), "Rest".into())];
    if let Some(doc) = project
        .procedural_appearances
        .get(&model)
        .and_then(|s| Document::parse(s).ok())
    {
        result.extend(
            doc.rows
                .iter()
                .filter(|r| {
                    r.tiles.iter().any(|t| {
                        matches!(t.op, denrim_materials::geometry::Operation::Motion { .. })
                    })
                })
                .map(|r| (r.id.clone(), r.label.clone())),
        );
    }
    result
}
pub fn decorate(view: &mut MaterialEditorView, asset: &EditableMaterialAsset, project: &Project) {
    view.sources.push(MaterialSourceView {
        id: SOURCE.into(),
        label: "Model Render".into(),
        category: "Geometry".into(),
    });
    view.image_sources.extend(choices(project));
    for tile in view.rows.iter_mut().flat_map(|r| &mut r.operations) {
        let Some(settings) = image(asset, &tile.id).as_ref().and_then(spec) else {
            continue;
        };
        tile.label = "Model Render".into();
        if let Some((_, label)) = view
            .image_bindings
            .iter_mut()
            .find(|(id, _)| id == &tile.id)
        {
            *label = view
                .image_sources
                .iter()
                .find(|(id, _)| id == &format!("@model:{}", settings.model))
                .map_or(
                    "Missing model · last image retained".into(),
                    |(_, label)| label.clone(),
                );
        }
        let motion = clips(project, settings.model);
        for (id, label, value, min, max, options) in [
            ("yaw", "Camera angle", settings.yaw, -180., 180., vec![]),
            (
                "pitch",
                "Camera elevation",
                settings.pitch,
                -90.,
                90.,
                vec![],
            ),
            ("phase", "Pose", settings.phase, 0., 1., vec![]),
            ("size", "Pixels", settings.size as f32, 16., 256., vec![]),
            ("frames", "Frames", settings.frames as f32, 1., 16., vec![]),
            (
                "clip",
                "Animation",
                motion
                    .iter()
                    .position(|(id, _)| id == &settings.clip)
                    .unwrap_or(0) as f32,
                0.,
                motion.len().saturating_sub(1) as f32,
                motion.into_iter().map(|(_, label)| label).collect(),
            ),
        ] {
            let parameter = format!("@model-{id}:0");
            tile.parameters.push(MaterialNumberView {
                id: parameter.clone(),
                label: label.into(),
                value,
                min,
                max,
            });
            tile.properties.push(MaterialPropertyView {
                id: format!("@model-{id}"),
                label: label.into(),
                group: "Model Render".into(),
                kind: if options.is_empty() {
                    MaterialPropertyKind::Number
                } else {
                    MaterialPropertyKind::Choice
                },
                components: vec![parameter],
                choices: options,
                tooltip: String::new(),
            });
        }
    }
}
pub fn edit(
    asset: &EditableMaterialAsset,
    operation: &str,
    parameter: &str,
    value: f32,
    project: &Project,
) -> Result<EditableMaterialAsset, String> {
    let mut settings = image(asset, operation)
        .as_ref()
        .and_then(spec)
        .ok_or("Select a model render source")?;
    if !value.is_finite() {
        return Err("Model render value must be finite".into());
    }
    match parameter.trim_end_matches(":0").strip_prefix("@model-") {
        Some("yaw") => settings.yaw = value,
        Some("pitch") => settings.pitch = value,
        Some("phase") => settings.phase = value,
        Some("size") => settings.size = value.round() as u32,
        Some("frames") => settings.frames = value.round() as u32,
        Some("clip") => {
            settings.clip = clips(project, settings.model)
                .get(value.round() as usize)
                .ok_or("Animation is missing")?
                .0
                .clone()
        }
        _ => return Err("Unknown model render setting".into()),
    }
    assign(asset, operation, settings)
}
#[derive(Clone)]
pub struct Request {
    pub tile: Uuid,
    pub operation: String,
    pub key: u64,
    pub settings: Spec,
    pub project: Arc<Project>,
}
pub struct Completed {
    pub tile: Uuid,
    pub operation: String,
    pub key: u64,
    pub result: Result<ImageSource, String>,
}
fn duration(project: &Project, tile: Uuid) -> f32 {
    project
        .procedural_materials
        .get(&tile)
        .and_then(|b| EditableMaterialAsset::reopen(b).ok())
        .and_then(|a| {
            a.root
                .project
                .materials
                .into_iter()
                .find(|m| m.id == a.selected_material)
        })
        .map_or(2., |m| m.animation.duration)
}
pub fn key(settings: &Spec, project: &Project, tile: Uuid) -> u64 {
    // Pixels generated by model renders are not inputs to other model renders.
    // Texture feedback is rejected by the worker. Geometry/material/palette edits
    // invalidate all affected model images; generated pixels cannot cause a loop.
    let mut geometry = project.procedural_appearances.iter().collect::<Vec<_>>();
    geometry.sort_by_key(|(id, _)| **id);
    let mut surfaces = project
        .procedural_materials
        .iter()
        .filter(|(id, b)| **id != tile && !has_source(b))
        .collect::<Vec<_>>();
    surfaces.sort_by_key(|(id, _)| **id);
    let mut settings = settings.clone();
    settings.rendered_key = None;
    rusterix::procedural_avatar::fingerprint(&(
        serde_json::to_string(&settings).unwrap(),
        duration(project, tile).to_bits(),
        geometry,
        surfaces,
        serde_json::to_string(&project.palette.colors).unwrap(),
    ))
}
pub struct Worker {
    jobs: mpsc::Sender<Request>,
    pub results: mpsc::Receiver<Completed>,
}
#[derive(Default)]
pub struct State {
    worker: Option<Worker>,
    input: Option<u64>,
    requested: std::collections::HashMap<(Uuid, String), u64>,
    pending: std::collections::HashMap<(Uuid, String), u64>,
}
impl State {
    pub fn pending(&self, tile: Uuid) -> bool {
        self.pending.keys().any(|(id, _)| *id == tile)
    }
    pub fn update(&mut self, project: &mut Project, ctx: &mut TheContext) -> bool {
        let mut changed = false;
        let completed = self
            .worker
            .as_ref()
            .map(|w| w.results.try_iter().collect::<Vec<_>>())
            .unwrap_or_default();
        for result in completed {
            let target = (result.tile, result.operation.clone());
            if self.pending.get(&target) != Some(&result.key) {
                continue;
            }
            self.pending.remove(&target);
            let Some(bundle) = project.procedural_materials.get(&result.tile) else {
                continue;
            };
            let Ok(asset) = EditableMaterialAsset::reopen(bundle) else {
                continue;
            };
            let Some(settings) = image(&asset, &result.operation).as_ref().and_then(spec) else {
                continue;
            };
            if key(&settings, project, result.tile) != result.key {
                continue;
            }
            match result.result.and_then(|rendered| {
                if image(&asset, &result.operation).as_ref() == Some(&rendered) {
                    return Ok(None);
                }
                image_source::set(&asset, &result.operation, Some(rendered))?
                    .save()
                    .map(Some)
                    .map_err(|e| e.to_string())
            }) {
                Ok(Some(bundle)) => {
                    project.procedural_materials.insert(result.tile, bundle);
                    changed = true;
                }
                Ok(None) => (),
                Err(error) => ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    format!("Model render: {error}"),
                )),
            }
        }
        let input = image_source::input_key(project);
        if self.input == Some(input) {
            return changed;
        }
        self.input = Some(input);
        let mut requests = vec![];
        let mut active = std::collections::HashSet::new();
        for (tile, bundle) in &project.procedural_materials {
            if project.material_compositions.contains_key(tile) || !has_source(bundle) {
                continue;
            }
            let Ok(asset) = EditableMaterialAsset::reopen(bundle) else {
                continue;
            };
            for operation in asset
                .root
                .project
                .materials
                .iter()
                .filter(|m| m.id == asset.selected_material)
                .flat_map(|m| &m.rows)
                .flat_map(|r| &r.tiles)
            {
                let Some(settings) = operation.image.as_ref().and_then(spec) else {
                    continue;
                };
                let key = key(&settings, project, *tile);
                let target = (*tile, operation.id.clone());
                active.insert(target.clone());
                if settings.rendered_key == Some(key)
                    && operation.image.as_ref().is_some_and(|image| {
                        image.width == settings.size as usize
                            && image.height == settings.size as usize
                            && image.frame_count() == settings.frames as usize
                    })
                {
                    self.pending.remove(&target);
                    self.requested.insert(target, key);
                    continue;
                }
                if self.requested.get(&target) == Some(&key) {
                    continue;
                }
                requests.push((*tile, operation.id.clone(), key, settings));
            }
        }
        self.pending.retain(|target, _| active.contains(target));
        self.requested.retain(|target, _| active.contains(target));
        if requests.is_empty() {
            return changed;
        }
        if self.worker.is_none() {
            let Some((device, queue)) = RUSTERIX.read().unwrap().scene_handler.vm.gpu_context()
            else {
                self.input = None;
                return changed;
            };
            self.worker = Some(Worker::new(device, queue));
        }
        let snapshot = Arc::new(project.clone());
        for (tile, operation, key, settings) in requests {
            self.requested.insert((tile, operation.clone()), key);
            self.pending.insert((tile, operation.clone()), key);
            self.worker.as_ref().unwrap().send(Request {
                tile,
                operation,
                key,
                settings,
                project: snapshot.clone(),
            });
        }
        changed
    }
}
impl Worker {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let (jobs, incoming) = mpsc::channel::<Request>();
        let (done, results) = mpsc::channel();
        std::thread::Builder::new()
            .name("MZ model images".into())
            .spawn(move || {
                let renderer = crate::geometry_editor::preview::Renderer::new(&device);
                let mut pending = std::collections::VecDeque::new();
                loop {
                    if pending.is_empty() {
                        match incoming.recv() {
                            Ok(r) => pending.push_back(r),
                            Err(_) => break,
                        }
                    }
                    while let Ok(r) = incoming.try_recv() {
                        pending.retain(|old: &Request| {
                            old.tile != r.tile || old.operation != r.operation
                        });
                        pending.push_back(r);
                    }
                    let request = pending.pop_front().unwrap();
                    let result = render(&request, &renderer, &device, &queue);
                    if done
                        .send(Completed {
                            tile: request.tile,
                            operation: request.operation,
                            key: request.key,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("model image worker");
        Self { jobs, results }
    }
    pub fn send(&self, request: Request) {
        let _ = self.jobs.send(request);
    }
}
fn render(
    request: &Request,
    renderer: &crate::geometry_editor::preview::Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<ImageSource, String> {
    let settings = &request.settings;
    settings.reference()?;
    let source = request
        .project
        .procedural_appearances
        .get(&settings.model)
        .ok_or("The model render source was removed; retaining its last image")?;
    let doc = Document::parse(source).map_err(|e| e.to_string())?;
    let doc = shared::appearance_assets::resolve_dependencies(&request.project, &doc)?;
    let mut evaluation = doc.evaluate().map_err(|e| e.to_string())?;
    crate::geometry_editor::textures::resolve_evaluation(&mut evaluation, &request.project)?;
    for texture in evaluation
        .mesh
        .parts
        .iter()
        .filter_map(|p| p.texture.as_ref())
    {
        let same = texture.source == request.tile.to_string()
            || texture
                .source
                .strip_prefix("ruleset:")
                .is_some_and(|name| shared::rulesets::material_asset_id(name) == request.tile);
        if same
            || texture
                .bundle
                .as_ref()
                .is_some_and(|bundle| has_source(bundle))
        {
            return Err("Model render texture feedback: use a separate surface material, not a model-render icon, on this model".into());
        }
    }
    let motion = if settings.clip.is_empty() {
        None
    } else {
        Some(match evaluation.rows.get(&settings.clip) {
            Some(Value::Motion(m)) => m,
            _ => return Err("The model animation was removed".into()),
        })
    };
    let framing = motion.map(|m| crate::geometry_editor::animation::bounds(&evaluation.mesh, &[m]));
    let mut frames = vec![];
    let surface_duration = duration(&request.project, request.tile);
    for frame in 0..settings.frames {
        let phase = settings.phase + frame as f32 / settings.frames as f32;
        let mesh = if let Some(motion) = motion {
            evaluation
                .mesh
                .posed(Some(motion), phase * motion.duration)
                .map_err(|e| e.to_string())?
                .mesh
        } else {
            evaluation.mesh.clone()
        };
        let job = crate::geometry_editor::preview::Job {
            surface_seconds: phase * surface_duration,
            revision: 0,
            mesh,
            palette: request.project.palette.clone(),
            width: settings.size,
            height: settings.size,
            yaw: settings.yaw,
            pitch: settings.pitch,
            background: [0; 4],
            grip: None,
            axes: [[0; 4]; 3],
            framing,
            bones: vec![],
            attachment_frames: vec![],
        };
        let mut rgba = renderer.render(device, queue, &job)?.pixels().to_vec();
        shared::palette::fit_rgba(&mut rgba, &request.project.palette);
        frames.push(rgba);
    }
    let mut result = ImageSource::rgba8(settings.size as usize, settings.size as usize, &frames)
        .map_err(|e| e.to_string())?;
    let mut completed = settings.clone();
    completed.rendered_key = Some(request.key);
    result.reference = Some(completed.reference()?);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Project, Uuid, Uuid, EditableMaterialAsset) {
        let mut p = Project::default();
        let model = Uuid::new_v4();
        p.procedural_appearances
            .insert(model, denrim_materials::geometry::SWORD_EXAMPLE.into());
        let tile = Uuid::new_v4();
        let a=EditableMaterialAsset::parse(&tile.to_string(), "project 1\nmaterial \"surface\" called \"Model icon\"\n  mode pbr\n  row \"image\"\n    use \"Image / Tile Source\" id \"model\"\n  output base color from row \"image\" xyz\n  output opacity from row \"image\" w\n", "surface").unwrap();
        let a = assign(&a, "model", Spec::new(model)).unwrap();
        p.procedural_materials.insert(tile, a.save().unwrap());
        (p, model, tile, a)
    }
    #[test]
    fn model_image_settings_survive_save_and_caches_ignore_derived_pixels() {
        let (mut p, model, tile, a) = fixture();
        let settings = spec(&image(&a, "model").unwrap()).unwrap();
        let before = key(&settings, &p, tile);
        let mut rendered = settings.clone();
        rendered.rendered_key = Some(before);
        assert_eq!(before, key(&rendered, &p, tile));
        let mut pixels = ImageSource::rgba8(32, 32, &[vec![0; 32 * 32 * 4]]).unwrap();
        pixels.reference = Some(rendered.reference().unwrap());
        let saved = image_source::set(&a, "model", Some(pixels))
            .unwrap()
            .save()
            .unwrap();
        p.procedural_materials.insert(tile, saved);
        let mut state = State::default();
        let mut ctx = TheContext::new(800, 600, 1.);
        assert!(!state.update(&mut p, &mut ctx));
        assert!(state.worker.is_none(), "Reopen must use saved model pixels");
        assert!(!state.pending(tile));
        state
            .pending
            .insert((tile, "model".into()), before.wrapping_add(1));
        state.input = None;
        state.update(&mut p, &mut ctx);
        assert!(
            !state.pending(tile),
            "Undo to a saved image cancels stale pending work immediately"
        );
        p.procedural_appearances.get_mut(&model).unwrap().push('\n');
        assert_ne!(before, key(&settings, &p, tile));
        let edited = edit(&a, "model", "@model-frames:0", 4., &p).unwrap();
        assert_eq!(edited.root.project.materials[0].animation.frames, 4);
        assert!(edit(&a, "model", "@model-size:0", f32::NAN, &p).is_err());
    }
    #[test]
    #[ignore = "native GPU model-to-material rendering"]
    fn model_render_gpu_has_transparency_palette_colors_and_downstream_material_output() {
        let (p, _, tile, a) = fixture();
        let instance = wgpu::Instance::default();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .unwrap();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let renderer = crate::geometry_editor::preview::Renderer::new(&device);
        let settings = spec(&image(&a, "model").unwrap()).unwrap();
        let input = key(&settings, &p, tile);
        let source = render(
            &Request {
                tile,
                operation: "model".into(),
                key: input,
                settings,
                project: Arc::new(p.clone()),
            },
            &renderer,
            &device,
            &queue,
        )
        .unwrap();
        assert_eq!(source.width, 32);
        assert_eq!(spec(&source).unwrap().rendered_key, Some(input));
        let expected_alpha = source
            .planes
            .iter()
            .find(|p| p.channel == "color")
            .unwrap()
            .frames[0]
            .as_bytes()
            .chunks_exact(8)
            .map(|pixel| {
                u8::from_str_radix(std::str::from_utf8(&pixel[6..8]).unwrap(), 16).unwrap()
            })
            .collect::<Vec<_>>();
        assert!(
            expected_alpha.iter().filter(|a| **a > 200).count() > 20,
            "The source must contain a complete model"
        );
        let next = image_source::set(&a, "model", Some(source)).unwrap();
        let texture = shared::ruleset_materials::bake_asset(&next, &p.palette, 32).unwrap();
        assert_eq!(
            expected_alpha,
            texture
                .data
                .chunks_exact(4)
                .map(|p| p[3])
                .collect::<Vec<_>>(),
            "MZ must preserve the model image's framing and coverage"
        );
        assert!(texture.data.chunks_exact(4).any(|p| p[3] == 0));
        assert!(texture.data.chunks_exact(4).any(|p| p[3] > 200));
        if let Ok(path) = std::env::var("ELDIRON_MODEL_SOURCE_PREVIEW") {
            let img = TheRGBABuffer::from(texture.data, 32, 32);
            std::fs::write(path, img.to_png().unwrap()).unwrap();
        }
        // The modelling viewport must select animated surface frames too, even
        // when neither the model geometry nor its camera changes.
        let mut animated = a.clone();
        let pixels = ImageSource::rgba8(
            2,
            2,
            &[
                vec![255, 0, 0, 255].repeat(4),
                vec![0, 0, 255, 255].repeat(4),
            ],
        )
        .unwrap();
        animated = image_source::set(&animated, "model", Some(pixels)).unwrap();
        animated.root.project.materials[0].animation.duration = 1.;
        let texture = denrim_materials::geometry::Texture {
            source: Uuid::new_v4().to_string(),
            bundle: Some(animated.save().unwrap()),
            size: 16,
        };
        let mut cache = crate::geometry_editor::textures::Cache::default();
        let first = cache
            .image(&texture, &p.palette, &device, &queue, 0.1)
            .unwrap()
            .color
            .clone();
        let last = cache
            .image(&texture, &p.palette, &device, &queue, 0.6)
            .unwrap()
            .color
            .clone();
        assert_ne!(first, last);
        assert_eq!(
            first,
            cache
                .image(&texture, &p.palette, &device, &queue, 1.1)
                .unwrap()
                .color
        );
    }
}
