use super::*;
use crate::editor_overlay::procedural::ColorNoise;
use crate::prelude::*;
fn capture(gpu: &GpuContext, image: Arc<GpuImage>) -> Vec<u8> {
    let mut ticket = renderkit::ReadbackTicket::capture(gpu, image).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    ticket.try_finish(gpu).unwrap().unwrap().pixels().to_vec()
}
#[test]
#[ignore = "actual native GPU procedural source/reference comparison"]
fn procedural_icon_gpu_sources_match_legacy_color_and_noise_without_pixel_uploads() {
    let gpu = pollster::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut icons = GpuIcons::new(gpu.clone(), 1024 * 1024, 128).unwrap();
    let mut cases = vec![];
    for size in [2, 40, 63] {
        for target in [0, 1, 2] {
            for intensity in [0., 0.2, 0.7, 1.] {
                for pixelization in [1, 7, 80] {
                    let mut values = rusterix::ValueContainer::default();
                    values.set("noise_target", Value::Int(target));
                    values.set("noise_intensity", Value::Float(intensity));
                    values.set("pixelization", Value::Int(pixelization));
                    let source = rusterix::PixelSource::Color(TheColor::new(0.19, 0.57, 0.83, 0.4));
                    cases.push(
                        ProceduralIcon::new(&source, ColorNoise::from_values(&values), size)
                            .unwrap(),
                    );
                }
            }
        }
    }
    for size in [2, 40, 63] {
        for voronoi in [false, true] {
            for scale in [0, 8, 99] {
                for seed in [0, 917, u32::MAX] {
                    let source =
                        rusterix::PixelSource::Noise(rusterix::map::pixelsource::NoiseMaterial {
                            voronoi,
                            scale,
                            seed,
                            low_color: Some([18, 67, 160, 40]),
                            high_color: Some([234, 169, 45, 230]),
                            ..Default::default()
                        });
                    cases.push(ProceduralIcon::new(&source, ColorNoise::default(), size).unwrap());
                }
            }
        }
    }
    for target in [-1, 3] {
        for pixelization in [0, -3, 99999] {
            let mut values = rusterix::ValueContainer::default();
            values.set("noise_target", Value::Int(target));
            values.set("noise_intensity", Value::Float(-0.3));
            values.set("pixelization", Value::Int(pixelization));
            cases.push(
                ProceduralIcon::new(
                    &rusterix::PixelSource::Color(TheColor::new(0.19, 0.57, 0.83, 0.4)),
                    ColorNoise::from_values(&values),
                    40,
                )
                .unwrap(),
            );
        }
    }
    for color in [
        TheColor::new(0., 0., 0., 0.),
        TheColor::new(1., 1., 1., 1.),
        TheColor::new(0.5, 0.5, 0.5, 0.2),
    ] {
        for target in [1, 2] {
            let mut values = rusterix::ValueContainer::default();
            values.set("noise_target", Value::Int(target));
            values.set("noise_intensity", Value::Float(0.7));
            cases.push(
                ProceduralIcon::new(
                    &rusterix::PixelSource::Color(color.clone()),
                    ColorNoise::from_values(&values),
                    40,
                )
                .unwrap(),
            );
        }
    }
    for size in [0, 257] {
        for voronoi in [false, true] {
            let source = rusterix::PixelSource::Noise(rusterix::map::pixelsource::NoiseMaterial {
                voronoi,
                low: 230,
                high: 40,
                ..Default::default()
            });
            cases.push(ProceduralIcon::new(&source, ColorNoise::default(), size).unwrap());
        }
    }
    let mut max_error = 0;
    for (index, source) in cases.iter().enumerate() {
        let before = gpu.transfers();
        let builds = icons.stats().builds;
        let handle = icons.prepare(&mut registry, source).unwrap();
        let built = icons.stats().builds != builds;
        assert_eq!(
            gpu.transfers().uploaded_bytes - before.uploaded_bytes,
            if built { 96 } else { 0 }
        );
        assert_eq!(gpu.transfers().readback_bytes, before.readback_bytes);
        let pixels = capture(&gpu, registry.get(handle).unwrap());
        let expected = source.reference_pixels();
        assert_eq!(pixels.len(), expected.data.len());
        let error = pixels
            .iter()
            .zip(&expected.data)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        max_error = max_error.max(error);
        assert!(
            error <= 1,
            "case {index}, words {:?}, max channel error {error}",
            source.words()
        );
        let warm = gpu.transfers();
        assert_eq!(icons.prepare(&mut registry, source).unwrap(), handle);
        assert_eq!(gpu.transfers().uploaded_bytes, warm.uploaded_bytes);
        icons.finish_frame(&mut registry, &root(&[handle])).unwrap();
        assert!(!icons.stats().over_budget);
        assert!(
            icons.stats().resident_images <= 128 && icons.stats().resident_bytes <= 1024 * 1024
        );
    }
    let invalid = ProceduralIcon::new(
        &rusterix::PixelSource::Color(TheColor::new(0., 0., 0., 1.)),
        ColorNoise::default(),
        0,
    )
    .unwrap();
    let before = gpu.transfers();
    assert!(icons.prepare(&mut registry, &invalid).is_err());
    assert_eq!(gpu.transfers().uploaded_bytes, before.uploaded_bytes);
    println!(
        "{} procedural sources, max encoded channel difference {max_error}",
        cases.len()
    );
}

