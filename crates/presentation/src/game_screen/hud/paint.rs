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
        let Some(prepared) = &self.prepared else {
            return Ok(());
        };
        let frame = &prepared.frame;
        p.list.ops.push(PaintOp::PushClip(rect(frame.rect)));
        p.fill(frame.rect, frame.template.background_color);
        for (tile, bounds) in screen_model::container_frame_tiles(frame.rect, &frame.template) {
            tile_reference(
                p,
                world.session_id,
                world.assets,
                tile,
                bounds,
                revision,
                animation,
            )?;
        }
        p.border(frame.rect, 1., frame.template.border_color);
        if let Some(title) = frame.title_bar {
            p.fill(title, [20, 24, 30, 220]);
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
                    color: color([236, 233, 214, 255]),
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
        for slot in &frame.slots {
            if !frame
                .template
                .tiles
                .slot
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
                p.fill(slot.rect, frame.template.slot_color);
            }
            p.border(slot.rect, 1., frame.template.slot_border_color);
            if let Some(source) = prepared.items.get(&slot.index) {
                let size = source.image();
                let mut bounds = slot.rect.with_border(4.);
                let scale =
                    (bounds.width / size.width() as f32).min(bounds.height / size.height() as f32);
                let (width, height) = (size.width() as f32 * scale, size.height() as f32 * scale);
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
        p.list.ops.push(PaintOp::PopClip);
        if frame.max_scroll[1] > 0. && frame.clip.height > 0. {
            let height = (frame.clip.height as f64 * frame.clip.height as f64
                / (frame.clip.height as f64 + frame.max_scroll[1]))
                .max(8.)
                .min(frame.clip.height as f64) as f32;
            let y = frame.clip.y
                + (frame.clip.height - height) * (frame.scroll[1] / frame.max_scroll[1]) as f32;
            p.fill(
                Rect::new(frame.rect.x + frame.rect.width - 3., y, 2., height),
                frame.template.border_color,
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
                frame.template.border_color,
            );
        }
        p.list.ops.push(PaintOp::PopClip);
        Ok(())
    }
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
