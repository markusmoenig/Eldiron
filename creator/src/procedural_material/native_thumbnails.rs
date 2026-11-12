//! Bounded GPU-only graph preview cache. Source documents remain CPU assets;
//! output colors never cross a readback/upload bridge for normal display.
use super::{
    native_preview::{Preview, Request},
    operation_preview_asset,
};
use denrim_materials::integration::EditableMaterialAsset;
use renderkit::{ImageHandle, ImageRegistry};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
// At most 8192 immutable 64x64 RGBA outputs = 128 MiB, plus one pending evaluation.
// This is a provisional retention limit; M0 release budgets remain open.
const MAX_IMAGES: usize = 8192;
struct Entry {
    bundle: String,
    asset: Arc<EditableMaterialAsset>,
    image: Option<ImageHandle>,
    ready: bool,
    failure: Option<String>,
}
#[derive(Default)]
pub(super) struct Cache {
    owner: Option<u64>,
    source: Option<(String, EditableMaterialAsset)>,
    entries: HashMap<String, Entry>,
    invalid: HashMap<String, String>,
    source_failure: Option<(String, String)>,
    producer: Option<Preview>,
    retired: Vec<ImageHandle>,
}
pub(super) struct Output {
    pub handles: HashMap<String, ImageHandle>,
    pub waiting: bool,
    pub errors: Vec<String>,
}
impl Cache {
    pub fn invalidate(&mut self) {
        self.producer = None;
        for entry in self.entries.values_mut() {
            self.retired.extend(entry.image.take());
            entry.ready = false;
            entry.failure = None;
        }
    }
    pub fn clear(&mut self) {
        self.invalidate();
        self.source = None;
        self.entries.clear();
        self.invalid.clear();
        self.source_failure = None;
    }
    pub fn prepare(
        &mut self,
        images: &mut ImageRegistry,
        bundle: Option<&str>,
        keys: &[String],
    ) -> Result<Output, String> {
        if self.owner != Some(images.owner_id()) {
            self.invalidate();
            self.retired.clear(); // The previous domain owns its handles.
            self.owner = Some(images.owner_id());
        }
        for handle in self.retired.drain(..) {
            images.remove(handle);
        }
        if keys.len() > MAX_IMAGES {
            return Err("Visible material thumbnails exceed the 128 MiB output budget".into());
        }
        let keys: Vec<_> = keys
            .iter()
            .filter(|k| !k.is_empty())
            .take(MAX_IMAGES)
            .cloned()
            .collect();
        let active: HashSet<_> = keys.iter().cloned().collect();
        self.invalid.retain(|key, _| active.contains(key));
        self.entries.retain(|key, entry| {
            if active.contains(key) {
                true
            } else {
                if let Some(handle) = entry.image {
                    images.remove(handle);
                }
                false
            }
        });
        let Some(bundle) = bundle.filter(|_| !keys.is_empty()) else {
            self.producer = None;
            if bundle.is_none() {
                for entry in self.entries.values() {
                    if let Some(handle) = entry.image {
                        images.remove(handle);
                    }
                }
                self.entries.clear();
            }
            return Ok(self.output());
        };
        let changed = self.source.as_ref().is_none_or(|(old, _)| old != bundle);
        if changed {
            if let Some((_, error)) = self
                .source_failure
                .as_ref()
                .filter(|(failed, _)| failed == bundle)
            {
                return Err(error.clone());
            }
            let result = (|| {
                let asset = EditableMaterialAsset::reopen(bundle).map_err(|e| e.to_string())?;
                asset.validate().map_err(|e| e.to_string())?;
                Ok::<_, String>(asset)
            })();
            match result {
                Ok(asset) => {
                    if self.source.as_ref().is_some_and(|(_, old)| {
                        old.root.id != asset.root.id
                            || old.selected_material != asset.selected_material
                    }) {
                        self.producer = None;
                        for entry in self.entries.values() {
                            if let Some(handle) = entry.image {
                                images.remove(handle);
                            }
                        }
                        self.entries.clear();
                    }
                    self.source = Some((bundle.to_owned(), asset));
                    self.source_failure = None;
                    self.invalid.clear();
                }
                Err(error) => {
                    self.source_failure = Some((bundle.to_owned(), error.clone()));
                    return Err(error);
                }
            }
        }
        let mut errors = Vec::new();
        for key in &keys {
            if self.invalid.contains_key(key) || (!changed && self.entries.contains_key(key)) {
                continue;
            }
            let resolved = (|| {
                let asset = operation_preview_asset(&self.source.as_ref().unwrap().1, key)?;
                let bundle = asset.save().map_err(|e| e.to_string())?;
                Ok::<_, String>((asset, bundle))
            })();
            match resolved {
                Ok((asset, bundle)) => {
                    if self
                        .entries
                        .get(key)
                        .is_some_and(|old| old.bundle == bundle)
                    {
                        continue;
                    }
                    let image = self.entries.get(key).and_then(|old| old.image);
                    self.entries.insert(
                        key.clone(),
                        Entry {
                            bundle,
                            asset: Arc::new(asset),
                            image,
                            ready: false,
                            failure: None,
                        },
                    );
                }
                Err(error) => {
                    errors.push(format!("Operation thumbnail {key}: {error}"));
                    self.invalid.insert(key.clone(), error.clone());
                    if let Some(entry) = self.entries.get_mut(key) {
                        entry.failure = Some(error);
                    }
                }
            }
        }
        // Advance at most one operation per application frame. Compilation,
        // status readiness, and immutable publication share this injected GPU.
        if let Some(key) = keys.iter().find(|key| {
            self.entries
                .get(*key)
                .is_some_and(|e| !e.ready && e.failure.is_none())
        }) {
            if self.producer.is_none() {
                self.producer = Some(Preview::thumbnails(images.context().clone())?);
            }
            let entry = self.entries.get_mut(key).unwrap();
            let request = Request {
                asset: entry.asset.clone(),
                time: 0.,
                origin: Some([-0.5; 2]),
                pixels: 0,
                continuous: true,
                channel: 0,
                size: [64; 2],
                checker: [[0; 4]; 2],
            };
            match self.producer.as_mut().unwrap().render(&request, 1.) {
                Ok(Some(frame)) => {
                    let handle = images
                        .insert(frame.image.clone())
                        .map_err(|e| e.to_string())?;
                    self.producer
                        .as_mut()
                        .unwrap()
                        .transfer_output(&frame.image);
                    if let Some(old) = entry.image.replace(handle) {
                        images.remove(old);
                    }
                    entry.ready = true;
                }
                Ok(None) => {}
                Err(error) => {
                    errors.push(format!("Operation thumbnail {key}: {error}"));
                    entry.failure = Some(error);
                }
            }
        }
        let mut output = self.output();
        output.errors = errors;
        Ok(output)
    }
    #[cfg(test)]
    pub(super) fn handles(&self) -> HashMap<String, ImageHandle> {
        self.output().handles
    }
    fn output(&self) -> Output {
        Output {
            handles: self
                .entries
                .iter()
                .filter_map(|(key, entry)| entry.image.map(|handle| (key.clone(), handle)))
                .collect(),
            waiting: self
                .entries
                .values()
                .any(|entry| !entry.ready && entry.failure.is_none()),
            errors: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderkit::{GpuContext, ReadbackTicket};
    fn wait(gpu: &GpuContext) {
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
    }
    fn complete(
        cache: &mut Cache,
        images: &mut ImageRegistry,
        bundle: &str,
        keys: &[String],
    ) -> Output {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            let output = cache.prepare(images, Some(bundle), keys).unwrap();
            assert!(output.errors.is_empty(), "{:?}", output.errors);
            if !output.waiting {
                return output;
            }
            assert!(std::time::Instant::now() < deadline);
            wait(images.context());
        }
    }
    #[test]
    #[ignore = "requires native GPU"]
    fn native_material_graph_thumbnails_match_reference_remain_resident_and_recover() {
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let id = theframework::prelude::Uuid::new_v4();
        let source=super::super::STARTER.replace("  row \"grain\"", "  row \"alpha\"\n    use \"Color\" id \"alpha\"\n      set \"color\" to 0.8 0.1 0.4 0.3\n  row \"grain\"");
        let asset = EditableMaterialAsset::parse("thumbnail-review", &source, "surface").unwrap();
        let bundle = asset.save().unwrap();
        let keys: Vec<_> = asset.root.project.materials[0]
            .rows
            .iter()
            .flat_map(|r| &r.tiles)
            .map(|t| t.id.clone())
            .collect();
        assert!(
            keys.len() > 3,
            "exercise immutable ownership beyond the viewport lease pool"
        );
        let reference = super::super::gpu::Worker::new(gpu.device.clone(), gpu.queue.clone());
        reference.send(super::super::gpu::Job {
            pixels_per_tile: 16,
            tile: id,
            revision: 1,
            bundle: bundle.clone(),
            continuous: true,
            selection: None,
            preview_aspect: 1.,
        });
        let reference = reference
            .results
            .recv_timeout(std::time::Duration::from_secs(30))
            .unwrap()
            .frame
            .unwrap();
        let mut images = ImageRegistry::new(gpu.clone());
        let mut cache = Cache::default();
        let before = gpu.transfers();
        let output = complete(&mut cache, &mut images, &bundle, &keys);
        assert_eq!(output.handles.len(), keys.len());
        assert_eq!(gpu.transfers().readback_bytes, before.readback_bytes);
        assert_eq!(gpu.transfers().uploaded_bytes, before.uploaded_bytes);
        assert_eq!(
            gpu.transfers().status_readback_bytes - before.status_readback_bytes,
            4 * keys.len() as u64
        );
        let mut retained = Vec::new();
        for (key, pixels) in reference.thumbnails {
            let image = images.get(output.handles[&key]).unwrap();
            let mut read = ReadbackTicket::capture(&gpu, image.clone()).unwrap();
            wait(&gpu);
            let actual = read.try_finish(&gpu).unwrap().unwrap();
            assert_eq!(actual.pixels().len(), pixels.len());
            if let Some((at, (a, b))) = actual
                .pixels()
                .chunks_exact(4)
                .zip(pixels.chunks_exact(4))
                .enumerate()
                .find(|(_, (a, b))| a != b)
            {
                panic!("{key}: pixel {at}: native={a:?}, reference={b:?}");
            }
            retained.push((image, actual.pixels().to_vec()));
        }
        assert!(
            retained
                .iter()
                .any(|(_, pixels)| pixels.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255)),
            "translucent source coverage is preserved"
        );
        let evaluations = cache.producer.as_ref().unwrap().evaluations;
        let before = gpu.transfers();
        let unchanged = complete(&mut cache, &mut images, &bundle, &keys);
        assert_eq!(unchanged.handles, output.handles);
        assert_eq!(
            gpu.transfers(),
            before,
            "unchanged graph does no GPU work/transfers"
        );
        // Root display-domain changes leave unit-domain graph sources unchanged.
        let mut shifted = asset.clone();
        let mut domain = denrim_materials::integration::InstanceRecord::new(
            super::super::composition::SURFACE_DOMAIN,
        );
        domain.uv_transform[4] = 1.8;
        domain.uv_transform[5] = -0.123;
        shifted.instances.push(domain);
        let shifted = shifted.save().unwrap();
        let metadata = complete(&mut cache, &mut images, &shifted, &keys);
        assert_eq!(metadata.handles, output.handles);
        assert_eq!(gpu.transfers(), before);
        assert_eq!(cache.producer.as_ref().unwrap().evaluations, evaluations);
        // Replace source pixels while old explicit consumers still retain their images.
        let changed_source = source.replace("0.8 0.1 0.4 0.3", "0.1 0.9 0.2 0.6");
        let changed_asset =
            EditableMaterialAsset::parse("thumbnail-review", &changed_source, "surface").unwrap();
        let changed_bundle = changed_asset.save().unwrap();
        let before = gpu.transfers();
        let changed = complete(&mut cache, &mut images, &changed_bundle, &keys);
        assert_ne!(changed.handles["alpha"], metadata.handles["alpha"]);
        assert_eq!(gpu.transfers().readback_bytes, before.readback_bytes);
        assert_eq!(gpu.transfers().uploaded_bytes, before.uploaded_bytes);
        let changed_image = images.get(changed.handles["alpha"]).unwrap();
        let mut read = ReadbackTicket::capture(&gpu, changed_image).unwrap();
        wait(&gpu);
        let changed_pixels = read.try_finish(&gpu).unwrap().unwrap();
        assert!(changed_pixels.pixels().chunks_exact(4).all(|p| p[3] == 153));
        let one = complete(&mut cache, &mut images, &shifted, &keys[..1]);
        assert_eq!(one.handles.len(), 1);
        for key in &keys[1..] {
            assert!(images.get(changed.handles[key]).is_none());
        }
        let reopened = complete(&mut cache, &mut images, &shifted, &keys);
        assert_eq!(reopened.handles.len(), keys.len());
        // Start a new evaluation, close its graph, then process its old callback.
        cache.invalidate();
        assert!(
            cache
                .prepare(&mut images, Some(&shifted), &keys)
                .unwrap()
                .waiting
        );
        let hidden = cache.prepare(&mut images, Some(&shifted), &[]).unwrap();
        assert!(hidden.handles.is_empty() && !hidden.waiting);
        wait(&gpu);
        let recovered = complete(&mut cache, &mut images, &shifted, &keys);
        assert_eq!(recovered.handles.len(), keys.len());
        // Switching the source owner revokes all old graph handles immediately.
        let owner_asset =
            EditableMaterialAsset::parse("other-material-owner", &source, "surface").unwrap();
        let owner_bundle = owner_asset.save().unwrap();
        let switched = cache
            .prepare(&mut images, Some(&owner_bundle), &keys)
            .unwrap();
        for old in recovered.handles.values() {
            assert!(images.get(*old).is_none());
            assert!(!switched.handles.values().any(|handle| handle == old));
        }
        let recovered = complete(&mut cache, &mut images, &owner_bundle, &keys);
        let foreign_handle = recovered.handles[&keys[0]];
        let replacement_gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let mut replacement = ImageRegistry::new(replacement_gpu.clone());
        let rebuilt = complete(&mut cache, &mut replacement, &owner_bundle, &keys);
        assert_eq!(rebuilt.handles.len(), keys.len());
        assert!(images.get(foreign_handle).is_some());
        assert_eq!(replacement_gpu.transfers().readback_bytes, 0);
        assert_eq!(replacement_gpu.transfers().uploaded_bytes, 0);
        assert!(
            cache
                .prepare(&mut replacement, Some("broken bundle"), &keys)
                .is_err()
        );
        let valid = complete(&mut cache, &mut replacement, &owner_bundle, &keys);
        assert_eq!(valid.handles, rebuilt.handles);
        cache.clear();
        assert!(
            cache
                .prepare(&mut replacement, None, &[])
                .unwrap()
                .handles
                .is_empty()
        );
        for handle in valid.handles.values() {
            assert!(replacement.get(*handle).is_none());
        }
        // Already retained CPU-capture consumers still hold the original immutable outputs.
        for (image, expected) in retained {
            let mut read = ReadbackTicket::capture(&gpu, image).unwrap();
            wait(&gpu);
            assert_eq!(read.try_finish(&gpu).unwrap().unwrap().pixels(), expected);
        }
        assert_eq!(asset.save().unwrap(), bundle);
    }
}
