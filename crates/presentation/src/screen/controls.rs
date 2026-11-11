use super::*;
use rustc_hash::FxHashMap;
use rusterix::{
    Assets, Currencies, Entity, Map, Rect,
    client::{
        draw2d::TheHorizontalAlign,
        widget::{
            TextInputWidget,
            choice::{ChoiceWidget, ChoiceWidgetKind},
            stat::StatWidget,
            text::TextWidget,
        },
    },
};
use theframework::paint::{PaintOp, Rect as Uv, Sampling};
use theframework_core::TheTime;

impl ScreenPainter<'_> {
    pub fn text_widget(
        &mut self,
        widget: &TextWidget,
        map: &Map,
        assets: &Assets,
        currencies: &Currencies,
        time: &TheTime,
        state: &FxHashMap<String, String>,
    ) {
        let family = widget
            .table
            .get("ui")
            .and_then(|v| v.get("font"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let align = match widget.horizontal_align {
            TheHorizontalAlign::Center => Align::Center,
            TheHorizontalAlign::Right => Align::Right,
            _ => Align::Left,
        };
        self.list.ops.push(PaintOp::PushClip(rect(widget.rect)));
        let mut y = widget.rect.y;
        for line in widget.resolved_lines(map, currencies, assets, time, state) {
            if line.contains('\t') {
                for (index, segment) in line.split('\t').enumerate() {
                    let x = widget.rect.x + widget.tab_width * index as f32;
                    self.label(
                        Rect::new(
                            x,
                            y,
                            (widget.rect.width - (x - widget.rect.x)).max(0.),
                            widget.font_size,
                        ),
                        segment,
                        family,
                        widget.font_size,
                        widget.color,
                        Align::Left,
                        false,
                    );
                }
            } else {
                self.label(
                    Rect::new(widget.rect.x, y, widget.rect.width, widget.font_size),
                    &line,
                    family,
                    widget.font_size,
                    widget.color,
                    align,
                    false,
                );
            }
            y += widget.font_size + widget.spacing;
        }
        self.list.ops.push(PaintOp::PopClip);
    }
    pub fn text_input_chrome(&mut self, widget: &TextInputWidget, focused: bool) {
        self.fill(widget.rect, widget.background_color);
        self.border(
            widget.rect,
            widget.border_size.max(if focused { 1 } else { 0 }) as f32,
            if focused {
                [238, 214, 118, 255]
            } else {
                widget.border_color
            },
        );
    }
    /// Stateless preview adapter. Interactive screens retain an EditorView so
    /// selection, composition, history and glyph layout survive each frame.
    pub fn text_input(&mut self, widget: &TextInputWidget, focused: bool) {
        self.text_input_chrome(widget, focused);
        let mut view = theframework::text_edit::EditorView::new(false);
        view.editor.set_text(self.text, &widget.text);
        view.set_focused(self.text, focused);
        view.prepare(
            self.text,
            theframework::text::TextOptions {
                family: &widget.font,
                size: widget.font_size,
                width: None,
                line_height: None,
            },
            rect(Rect::new(
                widget.rect.x + 8.,
                widget.rect.y,
                (widget.rect.width - 16.).max(0.),
                widget.rect.height,
            )),
            true,
        );
        view.paint(
            self.text,
            self.list,
            theframework::text_edit::EditorColors {
                text: color(widget.color),
                ..Default::default()
            },
            true,
        );
    }
    pub fn stat(
        &mut self,
        widget: &StatWidget,
        assets: &Assets,
        entity: Option<&Entity>,
        frame: usize,
    ) -> Result<(), String> {
        let visual = widget.presentation(assets, entity);
        self.fill(visual.rect, visual.background);
        if !visual.tile_mode {
            self.fill(visual.fill, visual.color);
        } else if let Some(id) = visual.source {
            if let Some(tile) = assets.tiles.get(&id).or_else(|| assets.materials.get(&id)) {
                if let Some(texture) = tile.textures.get(tile.frame_index(frame)) {
                    if visual.clip_source {
                        self.list.ops.push(PaintOp::PushClip(rect(visual.fill)));
                    }
                    self.texture(
                        &format!("tile/{id}/{}", tile.frame_index(frame)),
                        texture,
                        visual.rect,
                        Uv::new(0., 0., 1., 1.),
                        Sampling::Nearest,
                        [1.; 4],
                    )?;
                    if visual.clip_source {
                        self.list.ops.push(PaintOp::PopClip);
                    }
                }
            }
        }
        self.border(visual.rect, visual.border_width, visual.border);
        Ok(())
    }
    pub fn choice(&mut self, widget: &ChoiceWidget, cursor: [f32; 2]) {
        let item = |p: &mut Self, bounds: Rect, label: &str, selected: bool, base| {
            let hovered = bounds.contains(scenevm::prelude::Vec2::new(cursor[0], cursor[1]));
            p.fill(
                bounds,
                if selected {
                    widget.selected_color
                } else if hovered {
                    widget.hover_color
                } else {
                    base
                },
            );
            p.border(
                bounds,
                widget.border_size.max(0) as f32,
                widget.border_color,
            );
            p.label(
                Rect::new(
                    bounds.x + widget.text_padding,
                    bounds.y,
                    (bounds.width - widget.text_padding * 2.).max(0.),
                    bounds.height,
                ),
                label,
                &widget.font,
                widget.font_size,
                if selected {
                    widget.text_color
                } else {
                    widget.muted_text_color
                },
                Align::Left,
                true,
            );
            if selected && widget.indicator_size > 0. {
                p.fill(
                    Rect::new(
                        bounds.x,
                        bounds.y + bounds.height - widget.indicator_size,
                        bounds.width,
                        widget.indicator_size,
                    ),
                    widget.indicator_color,
                );
            }
        };
        match widget.kind {
            ChoiceWidgetKind::TabBar => {
                for (i, option) in widget.options.iter().enumerate() {
                    item(
                        self,
                        widget.tab_rect(i),
                        &option.label,
                        i == widget.selected,
                        widget.background_color,
                    );
                }
            }
            ChoiceWidgetKind::Dropdown => {
                item(
                    self,
                    widget.rect,
                    widget
                        .options
                        .get(widget.selected)
                        .map(|o| o.label.as_str())
                        .unwrap_or(""),
                    widget.open,
                    widget.background_color,
                );
                let w = widget.rect.height.min(widget.rect.width) * 0.25;
                let x = widget.rect.x + widget.rect.width - widget.text_padding - w;
                let y = widget.rect.y + widget.rect.height * 0.5;
                self.list.ops.push(PaintOp::Fill {
                    path: std::sync::Arc::new(theframework::paint::Path {
                        elements: vec![
                            theframework::paint::PathElement::Move([x, y - w * 0.25]),
                            theframework::paint::PathElement::Line([x + w, y - w * 0.25]),
                            theframework::paint::PathElement::Line([x + w * 0.5, y + w * 0.5]),
                            theframework::paint::PathElement::Close,
                        ],
                    }),
                    rule: theframework::paint::FillRule::NonZero,
                    brush: color(widget.indicator_color).into(),
                });
                if widget.open {
                    self.fill(widget.popup_rect(), widget.panel_color);
                    self.border(
                        widget.popup_rect(),
                        widget.border_size.max(0) as f32,
                        widget.border_color,
                    );
                    for (i, option) in widget.options.iter().enumerate() {
                        item(
                            self,
                            widget.dropdown_item_rect(i),
                            &option.label,
                            i == widget.selected,
                            widget.panel_color,
                        );
                    }
                }
            }
        }
    }
}
