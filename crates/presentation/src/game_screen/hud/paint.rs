use super::*;
use theframework::paint::{Path, PathElement};
impl Hud {
    pub(in crate::game_screen) fn paint(
        &self,
        p: &mut ScreenPainter<'_>,
        world: &RuntimeView<'_>,
        revision: ContentRevision,
        animation: usize,
        cursor: [f32; 2],
    ) -> Result<(), String> {
        for prepared in &self.prepared {
            let frame = &prepared.frame;
            p.list.ops.push(PaintOp::PushClip(rect(frame.rect)));
            p.fill(frame.rect, frame.style.background);
            if let Some(source) = &prepared.skin.frame {
                sources::blit(p, source, frame.rect, prepared.skin.frame_slice)?;
            }
            for (tile, bounds) in &frame.tiles {
                tile_reference(
                    p,
                    world.session_id,
                    world.assets,
                    tile,
                    *bounds,
                    revision,
                    animation,
                )?;
            }
            p.border(frame.rect, frame.style.border_width, frame.style.border);
            if let Some(title) = frame.title_bar {
                p.fill(title, frame.style.title_background);
            }
            if let Some(title) = frame.title_rect {
                p.list.ops.push(PaintOp::PushClip(rect(title)));
                let origin = [
                    title.x,
                    title.y + (title.height - prepared.title.height()) / 2.,
                ];
                for run in &prepared.title.runs {
                    p.list.ops.push(PaintOp::GlyphRun {
                        run: run.clone(),
                        origin,
                        color: color(frame.style.text),
                    });
                }
                p.list.ops.push(PaintOp::PopClip);
            }
            if let Some(close) = frame.close {
                let hovered = rect(close).contains(cursor);
                p.fill(
                    close,
                    if hovered {
                        [70, 78, 88, 245]
                    } else {
                        [42, 47, 54, 230]
                    },
                );
                p.border(
                    close,
                    1.,
                    if hovered {
                        [174, 179, 183, 255]
                    } else {
                        [98, 105, 116, 255]
                    },
                );
                let mut path = Path::default();
                path.elements.extend([
                    PathElement::Move([close.x + 6., close.y + 5.]),
                    PathElement::Line([close.x + close.width - 6., close.y + close.height - 5.]),
                    PathElement::Move([close.x + close.width - 6., close.y + 5.]),
                    PathElement::Line([close.x + 6., close.y + close.height - 5.]),
                ]);
                p.list.ops.push(PaintOp::Stroke {
                    path: Arc::new(path),
                    stroke: theframework::paint::Stroke {
                        width: 1.5,
                        cap: theframework::paint::LineCap::Butt,
                        join: theframework::paint::LineJoin::Round,
                        brush: theframework::paint::Brush::Solid(color([220, 220, 210, 255])),
                    },
                });
            }
            p.list.ops.push(PaintOp::PushClip(rect(frame.clip)));
            if let Some((_, bounds)) = frame.avatar {
                if let Some(avatar) = &prepared.avatar {
                    paint_source(p, avatar, bounds.with_border(4.))?;
                }
            }
            for slot in &frame.slots {
                if !frame
                    .slot_tile
                    .as_deref()
                    .map(|tile| {
                        tile_reference(
                            p,
                            world.session_id,
                            world.assets,
                            tile,
                            slot.rect,
                            revision,
                            animation,
                        )
                    })
                    .transpose()?
                    .unwrap_or(false)
                {
                    p.fill(slot.rect, frame.style.slot);
                }
                p.border(
                    slot.rect,
                    1.,
                    if slot.selected {
                        frame.style.selected
                    } else if slot.source.is_some() {
                        frame.style.occupied
                    } else {
                        frame.style.slot_border
                    },
                );
                if let Some(source) = prepared.items.get(&slot.index) {
                    let size = source.image();
                    let mut bounds = slot.rect.with_border(4.);
                    let scale = (bounds.width / size.width() as f32)
                        .min(bounds.height / size.height() as f32);
                    let (width, height) =
                        (size.width() as f32 * scale, size.height() as f32 * scale);
                    bounds.x += (bounds.width - width) / 2.;
                    bounds.y += (bounds.height - height) / 2.;
                    bounds.width = width;
                    bounds.height = height;
                    p.list.ops.push(PaintOp::Image {
                        image: source.resolve(p.images)?,
                        rect: rect(bounds),
                        uv: Uv::new(0., 0., 1., 1.),
                        sampling: Sampling::Nearest,
                        tint: [1.; 4],
                    });
                }
                if let Some((_, quantity)) = prepared.quantities.get(&slot.index) {
                    p.list.ops.push(PaintOp::PushClip(rect(slot.rect)));
                    let origin = [
                        slot.rect.x + slot.rect.width - quantity.width() - 2.,
                        slot.rect.y + slot.rect.height - quantity.height() - 1.,
                    ];
                    p.fill(
                        Rect::new(
                            origin[0] - 1.,
                            origin[1],
                            quantity.width() + 2.,
                            quantity.height(),
                        ),
                        [0, 0, 0, 170],
                    );
                    for run in &quantity.runs {
                        p.list.ops.push(PaintOp::GlyphRun {
                            run: run.clone(),
                            origin,
                            color: [1.; 4],
                        });
                    }
                    p.list.ops.push(PaintOp::PopClip);
                }
            }
            card::paint(p, prepared, cursor, self.command_capture.as_ref())?;
            for (spec, layout) in prepared.labels.iter().filter(|(spec, _)| spec.content) {
                paint_label(p, spec, layout);
            }
            p.list.ops.push(PaintOp::PopClip);
            if let (Some(spec), Some(detail)) = (&frame.detail, &prepared.detail) {
                detail::paint(p, spec, detail, frame.style.border);
            }
            if frame.max_scroll[1] > 0. && frame.clip.height > 0. {
                let height = (frame.clip.height as f64 * frame.clip.height as f64
                    / (frame.clip.height as f64 + frame.max_scroll[1]))
                    .max(8.)
                    .min(frame.clip.height as f64) as f32;
                let y = frame.clip.y
                    + (frame.clip.height - height) * (frame.scroll[1] / frame.max_scroll[1]) as f32;
                p.fill(
                    Rect::new(frame.rect.x + frame.rect.width - 3., y, 2., height),
                    frame.style.border,
                );
            }
            if frame.max_scroll[0] > 0. && frame.clip.width > 0. {
                let width = (frame.clip.width as f64 * frame.clip.width as f64
                    / (frame.clip.width as f64 + frame.max_scroll[0]))
                    .max(8.)
                    .min(frame.clip.width as f64) as f32;
                let x = frame.clip.x
                    + (frame.clip.width - width) * (frame.scroll[0] / frame.max_scroll[0]) as f32;
                p.fill(
                    Rect::new(x, frame.rect.y + frame.rect.height - 3., width, 2.),
                    frame.style.border,
                );
            }
            for (spec, layout) in prepared
                .labels
                .iter()
                .take(frame.labels.len())
                .filter(|(spec, _)| !spec.content)
            {
                paint_label(p, spec, layout);
            }
            for (index, button) in frame.buttons.iter().enumerate() {
                p.fill(button.rect, button.background);
                p.border(
                    button.rect,
                    1.,
                    if button.selected {
                        frame.style.selected
                    } else {
                        frame.style.border
                    },
                );
                let (spec, layout) = &prepared.labels[frame.labels.len() + index];
                paint_label(p, spec, layout);
            }
            if let Some(id) = self.focused.filter(|id| id.panel == frame.id) {
                if !matches!(id.part, HudPart::Body | HudPart::Title) {
                    if let Some((_, bounds)) = frame
                        .controls()
                        .into_iter()
                        .find(|(control, _)| *control == id)
                    {
                        p.border(bounds.with_border(1.), 1., frame.style.selected);
                    }
                }
            }
            p.list.ops.push(PaintOp::PopClip);
        }

        Ok(())
    }
}
fn paint_label(p: &mut ScreenPainter<'_>, spec: &model::PanelLabel, layout: &TextLayout) {
    p.list.ops.push(PaintOp::PushClip(rect(spec.rect)));
    let origin = [
        spec.rect.x
            + if spec.centered {
                (spec.rect.width - layout.width()) / 2.
            } else {
                0.
            },
        spec.rect.y + (spec.rect.height - layout.height()) / 2.,
    ];
    for run in &layout.runs {
        p.list.ops.push(PaintOp::GlyphRun {
            run: run.clone(),
            origin,
            color: color(spec.color),
        });
    }
    p.list.ops.push(PaintOp::PopClip);
}
fn paint_source(
    p: &mut ScreenPainter<'_>,
    source: &PreparedContent,
    bounds: Rect,
) -> Result<(), String> {
    let image = source.image();
    let mut uv = Uv::new(0., 0., 1., 1.);
    let (width, height) = if source.avatar {
        let Some([x, y, max_x, max_y]) = source.alpha_bounds else {
            return Ok(());
        };
        let (width, height) = (max_x - x, max_y - y);
        uv = Uv::new(
            x as f32 / image.width() as f32,
            y as f32 / image.height() as f32,
            width as f32 / image.width() as f32,
            height as f32 / image.height() as f32,
        );
        (width as f32, height as f32)
    } else {
        (image.width() as f32, image.height() as f32)
    };
    let scale = (bounds.width / width).min(bounds.height / height).max(0.);
    let (width, height) = (width * scale, height * scale);
    p.list.ops.push(PaintOp::Image {
        image: source.resolve(p.images)?,
        rect: Uv::new(
            bounds.x + (bounds.width - width) / 2.,
            bounds.y + (bounds.height - height) / 2.,
            width,
            height,
        ),
        uv,
        sampling: Sampling::Nearest,
        tint: [1.; 4],
    });
    Ok(())
}
fn tile_reference(
    p: &mut ScreenPainter<'_>,
    session: Uuid,
    assets: &rusterix::Assets,
    name: &str,
    bounds: Rect,
    revision: ContentRevision,
    animation: usize,
) -> Result<bool, String> {
    let Some(tile) = screen_model::resolve_tile_reference(assets, name) else {
        return Ok(false);
    };
    let phase = tile.frame_index(animation);
    let Some(texture) = tile
        .textures
        .get(phase)
        .filter(|t| t.width > 0 && t.height > 0)
    else {
        return Ok(false);
    };
    let image = p.images.rgba_versioned(
        &format!("hud/tile/{session}/{}/{phase}", tile.id),
        revision.0,
        texture.width as u32,
        texture.height as u32,
        &texture.data,
    )?;
    p.list.ops.push(PaintOp::Image {
        image,
        rect: rect(bounds),
        uv: Uv::new(0., 0., 1., 1.),
        sampling: Sampling::Nearest,
        tint: [1.; 4],
    });
    Ok(true)
}

impl Hud {
    pub(in crate::game_screen) fn paint_command_drag(
        &self,
        p: &mut ScreenPainter<'_>,
        cursor: [f32; 2],
    ) -> Result<(), String> {
        card::ghost(p, &self.prepared, self.command_capture.as_ref(), cursor)
    }
}
