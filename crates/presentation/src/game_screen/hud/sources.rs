//! Retained source assets are independent of catalogue viewport dimensions.
use super::*;
use rusterix::client::widget::ButtonVisualState;
#[derive(Default)]
pub(super) struct PreparedSkin {
    pub frame: Option<Arc<PreparedContent>>,
    pub frame_slice: usize,
    pub slot: Option<Arc<PreparedContent>>,
    pub slot_slice: usize,
}
pub(super) fn prepare(
    world: &RuntimeView<'_>,
    elements: &ScreenElements,
    frame: &PanelFrame,
    old: Option<&PreparedHud>,
    fonts: &mut GameFonts,
    sources: &mut ContentSources,
    revision: ContentRevision,
    animation: usize,
) -> Result<
    (
        FxHashMap<(usize, u8), Arc<PreparedContent>>,
        FxHashMap<usize, Arc<TextLayout>>,
        FxHashMap<usize, (String, Arc<TextLayout>)>,
        PreparedSkin,
    ),
    String,
> {
    let mut icons = FxHashMap::default();
    let mut names = FxHashMap::default();
    let mut cooldowns = FxHashMap::default();
    for card in &frame.cards {
        for state in [
            ButtonVisualState::Normal,
            ButtonVisualState::Hover,
            ButtonVisualState::Pressed,
            ButtonVisualState::Selected,
            ButtonVisualState::Disabled,
        ] {
            if let Some(source) = sources.prepare(
                ContentRecipe::CommandIcon {
                    command: &card.command,
                    state,
                    frame: animation,
                },
                world.assets,
                revision,
            )? {
                icons.insert((card.index, state as u8), source);
            }
        }
        let compatible = old.filter(|old| {
            old.title_key.3 == fonts.version()
                && old.frame.style.font == frame.style.font
                && old.frame.style.quantity_size == frame.style.quantity_size
        });
        let name = compatible
            .filter(|old| {
                old.frame
                    .cards
                    .iter()
                    .any(|previous| previous.index == card.index && previous.name == card.name)
            })
            .and_then(|old| old.card_names.get(&card.index))
            .cloned()
            .unwrap_or_else(|| {
                Arc::new(fonts.text.layout(
                    &card.name,
                    &frame.style.font,
                    frame.style.quantity_size,
                    None,
                ))
            });
        names.insert(card.index, name);
        if card.state.cooldown_remaining > 0. {
            let text = format!("{:.1}", card.state.cooldown_remaining);
            let layout = compatible
                .and_then(|old| old.cooldowns.get(&card.index))
                .filter(|(old, _)| *old == text)
                .map(|(_, layout)| layout.clone())
                .unwrap_or_else(|| {
                    Arc::new(fonts.text.layout(
                        &text,
                        &frame.style.font,
                        frame.style.quantity_size,
                        None,
                    ))
                });
            cooldowns.insert(card.index, (text, layout));
        }
    }
    let mut skin = PreparedSkin::default();
    if let Some(kind) = frame.catalog_skin {
        let c = model::catalog::configuration(elements, kind);
        skin.frame = c
            .frame_texture
            .as_ref()
            .map(|texture| {
                sources.prepare(ContentRecipe::Texture { texture }, world.assets, revision)
            })
            .transpose()?
            .flatten();
        skin.slot = c
            .slot_texture
            .as_ref()
            .map(|texture| {
                sources.prepare(ContentRecipe::Texture { texture }, world.assets, revision)
            })
            .transpose()?
            .flatten();
        skin.frame_slice = c.frame_slice;
        skin.slot_slice = c.slot_slice;
    }
    Ok((icons, names, cooldowns, skin))
}

/// Nine-slice source art is a set of GPU image blits, never a stretched bitmap.
pub(super) fn blit(
    p: &mut ScreenPainter<'_>,
    source: &PreparedContent,
    bounds: Rect,
    slice: usize,
) -> Result<(), String> {
    let image = source.resolve(p.images)?;
    let (width, height) = (
        source.image().width() as f32,
        source.image().height() as f32,
    );
    if bounds.width <= 0. || bounds.height <= 0. {
        return Ok(());
    }
    let sx = (slice as f32).min(width / 2.);
    let sy = (slice as f32).min(height / 2.);
    let dx = sx.min(bounds.width / 2.);
    let dy = sy.min(bounds.height / 2.);
    let xs = [0., sx, width - sx, width];
    let ys = [0., sy, height - sy, height];
    let xd = [
        bounds.x,
        bounds.x + dx,
        bounds.x + bounds.width - dx,
        bounds.x + bounds.width,
    ];
    let yd = [
        bounds.y,
        bounds.y + dy,
        bounds.y + bounds.height - dy,
        bounds.y + bounds.height,
    ];
    for row in 0..3 {
        for column in 0..3 {
            let (w, h) = (xd[column + 1] - xd[column], yd[row + 1] - yd[row]);
            if w <= 0. || h <= 0. || xs[column + 1] <= xs[column] || ys[row + 1] <= ys[row] {
                continue;
            }
            p.list.ops.push(PaintOp::Image {
                image,
                rect: Uv::new(xd[column], yd[row], w, h),
                uv: Uv::new(
                    xs[column] / width,
                    ys[row] / height,
                    (xs[column + 1] - xs[column]) / width,
                    (ys[row + 1] - ys[row]) / height,
                ),
                sampling: Sampling::Nearest,
                tint: [1.; 4],
            });
        }
    }
    Ok(())
}