fn noise(seed: u32) -> ProceduralIcon {
    ProceduralIcon::new(
        &rusterix::PixelSource::Noise(rusterix::map::pixelsource::NoiseMaterial {
            seed,
            ..Default::default()
        }),
        ColorNoise::default(),
        40,
    )
    .unwrap()
}
fn root(handles: &[ImageHandle]) -> PaintList {
    let mut list = PaintList::default();
    for handle in handles {
        list.ops.push(theframework::paint::PaintOp::PixelImage {
            image: *handle,
            rect: theframework::paint::Rect::new(0., 0., 40., 40.),
            tint: [1.; 4],
        });
    }
    list
}
#[test]
#[ignore = "actual native GPU bounded source residency and replacement devices"]
fn procedural_icon_cache_pins_retained_roots_and_rejects_foreign_devices_without_readbacks() {
    let gpu = pollster::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut icons = GpuIcons::new(gpu.clone(), 6400, 1).unwrap();
    let a = icons.prepare(&mut registry, &noise(1)).unwrap();
    let b = icons.prepare(&mut registry, &noise(2)).unwrap();
    assert_eq!(gpu.transfers().uploaded_bytes, 192);
    assert_eq!(gpu.transfers().readback_bytes, 0);
    icons.finish_frame(&mut registry, &root(&[a, b])).unwrap();
    assert!(icons.stats().over_budget);
    assert_eq!(icons.stats().pinned_bytes, 12800);
    assert!(registry.get(a).is_some() && registry.get(b).is_some());
    icons.finish_frame(&mut registry, &root(&[a])).unwrap();
    assert!(registry.get(a).is_some() && registry.get(b).is_none());
    assert!(!icons.stats().over_budget);
    assert_eq!(icons.stats().resident_bytes, 6400);
    let warm = gpu.transfers();
    assert_eq!(icons.prepare(&mut registry, &noise(1)).unwrap(), a);
    assert_eq!(gpu.transfers().uploaded_bytes, warm.uploaded_bytes);
    let c = icons.prepare(&mut registry, &noise(3)).unwrap();
    icons.finish_frame(&mut registry, &root(&[a, c])).unwrap();
    assert!(icons.stats().over_budget);
    icons.finish_frame(&mut registry, &root(&[c])).unwrap();
    assert!(registry.get(a).is_none());
    assert!(registry.get(c).is_some());
    let retained = registry.get(c).unwrap();
    let before = capture(&gpu, retained.clone());
    assert!(registry.remove(c));
    let rebuilt = icons.prepare(&mut registry, &noise(3)).unwrap();
    assert_ne!(rebuilt, c);
    assert_eq!(capture(&gpu, registry.get(rebuilt).unwrap()), before);
    let mut replacement = ImageRegistry::new(gpu.clone());
    let foreign_image = GpuImage::new(&gpu, ImageDescriptor::rgba(1, 1)).unwrap();
    let foreign = replacement.insert(foreign_image.clone()).unwrap();
    let current = icons.prepare(&mut replacement, &noise(3)).unwrap();
    assert_ne!(current.owner, rebuilt.owner);
    assert_eq!(icons.stats().pinned_bytes, 0);
    assert!(Arc::ptr_eq(
        &replacement.get(foreign).unwrap(),
        &foreign_image
    ));
    assert_eq!(capture(&gpu, replacement.get(current).unwrap()), before);
    let second_gpu = pollster::block_on(GpuContext::headless()).unwrap();
    let mut second_registry = ImageRegistry::new(second_gpu.clone());
    let transfers = second_gpu.transfers();
    assert!(icons.prepare(&mut second_registry, &noise(3)).is_err());
    assert_eq!(
        second_gpu.transfers().uploaded_bytes,
        transfers.uploaded_bytes
    );
    let mut rebuilt_icons = GpuIcons::new(second_gpu.clone(), 6400, 1).unwrap();
    let recovered = rebuilt_icons
        .prepare(&mut second_registry, &noise(3))
        .unwrap();
    assert_eq!(
        capture(&second_gpu, second_registry.get(recovered).unwrap()),
        before
    );
    icons.clear(&mut replacement);
    assert!(replacement.get(current).is_none());
    assert!(Arc::ptr_eq(
        &replacement.get(foreign).unwrap(),
        &foreign_image
    ));
    assert_eq!(icons.stats().resident_images, 0);
    assert_eq!(
        capture(&gpu, retained),
        before,
        "explicit leases survive registry retirement"
    );
}
