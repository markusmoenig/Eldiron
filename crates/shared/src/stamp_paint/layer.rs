//! Cooperative adaptation of an immutable published recipe source. Dropping the
//! owner retires an incomplete layer; no partial authoring representation escapes.
use super::*;

pub struct StampLayerPreparation {
    source: Arc<StampPaint>,
    cursor: usize,
    chunk: IsoPaintChunk,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_layer_matches_reference_and_pins_original_source() {
        let mut layer = IsoPaintLayer::default();
        layer.active_brush = "grass".into();
        layer.add_stamp(crate::iso_paint::IsoPaintPoint::new(
            [8, 8],
            Some(vek::Vec3::zero()),
            None,
        ));
        let mut assets = rusterix::Assets::default();
        let map = Uuid::new_v4();
        assert!(sync(&mut assets, map, &mut layer));
        let mut source = assets.stamp_paint.remove(&map).unwrap();
        let stamp = source.stamps[0].clone();
        Arc::make_mut(&mut source).stamps = vec![stamp; 8193];
        let expected = presentation_layer(&source);
        let mut preparation = StampLayerPreparation::new(source.clone());
        assert!(!preparation.advance(0));
        assert!(!preparation.advance(usize::MAX));
        assert!(StampLayerPreparation::new(source.clone()).finish().is_err());
        Arc::make_mut(&mut source).stamps.clear();
        while !preparation.advance(4096) {}
        let actual = preparation.finish().unwrap();
        assert_eq!(
            serde_json::to_value(&actual.chunks).unwrap(),
            serde_json::to_value(&expected.chunks).unwrap()
        );
    }
}
impl StampLayerPreparation {
    pub fn new(source: Arc<StampPaint>) -> Self {
        Self {
            source,
            cursor: 0,
            chunk: IsoPaintChunk::new([0; 2]),
        }
    }
    pub fn advance(&mut self, items: usize) -> bool {
        let start = web_time::Instant::now();
        for _ in 0..items.min(4096) {
            if start.elapsed() >= std::time::Duration::from_millis(2) {
                break;
            }
            let Some(stamp) = self.source.stamps.get(self.cursor) else {
                return true;
            };
            self.chunk
                .stamps
                .push(presentation_stamp(stamp, self.cursor));
            self.cursor += 1;
        }
        self.cursor == self.source.stamps.len()
    }
    pub fn finish(self) -> Result<IsoPaintLayer, String> {
        if self.cursor != self.source.stamps.len() {
            return Err("Stamp layer preparation is incomplete".into());
        }
        let mut layer = IsoPaintLayer::default();
        layer.chunks.insert("runtime-stamps".into(), self.chunk);
        Ok(layer)
    }
}
