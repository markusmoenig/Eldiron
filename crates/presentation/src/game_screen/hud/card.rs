//! Native cards and drag previews resolve the same retained source leases.
use super::*;
use rusterix::client::widget::{ButtonVisualState, Widget};
pub(super) fn paint(
    p: &mut ScreenPainter<'_>,
    prepared: &PreparedHud,
    cursor: [f32; 2],
    capture: Option<&command::CommandCapture>,
) -> Result<(), String> {
    let frame = &prepared.frame;
    for card in &frame.cards {
        p.fill(card.rect, frame.style.slot);
        if let Some(source) = &prepared.skin.slot {
            sources::blit(p, source, card.rect, prepared.skin.slot_slice)?;
        }
        p.border(
            card.rect,
            frame.style.slot_border_width,
            if card.selected {
                frame.style.selected
            } else {
                frame.style.slot_border
            },
        );
        let pressed = capture.is_some_and(|capture| {
            !capture.cancelled
                && !capture.dragging
                && capture.receipt.control == frame.control(HudPart::CatalogEntry(card.index))
        });
        let state = if !card.state.enabled {
            ButtonVisualState::Disabled
        } else if pressed {
            ButtonVisualState::Pressed
        } else if card.selected {
            ButtonVisualState::Selected
        } else if rect(card.hit).contains(cursor) {
            ButtonVisualState::Hover
        } else {
            ButtonVisualState::Normal
        };
        if let Some(source) = prepared.icons.get(&(card.index, state as u8)) {
            p.list.ops.push(PaintOp::FilteredImage {
                image: source.resolve(p.images)?,
                rect: rect(card.icon_rect),
                uv: Uv::new(0., 0., 1., 1.),
                sampling: Sampling::Nearest,
                tint: [1.; 4],
                filter: Widget::command_icon_filter(state),
            });
        } else if let Some(name) = prepared.card_names.get(&card.index) {
            text(p, name, card.icon_rect, frame.style.text);
        }
        if card.state.cooldown_remaining > 0. {
            let fraction = (card.state.cooldown_remaining
                / card.state.cooldown_total.max(f32::EPSILON))
            .clamp(0., 1.);
            p.fill(
                Rect::new(
                    card.rect.x,
                    card.rect.y,
                    card.rect.width,
                    card.rect.height * fraction,
                ),
                [0, 0, 0, 150],
            );
            if let Some((_, layout)) = prepared.cooldowns.get(&card.index) {
                text(p, layout, card.rect, frame.style.text);
            }
        }
    }
    Ok(())
}
fn text(p: &mut ScreenPainter<'_>, layout: &TextLayout, bounds: Rect, pixel: [u8; 4]) {
    p.list.ops.push(PaintOp::PushClip(rect(bounds)));
    let origin = [
        bounds.x + (bounds.width - layout.width()) / 2.,
        bounds.y + (bounds.height - layout.height()) / 2.,
    ];
    for run in &layout.runs {
        p.list.ops.push(PaintOp::GlyphRun {
            run: run.clone(),
            origin,
            color: color(pixel),
        });
    }
    p.list.ops.push(PaintOp::PopClip);
}
pub(super) fn ghost(
    p: &mut ScreenPainter<'_>,
    prepared: &[PreparedHud],
    capture: Option<&command::CommandCapture>,
    cursor: [f32; 2],
) -> Result<(), String> {
    let Some(capture) = capture.filter(|capture| capture.dragging && !capture.cancelled) else {
        return Ok(());
    };
    let HudPart::CatalogEntry(index) = capture.receipt.control.part else {
        return Ok(());
    };
    let Some(panel) = prepared
        .iter()
        .find(|panel| panel.frame.id == capture.receipt.control.panel)
    else {
        return Ok(());
    };
    let bounds = Rect::new(cursor[0] - 14., cursor[1] - 14., 28., 28.);
    if let Some(source) = panel.icons.get(&(index, ButtonVisualState::Normal as u8)) {
        sources::blit(p, source, bounds, 0)?;
    } else if let Some(layout) = panel.card_names.get(&index) {
        p.fill(bounds, [10, 13, 13, 230]);
        text(p, layout, bounds, panel.frame.style.text);
    }
    Ok(())
}
