//! Authored text game view preparation and retained glyph output. The room
//! description policy stays in the engine; no bitmap or scene target is made.
use super::*;
use theframework::text::{TextLayout, TextSystem};
use unicode_segmentation::UnicodeSegmentation;
#[derive(PartialEq)]
struct Source {
    lines: Vec<String>,
    family: String,
    size: f32,
    width: f32,
    line_height: f32,
    scale: [f32; 2],
}
struct Prepared {
    source: Source,
    layouts: Option<(GameFontVersion, Vec<Option<TextLayout>>)>,
}
#[derive(Default)]
pub(super) struct TextGames {
    entries: FxHashMap<Uuid, Prepared>,
    builds: u64,
}
impl TextGames {
    pub fn layout_builds(&self) -> u64 {
        self.builds
    }
    pub fn retain(&mut self, model: &ScreenElements) {
        self.entries.retain(|id, _| {
            model.game_widgets.get(id).is_some_and(|w| {
                w.backend_name == "text"
                    && !screen_model::widget_name_is_hidden(&w.name, &model.widgets_to_hide)
            })
        });
    }
    pub fn prepare(
        &mut self,
        id: Uuid,
        widget: &rusterix::client::widget::game::GameWidget,
        lines: Vec<String>,
    ) -> Result<(), String> {
        let size = widget.text_font_size;
        if !size.is_finite()
            || size <= 0.
            || !widget.rect.width.is_finite()
            || !widget.rect.height.is_finite()
        {
            return Err("Invalid text game view metrics".into());
        }
        if !widget.upscale.is_finite() || widget.upscale <= 0. {
            return Err("Invalid text game view upscale factor".into());
        }
        let dim = widget.render_surface_dim();
        let scale = [
            widget.rect.width / dim.width.max(1) as f32,
            widget.rect.height / dim.height.max(1) as f32,
        ];
        let line_height = size.ceil() + 6.;
        let count = ((dim.height as f32 - 12.).max(0.) / line_height)
            .ceil()
            .max(1.) as usize;
        let source = Source {
            lines: lines.into_iter().take(count).collect(),
            family: if widget.text_font_name.trim().is_empty() {
                "Roboto".into()
            } else {
                widget.text_font_name.clone()
            },
            size,
            width: (dim.width as f32 - 24.).max(0.),
            scale,
            line_height,
        };
        if self
            .entries
            .get(&id)
            .is_none_or(|entry| entry.source != source)
        {
            self.entries.insert(
                id,
                Prepared {
                    source,
                    layouts: None,
                },
            );
        }
        Ok(())
    }
    pub fn paint(
        &mut self,
        id: Uuid,
        widget: &rusterix::client::widget::game::GameWidget,
        version: GameFontVersion,
        p: &mut ScreenPainter<'_>,
    ) -> Result<(), String> {
        let prepared = self
            .entries
            .get_mut(&id)
            .ok_or("Text game view has not been prepared")?;
        if prepared
            .layouts
            .as_ref()
            .is_none_or(|(fonts, _)| *fonts != version)
        {
            let source = &prepared.source;
            let rows = source
                .lines
                .iter()
                .map(|line| {
                    fit(
                        p.text,
                        line,
                        &source.family,
                        source.size,
                        source.width,
                        &mut self.builds,
                    )
                })
                .collect();
            prepared.layouts = Some((version, rows));
        }
        p.list.ops.push(PaintOp::PushClip(rect(widget.rect)));
        p.fill(widget.rect, [0, 0, 0, 255]);
        let source = &prepared.source;
        p.list.ops.push(PaintOp::PushTransform(
            Transform::scale(source.scale[0], source.scale[1])
                .then(Transform::translate(widget.rect.x, widget.rect.y)),
        ));
        for (index, layout) in prepared.layouts.as_ref().unwrap().1.iter().enumerate() {
            let Some(layout) = layout else {
                continue;
            };
            let y = 12. + index as f32 * source.line_height;
            p.list.ops.push(PaintOp::PushClip(Uv::new(
                12.,
                y,
                source.width,
                source.line_height,
            )));
            for run in &layout.runs {
                p.list.ops.push(PaintOp::GlyphRun {
                    run: run.clone(),
                    origin: [12., y],
                    color: color(widget.text_color),
                });
            }
            p.list.ops.push(PaintOp::PopClip);
        }
        p.list.ops.push(PaintOp::PopTransform);
        p.list.ops.push(PaintOp::PopClip);
        Ok(())
    }
}
/// Keep the reference single-row/trailing-ellipsis policy, using shaped
/// measurements and whole graphemes. Narrow views clip the ellipsis safely.
fn fit(
    text: &mut TextSystem,
    line: &str,
    family: &str,
    size: f32,
    width: f32,
    builds: &mut u64,
) -> Option<TextLayout> {
    let line = line.trim_end();
    if line.is_empty() || width <= 0. {
        return None;
    }
    let mut shape = |content: &str| {
        *builds += 1;
        text.layout(content, family, size, None)
    };
    let original = shape(line);
    if original.width() < width {
        return Some(original);
    }
    let dots = shape("...");
    if dots.width() >= width {
        return Some(dots);
    }
    let cuts = line
        .grapheme_indices(true)
        .map(|(i, _)| i)
        .chain(std::iter::once(line.len()))
        .collect::<Vec<_>>();
    let (mut lo, mut hi) = (0, cuts.len() - 1);
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let candidate = shape(&format!("{}...", &line[..cuts[mid]]));
        if candidate.width() < width {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    Some(shape(&format!("{}...", &line[..cuts[lo]])))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_text_game_ellipsis_keeps_whole_graphemes_and_narrow_rows_bounded() {
        let mut fonts = GameFonts::default();
        let mut builds = 0;
        let source = "A e\u{301} 👩‍👩‍👧‍👦 東京 العربية lantern room with a long description";
        for width in [0., 1., 8., 30., 70., 120., 1000.] {
            let result = fit(&mut fonts.text, source, "Roboto", 18., width, &mut builds);
            if width == 0. {
                assert!(result.is_none());
                continue;
            }
            let layout = result.unwrap();
            let shown = layout.source.as_ref();
            if shown != source {
                assert!(shown.ends_with("..."));
                let prefix = &shown[..shown.len() - 3];
                assert!(
                    source
                        .grapheme_indices(true)
                        .map(|(i, _)| i)
                        .chain(std::iter::once(source.len()))
                        .any(|i| &source[..i] == prefix)
                );
            }
            let dots = fonts.text.layout("...", "Roboto", 18., None);
            assert!(layout.width() < width || dots.width() >= width);
        }
        assert!(builds > 0);
        assert!(fit(&mut fonts.text, "  ", "Roboto", 18., 100., &mut builds).is_none());
    }
}
