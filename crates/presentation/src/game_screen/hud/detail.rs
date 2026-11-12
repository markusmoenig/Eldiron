//! Wrapped catalogue detail blocks use the same measured Parley/Swash runs.
use super::*;
struct Block {
    text: String,
    size: f32,
    width: f32,
    layout: Arc<TextLayout>,
    x: f32,
    y: f32,
    color: [u8; 4],
}
pub(super) struct PreparedDetail {
    blocks: Vec<Block>,
    family: String,
    version: GameFontVersion,
    pub max_scroll: f32,
    pub scroll: f32,
    pub bounds: Rect,
    pub background: [u8; 4],
}
pub(super) fn prepare(
    spec: &model::DetailSpec,
    style: &style::PanelStyle,
    old: Option<&PreparedDetail>,
    fonts: &mut GameFonts,
) -> PreparedDetail {
    let bounds = spec.rect.with_border(6.);
    let old = old.filter(|old| old.family == style.font && old.version == fonts.version());
    let mut blocks = Vec::new();
    let mut add = |text: &str, size: f32, width: f32, x: f32, y: f32, color: [u8; 4]| {
        let width = width.max(1.);
        let layout = old
            .and_then(|old| {
                old.blocks
                    .iter()
                    .find(|block| block.text == text && block.size == size && block.width == width)
            })
            .map(|block| block.layout.clone())
            .unwrap_or_else(|| Arc::new(fonts.text.layout(text, &style.font, size, Some(width))));
        let height = layout.height();
        blocks.push(Block {
            text: text.into(),
            size,
            width,
            layout,
            x,
            y,
            color,
        });
        height
    };
    let mut y = 0.;
    if !spec.description.title.is_empty() {
        y += add(
            &spec.description.title,
            spec.font_size,
            bounds.width,
            0.,
            y,
            style.text,
        ) + 4.;
    }
    if let Some(subtitle) = &spec.description.subtitle {
        y += add(subtitle, spec.small_size, bounds.width, 0., y, style.muted) + 4.;
    }
    for row in &spec.rows {
        use screen_model::CatalogDetailRowKind as Kind;
        if row.kind == Kind::Section {
            y += 6.;
        }
        let size = if matches!(row.kind, Kind::Summary | Kind::Fact | Kind::Effect) {
            spec.font_size
        } else {
            spec.small_size
        };
        let color = if row.kind == Kind::Warning {
            [230, 160, 120, 255]
        } else if row.kind == Kind::Section {
            style.muted
        } else {
            style.text
        };
        let height = if let Some(label) = &row.label {
            let label_width = (bounds.width * 0.35).min(90.);
            let label_height = add(label, spec.small_size, label_width, 0., y, style.muted);
            label_height.max(add(
                &row.text,
                size,
                (bounds.width - label_width - 6.).max(1.),
                label_width + 6.,
                y,
                color,
            ))
        } else {
            add(&row.text, size, bounds.width, 0., y, color)
        };
        y += height + 3.;
    }
    let max_scroll = (y - bounds.height).max(0.);
    PreparedDetail {
        blocks,
        family: style.font.clone(),
        version: fonts.version(),
        max_scroll,
        scroll: spec.scroll.clamp(0., max_scroll),
        bounds,
        background: spec.background,
    }
}
pub(super) fn paint(
    p: &mut ScreenPainter<'_>,
    spec: &model::DetailSpec,
    detail: &PreparedDetail,
    border: [u8; 4],
) {
    p.fill(spec.rect, detail.background);
    p.border(spec.rect, 1., border);
    p.list.ops.push(PaintOp::PushClip(rect(detail.bounds)));
    for block in &detail.blocks {
        let origin = [
            detail.bounds.x + block.x,
            detail.bounds.y + block.y - detail.scroll,
        ];
        if origin[1] + block.layout.height() <= detail.bounds.y
            || origin[1] >= detail.bounds.y + detail.bounds.height
        {
            continue;
        }
        for run in &block.layout.runs {
            p.list.ops.push(PaintOp::GlyphRun {
                run: run.clone(),
                origin,
                color: color(block.color),
            });
        }
    }
    p.list.ops.push(PaintOp::PopClip);
    if detail.max_scroll > 0. && detail.bounds.height > 0. {
        let height = (detail.bounds.height * detail.bounds.height
            / (detail.bounds.height + detail.max_scroll))
            .max(8.)
            .min(detail.bounds.height);
        p.fill(
            Rect::new(
                spec.rect.x + spec.rect.width - 3.,
                detail.bounds.y
                    + (detail.bounds.height - height) * detail.scroll / detail.max_scroll,
                2.,
                height,
            ),
            border,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moving_and_scrolling_details_retain_wrapped_layouts_but_font_or_text_edits_rebuild() {
        let config = screen_model::CatalogPanelConfig::default();
        let style = style::PanelStyle::catalog(&config);
        let mut fonts = GameFonts::default();
        let mut spec = model::DetailSpec {
            rect: Rect::new(0., 0., 100., 80.),
            description: rusterix::client::rules_ui::RulesDescription {
                title: "A long wrapped catalogue title".into(),
                subtitle: None,
                lines: vec![],
            },
            rows: vec![screen_model::CatalogDetailRow {
                label: None,
                text: "A detailed action explanation that needs several wrapped lines.".into(),
                kind: screen_model::CatalogDetailRowKind::Summary,
            }],
            scroll: 0.,
            font_size: 14.,
            small_size: 10.,
            background: [0; 4],
        };
        let first = prepare(&spec, &style, None, &mut fonts);
        assert!(first.max_scroll > 0.);
        spec.rect.x = 50.;
        spec.rect.y = 60.;
        spec.scroll = 12.;
        let moved = prepare(&spec, &style, Some(&first), &mut fonts);
        assert!(Arc::ptr_eq(
            &first.blocks[0].layout,
            &moved.blocks[0].layout
        ));
        assert!(Arc::ptr_eq(
            &first.blocks[1].layout,
            &moved.blocks[1].layout
        ));
        spec.rows[0].text.push_str(" Changed.");
        let edited = prepare(&spec, &style, Some(&moved), &mut fonts);
        assert!(Arc::ptr_eq(
            &moved.blocks[0].layout,
            &edited.blocks[0].layout
        ));
        assert!(!Arc::ptr_eq(
            &moved.blocks[1].layout,
            &edited.blocks[1].layout
        ));
        fonts.text_mut();
        let fonts_changed = prepare(&spec, &style, Some(&edited), &mut fonts);
        assert!(!Arc::ptr_eq(
            &edited.blocks[0].layout,
            &fonts_changed.blocks[0].layout
        ));
    }
}
