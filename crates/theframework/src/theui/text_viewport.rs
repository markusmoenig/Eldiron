//! Shared, renderer-independent geometry for scrolling text surfaces.
use crate::paint::Rect;
pub(crate) struct TextViewportPlan {
    pub viewport: Rect,
    pub horizontal: Option<Rect>,
    pub vertical: Option<Rect>,
}
pub(crate) fn text_content_rect(outer: Rect, padding: (i32, i32, i32, i32)) -> Rect {
    Rect::new(
        outer.x + padding.0 as f32,
        outer.y + padding.1 as f32,
        (outer.width - (padding.0 + padding.2) as f32).max(0.),
        (outer.height - (padding.1 + padding.3) as f32).max(0.),
    )
}
pub(crate) fn plan_text_viewport(
    outer: Rect,
    padding: (i32, i32, i32, i32),
    size: [f32; 2],
    bar: f32,
    wrap: bool,
) -> TextViewportPlan {
    let content = text_content_rect(outer, padding);
    let (mut horizontal, mut vertical) = (size[0] > content.width, size[1] > content.height);
    if horizontal || vertical {
        horizontal = !wrap && size[0] > (content.width - bar).max(0.);
        vertical = size[1] > (content.height - bar).max(0.);
    }
    TextViewportPlan {
        viewport: Rect::new(
            content.x,
            content.y,
            (content.width - if vertical { bar } else { 0. }).max(0.),
            (content.height - if horizontal { bar } else { 0. }).max(0.),
        ),
        horizontal: horizontal.then(|| {
            Rect::new(
                outer.x,
                outer.y + outer.height - bar,
                (outer.width - if vertical { bar } else { 0. }).max(0.),
                bar,
            )
        }),
        vertical: vertical.then(|| {
            Rect::new(
                outer.x + outer.width - bar,
                outer.y,
                bar,
                (outer.height - if horizontal { bar } else { 0. }).max(0.),
            )
        }),
    }
}
