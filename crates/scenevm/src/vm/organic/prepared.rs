//! Checked CPU publication packets. Producers validate one source/instance per
//! cooperative item; installation moves complete immutable data without scanning.
use super::*;

#[derive(Default)]
pub struct PaintBillboardSourceBuilder {
    sprites: Vec<OrganicBillboardSprite>,
}
impl PaintBillboardSourceBuilder {
    pub fn push(&mut self, sprite: OrganicBillboardSprite) -> Result<u32, String> {
        if sprite.width == 0
            || sprite.height == 0
            || sprite.width > 256
            || sprite.height > 256
            || sprite.rgba.len() != sprite.width as usize * sprite.height as usize * 4
        {
            return Err("Invalid paint billboard source".into());
        }
        let slot =
            u32::try_from(self.sprites.len()).map_err(|_| "Too many paint billboard sources")?;
        self.sprites.push(sprite);
        Ok(slot)
    }
    pub fn finish(self) -> PaintBillboardSources {
        // Arc<Vec> wraps the completed allocation without copying its elements.
        PaintBillboardSources {
            sprites: Arc::new(self.sprites),
        }
    }
}
#[derive(Clone)]
pub struct PaintBillboardSources {
    sprites: Arc<Vec<OrganicBillboardSprite>>,
}
impl PaintBillboardSources {
    pub fn sprites(&self) -> &[OrganicBillboardSprite] {
        &self.sprites
    }
    pub fn prepare_instances(&self) -> PaintBillboardInstanceBuilder {
        PaintBillboardInstanceBuilder {
            sources: self.clone(),
            instances: Vec::new(),
        }
    }
}
pub struct PaintBillboardInstanceBuilder {
    sources: PaintBillboardSources,
    instances: Vec<OrganicBillboardInstance>,
}
impl PaintBillboardInstanceBuilder {
    pub fn push(&mut self, instance: OrganicBillboardInstance) -> Result<(), String> {
        if instance.sprite_index as usize >= self.sources.sprites.len()
            || !instance.width.is_finite()
            || instance.width <= 0.
            || !instance.height.is_finite()
            || instance.height <= 0.
            || !instance.center.iter().all(|value| value.is_finite())
        {
            return Err("Invalid paint billboard instance".into());
        }
        self.instances.push(instance);
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.instances.len()
    }
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }
    pub fn finish(self) -> PreparedPaintBillboards {
        PreparedPaintBillboards {
            sources: self.sources,
            instances: self.instances,
        }
    }
}
pub struct PreparedPaintBillboards {
    sources: PaintBillboardSources,
    instances: Vec<OrganicBillboardInstance>,
}
impl VM {
    pub fn install_prepared_paint_billboards(&mut self, packet: PreparedPaintBillboards) {
        if packet.instances.is_empty() {
            self.execute(Atom::ClearPaintBillboards);
            return;
        }
        let same_sources = self.paint_billboards.paged_sprites.is_empty()
            && Arc::ptr_eq(&self.paint_billboards.sprites, &packet.sources.sprites);
        self.paint_billboards = OrganicBillboardData {
            sprites: packet.sources.sprites,
            paged_sprites: Arc::default(),
            instances: packet.instances,
        };
        if same_sources {
            self.organic_instances_dirty = true;
        } else {
            self.invalidate_organic_source();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prepared_billboards_check_incrementally_and_match_reference_packets() {
        let pixels: Arc<[u8]> = vec![230, 40, 20, 255].into();
        let sprite = OrganicBillboardSprite {
            width: 1,
            height: 1,
            rgba: pixels.clone(),
        };
        let mut builder = PaintBillboardSourceBuilder::default();
        assert!(
            builder
                .push(OrganicBillboardSprite {
                    width: 0,
                    ..sprite.clone()
                })
                .is_err()
        );
        assert!(
            builder
                .push(OrganicBillboardSprite {
                    height: 257,
                    ..sprite.clone()
                })
                .is_err()
        );
        assert_eq!(builder.push(sprite.clone()).unwrap(), 0);
        let sources = builder.finish();
        let instance = OrganicBillboardInstance {
            center: [0.; 3],
            width: 1.,
            height: 2.,
            sprite_index: 0,
            flags: 1,
        };
        let mut instances = sources.prepare_instances();
        for invalid in [
            OrganicBillboardInstance {
                sprite_index: 1,
                ..instance
            },
            OrganicBillboardInstance {
                width: f32::NAN,
                ..instance
            },
            OrganicBillboardInstance {
                center: [f32::INFINITY; 3],
                ..instance
            },
        ] {
            assert!(instances.push(invalid).is_err());
        }
        assert!(instances.is_empty());
        instances.push(instance).unwrap();
        let mut vm = VM::new(16, 16);
        vm.install_prepared_paint_billboards(instances.finish());
        assert!(Arc::ptr_eq(&vm.paint_billboards.sprites, &sources.sprites));
        assert!(Arc::ptr_eq(&vm.paint_billboards.sprites[0].rgba, &pixels));
        let first = vm.organic_words(1024 * 1024).unwrap();
        let mut reference = VM::new(16, 16);
        reference.execute(Atom::SetPaintBillboards {
            sprites: vec![sprite],
            instances: vec![instance],
        });
        let expected = reference.organic_words(1024 * 1024).unwrap();
        assert_eq!(first.sources, expected.sources);
        assert_eq!(first.instances, expected.instances);
        let mut moved = sources.prepare_instances();
        moved
            .push(OrganicBillboardInstance {
                center: [2., 0., 0.],
                ..instance
            })
            .unwrap();
        vm.install_prepared_paint_billboards(moved.finish());
        assert!(!vm.organic_sources_dirty);
        assert!(vm.organic_instances_dirty);
        let next = vm.organic_words(1024 * 1024).unwrap();
        assert!(Arc::ptr_eq(&first.sources, &next.sources));
        assert_ne!(first.instances, next.instances);
        let weak = Arc::downgrade(&sources.sprites);
        drop(sources);
        assert!(weak.upgrade().is_some());
        let empty = PaintBillboardSourceBuilder::default()
            .finish()
            .prepare_instances()
            .finish();
        vm.install_prepared_paint_billboards(empty);
        assert!(weak.upgrade().is_none());
        assert!(vm.paint_billboards.sprites.is_empty());
        assert!(vm.paint_billboards.instances.is_empty());
    }
}
