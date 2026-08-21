use crate::prelude::*;

#[derive(Clone)]
pub(crate) struct TheNodeChromePaints {
    header: ThePaint,
    body: ThePaint,
    footer: ThePaint,
    preview: ThePaint,
    border: ThePaint,
}

impl TheNodeChromePaints {
    pub(crate) fn resolve(
        style: &mut Box<dyn TheStyle>,
        node_width: i32,
        node_height: i32,
        preview_height: i32,
        selected: bool,
    ) -> Self {
        let body_bounds = ThePixelRect::new(0, 0, node_width, node_height);
        let header_bounds = ThePixelRect::new(1, 1, node_width.saturating_sub(2), 18);
        let footer_bounds = ThePixelRect::new(
            1,
            node_height.saturating_sub(19),
            node_width.saturating_sub(2),
            18,
        );
        let preview_bounds = ThePixelRect::new(
            3,
            node_height
                .saturating_sub(19)
                .saturating_sub(preview_height)
                .saturating_add(2),
            node_width.saturating_sub(6),
            preview_height.saturating_sub(4),
        );
        let (header_role, body_role, footer_role, border_role) = if selected {
            (
                NodeHeaderSelectedChrome,
                NodeBodySelectedChrome,
                NodeFooterSelectedChrome,
                NodeBorderSelected,
            )
        } else {
            (
                NodeHeaderNormalChrome,
                NodeBodyNormalChrome,
                NodeFooterNormalChrome,
                NodeBorder,
            )
        };
        Self {
            header: style.theme().paint(header_role, header_bounds),
            body: style.theme().paint(body_role, body_bounds),
            footer: style.theme().paint(footer_role, footer_bounds),
            preview: style.theme().paint(NodePreviewBackground, preview_bounds),
            border: ThePaint::solid(*style.theme().color(border_role)),
        }
    }
}

pub(crate) fn draw_node_chrome(
    buffer: &mut TheRGBABuffer,
    preview_height: i32,
    paints: &TheNodeChromePaints,
) {
    let width = buffer.dim().width.max(0) as usize;
    let height = buffer.dim().height.max(0) as usize;
    draw_node_chrome_pixels(
        buffer.pixels_mut(),
        width,
        height,
        preview_height,
        paints,
        &mut ThePainter::new(),
    );
}

fn draw_node_chrome_pixels(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    preview_height: i32,
    paints: &TheNodeChromePaints,
    painter: &mut ThePainter,
) {
    let Ok(mut surface) = TheSurfaceMut::new(pixels, width, height) else {
        return;
    };
    let bounds = ThePixelRect::new(0, 0, width as i32, height as i32);
    surface.set_clip(bounds);

    let inner = ThePixelRect::new(
        1,
        1,
        bounds.width.saturating_sub(2),
        bounds.height.saturating_sub(2),
    );
    painter.fill_round_rect(&mut surface, bounds, 4.0, &paints.border);
    painter.fill_round_rect(&mut surface, inner, 3.0, &paints.body);

    let header = ThePixelRect::new(1, 1, bounds.width.saturating_sub(2), 18);
    painter.fill_round_rect(&mut surface, header, 3.0, &paints.header);
    painter.fill_rect(
        &mut surface,
        ThePixelRect::new(header.x, header.y.saturating_add(8), header.width, 10),
        &paints.header,
    );
    painter.fill_rect(
        &mut surface,
        ThePixelRect::new(1, 18, bounds.width.saturating_sub(2), 1),
        &paints.border,
    );

    let footer = ThePixelRect::new(
        1,
        bounds.height.saturating_sub(19),
        bounds.width.saturating_sub(2),
        18,
    );
    painter.fill_round_rect(&mut surface, footer, 3.0, &paints.footer);
    painter.fill_rect(
        &mut surface,
        ThePixelRect::new(footer.x, footer.y, footer.width, 10),
        &paints.footer,
    );
    painter.fill_rect(
        &mut surface,
        ThePixelRect::new(1, footer.y, bounds.width.saturating_sub(2), 1),
        &paints.border,
    );

    if preview_height > 0 {
        let preview = ThePixelRect::new(
            3,
            footer.y.saturating_sub(preview_height).saturating_add(2),
            bounds.width.saturating_sub(6),
            preview_height.saturating_sub(4),
        );
        let preview_inner = ThePixelRect::new(
            preview.x.saturating_add(1),
            preview.y.saturating_add(1),
            preview.width.saturating_sub(2),
            preview.height.saturating_sub(2),
        );
        painter.fill_round_rect(&mut surface, preview, 2.0, &paints.border);
        painter.fill_round_rect(&mut surface, preview_inner, 1.0, &paints.preview);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_chrome_preserves_guard_bytes() {
        const WIDTH: usize = 24;
        const HEIGHT: usize = 20;
        const GUARD: usize = 32;
        const SENTINEL: u8 = 0xc1;

        let body_len = WIDTH * HEIGHT * 4;
        let mut pixels = vec![0; body_len + GUARD];
        pixels[body_len..].fill(SENTINEL);
        let paints = TheNodeChromePaints {
            header: ThePaint::solid([45, 55, 70, 255]),
            body: ThePaint::solid([20, 21, 23, 255]),
            footer: ThePaint::solid([15, 16, 18, 255]),
            preview: ThePaint::solid([10, 11, 13, 255]),
            border: ThePaint::solid([83, 151, 207, 255]),
        };
        draw_node_chrome_pixels(
            &mut pixels,
            WIDTH,
            HEIGHT,
            12,
            &paints,
            &mut ThePainter::new(),
        );

        assert!(pixels[..body_len].iter().any(|byte| *byte != 0));
        assert!(pixels[body_len..].iter().all(|byte| *byte == SENTINEL));
    }
}
