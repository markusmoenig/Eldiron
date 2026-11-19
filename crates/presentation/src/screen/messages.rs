//! Shaped transcript/choice layout is prepared before input and reused for paint.
use super::*;
use rustc_hash::{FxHashMap, FxHashSet, FxHasher};
use rusterix::{
    Assets, Choice, Map, Pixel, PixelSource, Rect, Tile,
    client::widget::messages::{MessageMetrics, MessagesWidget},
};
use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};
use theframework::{
    paint::{PaintOp, Sampling},
    text::{TextLayout, TextOptions, TextSystem},
};
use theframework_core::{ImageAsset, TheTime, Uuid};

#[derive(Clone)]
enum Portrait {
    Tile(Uuid),
    Solid(Pixel),
    Generated(Uuid, Arc<ImageAsset>),
}
struct PreparedMessage {
    key: u64,
    text: TextLayout,
    right: Option<TextLayout>,
    portrait: Option<Portrait>,
    portrait_space: f32,
    text_width: f32,
    primary_width: f32,
    right_width: f32,
    text_height: f32,
    block_height: f32,
}
struct PlacedMessage {
    id: Uuid,
    text: Rect,
    portrait: Rect,
    color: Pixel,
}
struct Prompt {
    key: u64,
    text: TextLayout,
}
struct PlacedPrompt {
    bounds: Rect,
    color: Pixel,
    align: Align,
}

