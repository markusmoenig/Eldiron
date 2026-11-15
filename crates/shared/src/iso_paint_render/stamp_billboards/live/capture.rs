//! Freeze original lists cheaply, then visit recipes under an item/time quota.
//! Incomplete capture never exposes a request that could match/publish pixels.
use super::*;

pub struct StampRequestCapture {
    request: StampSpriteRequest,
    chunk: usize,
    stamp: usize,
    buckets: HashMap<u64, Vec<usize>>,
    previous: Option<StampSpriteRequest>,
    matches_previous: bool,
}
impl StampSpriteRequest {
    pub fn begin_capture(layer: &IsoPaintLayer) -> StampRequestCapture {
        Self::begin_capture_with_previous(layer, None)
    }
    pub fn begin_capture_with_previous(
        layer: &IsoPaintLayer,
        previous: Option<&Self>,
    ) -> StampRequestCapture {
        StampRequestCapture {
            request: Self {
                visible: layer.visible,
                sources: Arc::new(if layer.visible {
                    layer
                        .chunks
                        .values()
                        .filter(|chunk| !chunk.stamps.is_empty())
                        .map(|chunk| chunk.stamps.clone())
                        .collect()
                } else {
                    Vec::new()
                }),
                ..Default::default()
            },
            chunk: 0,
            stamp: 0,
            buckets: HashMap::new(),
            previous: previous.cloned(),
            matches_previous: previous.is_some(),
        }
    }
}
impl StampRequestCapture {
    pub fn same_source(&self, layer: &IsoPaintLayer) -> bool {
        self.request.same_source(layer)
    }
    pub fn advance(&mut self, items: usize) -> bool {
        let start = web_time::Instant::now();
        for _ in 0..items.min(4096) {
            if start.elapsed() >= std::time::Duration::from_millis(2) {
                break;
            }
            let Some(source) = self.request.sources.get(self.chunk) else {
                return true;
            };
            let stamp = &source[self.stamp];
            if stamp.world.is_some() || stamp.paint_geo.is_some() && stamp.surface_uv.is_some() {
                let bucket = self.buckets.entry(visual_hash(stamp)).or_default();
                if !bucket
                    .iter()
                    .any(|index| same_visual(&self.request.inputs[*index], stamp))
                {
                    if self.previous.as_ref().is_some_and(|previous| {
                        previous
                            .inputs
                            .get(self.request.inputs.len())
                            .is_none_or(|old| !same_visual(old, stamp))
                    }) {
                        self.matches_previous = false;
                    }
                    bucket.push(self.request.inputs.len());
                    Arc::get_mut(&mut self.request.inputs)
                        .unwrap()
                        .push(Arc::new(stamp.clone()));
                }
            }
            self.stamp += 1;
            if self.stamp == source.len() {
                self.chunk += 1;
                self.stamp = 0;
            }
        }
        self.chunk == self.request.sources.len()
    }
    pub fn finish(self) -> Result<StampSpriteRequest, String> {
        self.finish_with_previous().map(|(request, _)| request)
    }
    pub fn finish_with_previous(mut self) -> Result<(StampSpriteRequest, bool), String> {
        if self.chunk != self.request.sources.len() {
            return Err("Stamp request capture is incomplete".into());
        }
        self.matches_previous &= self
            .previous
            .as_ref()
            .is_some_and(|previous| previous.inputs.len() == self.request.inputs.len());
        if self.matches_previous {
            self.request.visual_identity = self.previous.as_ref().unwrap().visual_identity.clone();
        }
        Ok((self.request, self.matches_previous))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_capture_freezes_recipes_and_proves_pose_only_reuse() {
        let mut layer = IsoPaintLayer::default();
        layer.active_brush = "grass".into();
        layer.active_size = 1.;
        layer.add_stamp(crate::iso_paint::IsoPaintPoint::new(
            [8, 8],
            Some(Vec3::zero()),
            None,
        ));
        let previous = StampSpriteRequest::capture(&layer);
        let chunk = layer.chunks.values_mut().next().unwrap();
        let mut stamp = chunk.stamps[0].clone();
        stamp.world = Some([1., 2., 3.]);
        chunk.stamps = vec![stamp; 8193].into();
        let mut capture = StampSpriteRequest::begin_capture_with_previous(&layer, Some(&previous));
        assert!(!capture.advance(0));
        assert!(!capture.advance(usize::MAX));
        assert!(!StampSpriteRequest::begin_capture(&layer).finish().is_ok());
        layer.chunks.values_mut().next().unwrap().stamps[0].kind = "changed".into();
        assert!(!capture.same_source(&layer));
        while !capture.advance(4096) {}
        let (request, same) = capture.finish_with_previous().unwrap();
        assert!(same);
        assert!(Arc::ptr_eq(
            &previous.visual_identity,
            &request.visual_identity
        ));
        assert_eq!(request.inputs.len(), 1);
        assert_eq!(request.inputs[0].world, Some([1., 2., 3.]));
        let mut changed = StampSpriteRequest::begin_capture_with_previous(&layer, Some(&request));
        while !changed.advance(4096) {}
        let (changed, same) = changed.finish_with_previous().unwrap();
        assert!(!same);
        assert!(!request.same(&changed));
    }
    #[test]
    fn finished_artwork_releases_original_placement_catalogues() {
        let mut layer = IsoPaintLayer::default();
        layer.active_brush = "grass".into();
        layer.active_size = 1.;
        layer.add_stamp(crate::iso_paint::IsoPaintPoint::new(
            [8, 8],
            Some(Vec3::zero()),
            None,
        ));
        let request = StampSpriteRequest::capture(&layer);
        let original_lists = Arc::downgrade(&request.sources);
        let mut prep = request.prepare(None);
        for _ in 0..10000 {
            if prep.advance(1).unwrap() {
                let ready = prep.finish().unwrap();
                assert!(original_lists.upgrade().is_none());
                assert!(!ready.sprites().is_empty());
                assert!(ready.matches(&StampSpriteRequest::capture(&layer)));
                return;
            }
        }
        panic!("stamp preparation starved");
    }
}
