//! GPU/PNG comparison utilities for independent editor-overlay fixtures.
use super::*;
#[cfg(feature = "native-render")]
use theframework::paint::{PaintList, PaintOp, Rect};

pub(crate) fn expected(path: &std::path::Path) -> Vec<u8> {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    pixels.truncate(info.buffer_size());
    pixels
}

#[cfg(feature = "native-render")]
pub(crate) struct GpuPanel {
    pub(crate) gpu: std::sync::Arc<renderkit::GpuContext>,
    pub(crate) images: theframework::render_wgpu::UiImages,
    pub(crate) renderer: theframework::render_wgpu::PaintRenderer,
    pub(crate) text: theframework::text::TextSystem,
}
#[cfg(feature = "native-render")]
impl GpuPanel {
    pub(crate) fn new() -> Self {
        let gpu = pollster::block_on(renderkit::GpuContext::headless()).unwrap();
        let images = theframework::render_wgpu::UiImages::new(gpu.clone());
        let renderer = theframework::render_wgpu::PaintRenderer::new(gpu.clone()).unwrap();
        let mut text = theframework::text::TextSystem::default();
        text.register_font(
            include_bytes!("../../../crates/theframework/embedded/fonts/Roboto-Bold.ttf").to_vec(),
        );
        text.register_font(
            include_bytes!("../../../crates/theframework/embedded/fonts/SourceCodePro-Bold.ttf")
                .to_vec(),
        );
        Self {
            gpu,
            images,
            renderer,
            text,
        }
    }
    pub(crate) fn render(
        &mut self,
        list: &PaintList,
        size: (usize, usize),
        density: f32,
    ) -> Vec<u8> {
        use renderkit::*;
        let physical = (
            (size.0 as f32 * density).round() as u32,
            (size.1 as f32 * density).round() as u32,
        );
        let output =
            GpuImage::new(&self.gpu, ImageDescriptor::rgba(physical.0, physical.1)).unwrap();
        self.images.finish_frame(list).unwrap();
        let before = self.gpu.transfers().readback_bytes;
        for pass in 0..2 {
            let uploads = self.gpu.transfers().uploaded_bytes;
            let glyphs = self.renderer.glyphs.rasterizations;
            self.renderer
                .render(
                    list,
                    &self.images.registry,
                    &output.view,
                    output.descriptor.format,
                    physical,
                    density,
                    [12. / 255., 24. / 255., 36. / 255., 1.],
                )
                .unwrap();
            if pass == 1 {
                assert_eq!(self.gpu.transfers().uploaded_bytes, uploads);
                assert_eq!(self.renderer.glyphs.rasterizations, glyphs);
            }
            assert_eq!(
                self.gpu.transfers().readback_bytes,
                before,
                "drawing never requests output pixels"
            );
        }
        let mut capture = ReadbackTicket::capture(&self.gpu, output).unwrap();
        self.gpu
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        capture
            .try_finish(&self.gpu)
            .unwrap()
            .unwrap()
            .pixels()
            .to_vec()
    }
    pub(crate) fn compare_reference(
        &mut self,
        commands: &EditorOverlay,
        list: &PaintList,
        ctx: &TheContext,
        pixels: &[u8],
        reference: &[u8],
        name: &str,
    ) {
        self.compare_reference_many(&[commands], list, ctx, pixels, reference, name);
    }
    pub(crate) fn compare_reference_many(
        &mut self,
        commands: &[&EditorOverlay],
        list: &PaintList,
        ctx: &TheContext,
        pixels: &[u8],
        reference: &[u8],
        name: &str,
    ) {
        let size = commands.first().expect("nonempty overlay parts").size;
        assert!(commands.iter().all(|part| part.size == size));
        let mut mask = vec![false; size.0 * size.1];
        let mut mark = |rect: Rect| {
            let x0 = rect.x.floor().max(0.).min(size.0 as f32) as usize;
            let x1 = (rect.x + rect.width).ceil().max(0.).min(size.0 as f32) as usize;
            let y0 = rect.y.floor().max(0.).min(size.1 as f32) as usize;
            let y1 = (rect.y + rect.height).ceil().max(0.).min(size.1 as f32) as usize;
            for y in y0..y1 {
                for x in x0..x1 {
                    mask[y * size.0 + x] = true;
                }
            }
        };
        for commands in commands {
            for rect in commands.reference_text_bounds(ctx) {
                mark(rect);
            }
        }
        let mut clips = vec![Rect::new(0., 0., size.0 as f32, size.1 as f32)];
        for op in &list.ops {
            match op {
                PaintOp::PushClip(rect) => clips.push(clips.last().unwrap().intersection(*rect)),
                PaintOp::PopClip => {
                    clips.pop();
                }
                PaintOp::GlyphRun { run, origin, .. }
                | PaintOp::OpaqueGlyphRun { run, origin, .. } => {
                    for glyph in run.glyphs.iter() {
                        let position =
                            [origin[0] + glyph.position[0], origin[1] + glyph.position[1]];
                        if let Some((_, bitmap)) =
                            self.renderer.glyphs.glyph(run, glyph.id, 1., position)
                        {
                            mark(
                                Rect::new(
                                    position[0].floor() + bitmap.left as f32,
                                    position[1].floor() - bitmap.top as f32,
                                    bitmap.width as f32,
                                    bitmap.height as f32,
                                )
                                .intersection(*clips.last().unwrap()),
                            );
                        }
                    }
                }
                _ => (),
            }
        }
        assert_eq!(pixels.len(), reference.len());
        let mut compared = 0;
        let mut exact = 0;
        for (i, (actual, expected)) in pixels
            .chunks_exact(4)
            .zip(reference.chunks_exact(4))
            .enumerate()
        {
            assert_eq!(actual[3], 255);
            if actual[..3] == expected[..3] {
                exact += 1;
            }
            if mask[i] {
                continue;
            }
            compared += 1;
            assert!(
                actual[..3]
                    .iter()
                    .zip(&expected[..3])
                    .all(|(a, b)| a.abs_diff(*b) <= 1),
                "{name} ({},{}): {actual:?} vs {expected:?}",
                i % size.0,
                i / size.0
            );
        }
        assert!(
            compared > size.0 * size.1 * 9 / 10,
            "glyph exclusions cannot conceal general panel regressions: {compared}/{}",
            size.0 * size.1
        );
        println!(
            "Panel {name}: {compared} non-text pixels within one RGB unit, {exact}/{} full exact RGB; text metrics/AA remain open",
            size.0 * size.1
        );
        let mut png = png::Encoder::new(
            std::fs::File::create(format!("/private/tmp/eldiron-native-panel-{name}.png")).unwrap(),
            size.0 as u32,
            size.1 as u32,
        );
        png.set_color(png::ColorType::Rgba);
        png.set_depth(png::BitDepth::Eight);
        png.write_header()
            .unwrap()
            .write_image_data(pixels)
            .unwrap();
    }
}