/// Prefix and editable area share the transcript's shaped line metrics.
#[derive(Clone)]
pub struct MessageCommandLayout {
    pub hit: Rect,
    pub bounds: Rect,
    pub family: String,
    pub size: f32,
    pub color: Pixel,
}
/// CPU presentation state only. Keep one per transcript widget. All retained and
/// queued entries are measured before reveal/input; paint reuses the same layouts.
#[derive(Default)]
pub struct MessageLayouts {
    entries: FxHashMap<Uuid, PreparedMessage>,
    generated: FxHashMap<Uuid, Arc<ImageAsset>>,
    visible: Vec<PlacedMessage>,
    clip: Rect,
    bounds: Rect,
    background: Option<(Rect, Pixel)>,
    prompt: Option<Prompt>,
    placed_prompt: Option<PlacedPrompt>,
    layouts_built: u64,
    command_editor: bool,
    command: Option<MessageCommandLayout>,
}
fn tile_by_id<'a>(assets: &'a Assets, id: &Uuid) -> Option<&'a Tile> {
    assets
        .tiles
        .get(id)
        .or_else(|| assets.materials.get(id))
        .or_else(|| {
            assets
                .tile_indices
                .get(id)
                .and_then(|i| assets.tile_list.get(*i as usize))
        })
}
fn intersection(a: Rect, b: Rect) -> Rect {
    let r = rect(a).intersection(rect(b));
    if r.width <= 0. || r.height <= 0. {
        Rect::empty()
    } else {
        Rect::new(r.x, r.y, r.width, r.height)
    }
}
impl MessageLayouts {
    /// Native screen controllers paint and edit the line through EditorView.
    /// Standalone read-only previews may keep the static prompt/text layout.
    pub fn use_command_editor(&mut self, enabled: bool) {
        self.command_editor = enabled;
    }
    pub fn command(&self) -> Option<&MessageCommandLayout> {
        self.command.as_ref()
    }
    pub fn layouts_built(&self) -> u64 {
        self.layouts_built
    }
    pub fn visible_count(&self) -> usize {
        self.visible.len()
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    fn portrait_source(&mut self, source: &PixelSource, assets: &Assets) -> Option<Portrait> {
        if let Some(tile) = source.tile_ref_from_tile_list(assets) {
            return Some(Portrait::Tile(tile.id));
        }
        match source {
            PixelSource::Color(c) => Some(Portrait::Solid(c.to_u8_array())),
            PixelSource::Pixel(c) => Some(Portrait::Solid(*c)),
            PixelSource::PaletteIndex(i) => assets
                .palette
                .colors
                .get(*i as usize)
                .and_then(|c| c.as_ref())
                .map(|c| Portrait::Solid(c.to_u8_array())),
            PixelSource::Noise(noise) => {
                let id = noise.tile_id();
                if !self.generated.contains_key(&id) {
                    let texture = noise.to_tile(64).textures.pop()?;
                    let image =
                        ImageAsset::rgba(texture.width as u32, texture.height as u32, texture.data.into())
                            .ok()?;
                    self.generated.insert(id, Arc::new(image));
                }
                Some(Portrait::Generated(id, self.generated[&id].clone()))
            }
            _ => None,
        }
    }
    fn portrait(&mut self, sender: Option<u32>, map: &Map, assets: &Assets) -> Option<Portrait> {
        let entity = map.entities.iter().find(|e| Some(e.id) == sender)?;
        if let Some(source) = entity.attributes.get_source("portrait_tile_id") {
            return self.portrait_source(source, assets);
        }
        let id = entity.attributes.get_id("portrait_tile_id").or_else(|| {
            entity
                .attributes
                .get_str("portrait_tile_id")
                .and_then(|s| Uuid::parse_str(s.trim()).ok())
        })?;
        tile_by_id(assets, &id).map(|_| Portrait::Tile(id))
    }
    fn shape(
        &mut self,
        text: &mut TextSystem,
        content: &str,
        family: &str,
        size: f32,
        width: Option<f32>,
        line_height: f32,
    ) -> TextLayout {
        self.layouts_built += 1;
        text.layout_with_options(
            content,
            TextOptions {
                family,
                size,
                width,
                line_height: Some(line_height),
            },
            &[],
        )
    }
    /// Run before input and after controller mutations. Empty output slices are
    /// valid for resize/scroll/font changes; addressed outputs must be queued once.
    pub fn prepare(
        &mut self,
        widget: &mut MessagesWidget,
        text: &mut TextSystem,
        assets: &Assets,
        map: &Map,
        time: &TheTime,
        messages: &[rusterix::server::Message],
        choices: &[rusterix::MultipleChoice],
    ) -> Result<Option<FxHashMap<char, Choice>>, String> {
        widget.queue_messages(assets, map, time, messages, choices);
        let family = widget
            .table
            .get("ui")
            .and_then(|ui| ui.get("font"))
            .and_then(|f| f.as_str())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("Roboto")
            .to_owned();
        let size = if widget.font_size.is_finite() && widget.font_size > 0. {
            widget.font_size
        } else {
            20.
        };
        let spacing = widget.spacing.max(0.);
        let line_height = size + spacing;
        let gap = widget.message_spacing.max(0.);
        let portrait_size = widget.portrait_size.max(0.);
        let mut blocks = FxHashMap::default();
        for line in widget.layout_lines() {
            let portrait = if widget.portrait {
                self.portrait(line.sender_entity, map, assets)
            } else {
                None
            };
            let portrait_space = if portrait.is_some() {
                portrait_size + widget.portrait_gap.max(0.)
            } else {
                0.
            };
            let width = (widget.rect.width - portrait_space).max(size);
            let gutter = widget.column_width.max(0.).min(width * 0.25);
            let mut hash = FxHasher::default();
            (
                line.text.as_str(),
                family.as_str(),
                text.font_revision(),
                size.to_bits(),
                line_height.to_bits(),
                width.to_bits(),
                gutter.to_bits(),
                portrait_size.to_bits(),
            )
                .hash(&mut hash);
            let key = hash.finish();
            if self.entries.get(&line.id).is_none_or(|e| e.key != key) {
                let (left, right) = line
                    .text
                    .split_once('\u{1f}')
                    .map_or((line.text.as_str(), None), |(l, r)| (l, Some(r)));
                let right = right.map(|r| self.shape(text, r, &family, size, None, line_height));
                let right_width = right
                    .as_ref()
                    .map_or(0., |r| r.width().max(size).min(width * 0.45));
                let primary_width = if right.is_some() {
                    (width - right_width - gutter).max(1.)
                } else {
                    width
                };
                let right = match right {
                    Some(r) if r.width() > right_width => Some(self.shape(
                        text,
                        r.source.as_ref(),
                        &family,
                        size,
                        Some(right_width),
                        line_height,
                    )),
                    other => other,
                };
                let primary =
                    self.shape(text, left, &family, size, Some(primary_width), line_height);
                let text_height = primary
                    .height()
                    .max(right.as_ref().map_or(0., |r| r.height()))
                    .max(line_height);
                self.entries.insert(
                    line.id,
                    PreparedMessage {
                        key,
                        text: primary,
                        right,
                        portrait: None,
                        portrait_space,
                        text_width: width,
                        primary_width,
                        right_width,
                        text_height,
                        block_height: text_height.max(if portrait.is_some() {
                            portrait_size
                        } else {
                            0.
                        }),
                    },
                );
            }
            let entry = self.entries.get_mut(&line.id).unwrap();
            entry.portrait = portrait;
            entry.portrait_space = portrait_space;
            entry.block_height = entry.text_height.max(if entry.portrait.is_some() {
                portrait_size
            } else {
                0.
            });
            blocks.insert(line.id, entry.block_height);
        }
        let choices = widget.reveal_measured(MessageMetrics {
            blocks,
            line_height,
            block_gap: gap,
        })?;
        let retained = widget
            .layout_lines()
            .map(|l| l.id)
            .collect::<FxHashSet<_>>();
        self.entries.retain(|id, _| retained.contains(id));
        let generated = self
            .entries
            .values()
            .filter_map(|e| match &e.portrait {
                Some(Portrait::Generated(id, _)) => Some(*id),
                _ => None,
            })
            .collect::<FxHashSet<_>>();
        self.generated.retain(|id, _| generated.contains(id));
        self.bounds = widget.rect;
        self.visible.clear();
        self.placed_prompt = None;
        self.command = None;
        widget.set_return_to_choices_rect(Rect::empty());
        for line in &mut widget.messages {
            line.rect = Rect::empty();
        }
        self.background = widget.background.then(|| {
            let p = widget.background_padding.max(0.);
            (
                Rect::new(
                    widget.rect.x - p,
                    widget.rect.y - p,
                    widget.rect.width + p * 2.,
                    widget.rect.height + p * 2.,
                ),
                widget.background_color,
            )
        });
        if widget.rect.width <= 0. || widget.rect.height <= 0. {
            return Ok(choices);
        }
        let state = widget.view_state();
        let reserve = state.choices_below_view
            || (widget.active_choices.is_empty()
                && (state.command_input_enabled || (state.paused && !state.timed_pause)));
        self.clip = Rect::new(
            widget.rect.x,
            widget.rect.y,
            widget.rect.width,
            (widget.rect.height
                - if reserve {
                    state.line_height + state.block_gap
                } else {
                    0.
                })
            .max(0.),
        );
        let mut y = if widget.top_down {
            self.clip.y
        } else {
            self.clip.y + self.clip.height
        };
        let offset = state.scroll_offset.min(widget.messages.len());
        for line in widget.messages.iter_mut().rev().skip(offset) {
            let Some(entry) = self.entries.get(&line.id) else {
                continue;
            };
            let top = if widget.top_down {
                y
            } else {
                y - entry.block_height
            };
            let bounds = Rect::new(widget.rect.x, top, widget.rect.width, entry.block_height);
            line.rect = intersection(bounds, self.clip);
            if line.rect.width > 0. && line.rect.height > 0. {
                self.visible.push(PlacedMessage {
                    id: line.id,
                    text: Rect::new(
                        widget.rect.x + entry.portrait_space,
                        if widget.top_down {
                            top
                        } else {
                            y - entry.text_height
                        },
                        entry.text_width,
                        entry.text_height,
                    ),
                    portrait: Rect::new(widget.rect.x, top, portrait_size, portrait_size),
                    color: line.color,
                });
                if line.id == widget.clicked {
                    let color = &mut self.visible.last_mut().unwrap().color;
                    for c in &mut color[..3] {
                        *c = c.saturating_sub(100);
                    }
                }
            }
            y = if widget.top_down {
                top + entry.block_height + state.block_gap
            } else {
                top - state.block_gap
            };
            if (widget.top_down && y >= self.clip.y + self.clip.height)
                || (!widget.top_down && y <= self.clip.y)
            {
                break;
            }
        }
        let prompt = if state.choices_below_view {
            Some((
                "v Choices".to_owned(),
                widget
                    .active_choices
                    .first()
                    .map_or(widget.default_color, |l| l.color),
                Align::Right,
                true,
            ))
        } else if state.paused && !state.timed_pause {
            Some((
                widget.resolve_msg(&widget.continue_prompt, map, assets, time),
                widget.continue_prompt_color,
                Align::Right,
                false,
            ))
        } else if widget.active_choices.is_empty() && state.command_input_enabled {
            let label = if self.command_editor {
                format!("{} ", widget.command_prompt)
            } else if widget.command_active {
                format!("{} {}_", widget.command_prompt, widget.command_text)
            } else if widget.command_text.is_empty() {
                format!("{} ", widget.command_prompt)
            } else {
                format!("{} {}", widget.command_prompt, widget.command_text)
            };
            Some((label, widget.command_prompt_color, Align::Left, false))
        } else {
            None
        };
        if let Some((label, color, align, return_control)) = prompt {
            let mut hash = FxHasher::default();
            (
                label.as_str(),
                family.as_str(),
                text.font_revision(),
                size.to_bits(),
                line_height.to_bits(),
            )
                .hash(&mut hash);
            let key = hash.finish();
            if self.prompt.as_ref().is_none_or(|p| p.key != key) {
                let layout = self.shape(text, &label, &family, size, None, line_height);
                self.prompt = Some(Prompt { key, text: layout });
            }
            let mut bounds = Rect::new(
                widget.rect.x,
                widget.rect.y + widget.rect.height - state.line_height,
                widget.rect.width,
                state.line_height,
            );
            if return_control {
                bounds.width =
                    (self.prompt.as_ref().unwrap().text.width() + size).min(widget.rect.width);
                bounds.x = widget.rect.x + widget.rect.width - bounds.width;
                widget.set_return_to_choices_rect(intersection(bounds, widget.rect));
            }
            if self.command_editor
                && widget.active_choices.is_empty()
                && state.command_input_enabled
                && !state.paused
            {
                let prefix = self
                    .prompt
                    .as_ref()
                    .unwrap()
                    .text
                    .full_width()
                    .max(0.)
                    .min(bounds.width);
                self.command = Some(MessageCommandLayout {
                    hit: bounds,
                    bounds: Rect::new(
                        bounds.x + prefix,
                        bounds.y,
                        (bounds.width - prefix).max(0.),
                        bounds.height,
                    ),
                    family: family.clone(),
                    size,
                    color,
                });
            }
            self.placed_prompt = Some(PlacedPrompt {
                bounds,
                color,
                align,
            });
        } else {
            self.prompt = None;
        }
        Ok(choices)
    }
}
impl ScreenPainter<'_> {
    fn message_text(&mut self, layout: &TextLayout, bounds: Rect, pixel: Pixel, align: Align) {
        if bounds.width <= 0. || bounds.height <= 0. {
            return;
        }
        let x = bounds.x
            + match align {
                Align::Left => 0.,
                Align::Center => (bounds.width - layout.width()) * 0.5,
                Align::Right => bounds.width - layout.width(),
            };
        self.list.ops.push(PaintOp::PushClip(rect(bounds)));
        for run in &layout.runs {
            self.list.ops.push(PaintOp::GlyphRun {
                run: run.clone(),
                origin: [x, bounds.y],
                color: color(pixel),
            });
        }
        self.list.ops.push(PaintOp::PopClip);
    }
    /// Paint is a read-only consumer of the same prepared geometry used by input.
    pub fn messages(&mut self, layouts: &MessageLayouts, assets: &Assets) -> Result<(), String> {
        if layouts.bounds.width <= 0. || layouts.bounds.height <= 0. {
            return Ok(());
        }
        if let Some((bounds, color)) = layouts.background {
            self.fill(bounds, color);
        }
        self.list.ops.push(PaintOp::PushClip(rect(layouts.clip)));
        for placed in &layouts.visible {
            let entry = &layouts.entries[&placed.id];
            let visible_portrait = intersection(placed.portrait, layouts.clip);
            if visible_portrait.width > 0. && visible_portrait.height > 0. {
                match &entry.portrait {
                    Some(Portrait::Tile(id)) => {
                        if let Some(texture) =
                            tile_by_id(assets, id).and_then(|t| t.textures.first())
                        {
                            self.texture(
                                &format!("tile/{id}/0"),
                                texture,
                                placed.portrait,
                                theframework::paint::Rect::new(0., 0., 1., 1.),
                                Sampling::Nearest,
                                [1.; 4],
                            )?;
                        }
                    }
                    Some(Portrait::Solid(pixel)) => self.fill(placed.portrait, *pixel),
                    Some(Portrait::Generated(id, image)) => {
                        let handle = self.images.rgba(
                            &format!("portrait/{id}"),
                            image.width(),
                            image.height(),
                            image.pixels(),
                        )?;
                        self.list
                            .image(handle, rect(placed.portrait), Sampling::Nearest);
                    }
                    None => {}
                }
            }
            let mut left = placed.text;
            left.width = entry.primary_width;
            self.message_text(&entry.text, left, placed.color, Align::Left);
            if let Some(right) = &entry.right {
                let bounds = Rect::new(
                    placed.text.x + placed.text.width - entry.right_width,
                    placed.text.y,
                    entry.right_width,
                    placed.text.height,
                );
                self.message_text(right, bounds, placed.color, Align::Right);
            }
        }
        self.list.ops.push(PaintOp::PopClip);
        if let (Some(placed), Some(prompt)) = (&layouts.placed_prompt, &layouts.prompt) {
            self.list.ops.push(PaintOp::PushClip(rect(layouts.bounds)));
            self.message_text(&prompt.text, placed.bounds, placed.color, placed.align);
            self.list.ops.push(PaintOp::PopClip);
        }
        Ok(())
    }
}
