//! Retained world descriptions. CPU ownership and anchor policy are shared with
//! reference clients; all displayed text is shaped once and emitted as glyphs.
use super::*;
use rusterix::world_text::{WorldTexts, anchor_position, carrier, project};
use std::time::Duration;
use theframework::{
    platform::{InputEvent, PhysicalKey, PointerButton},
    text::{TextAlignment, TextLayout, TextOptions},
};

struct Prepared {
    width: f32,
    content_width: f32,
    fonts: GameFontVersion,
    text: TextLayout,
    footer: Option<(usize, usize, TextLayout)>,
}
struct Placed {
    id: Uuid,
    bounds: Uv,
    clip: Uv,
    text: Uv,
    offset: f32,
    lines: std::ops::Range<usize>,
}
#[derive(Default)]
pub struct WorldTextOverlay {
    session: Option<Uuid>,
    player: rusterix::party::PlayerView,
    state: WorldTexts,
    prepared: FxHashMap<Uuid, Prepared>,
    placed: Vec<Placed>,
    escape_capture: bool,
    layouts_built: u64,
}
impl WorldTextOverlay {
    pub(super) fn bind_runtime(&mut self, world: &RuntimeView<'_>) {
        if self.session != Some(world.session_id) || self.player != world.player {
            self.session = Some(world.session_id);
            self.player = world.player;
            self.state = WorldTexts::default();
            self.prepared.clear();
            self.invalidate_geometry();
        }
    }
    pub fn passages(&self) -> &[rusterix::world_text::WorldPassage] {
        self.state.passages()
    }
    pub fn visible_bounds(&self) -> impl Iterator<Item = Uv> + '_ {
        self.placed.iter().map(|p| p.bounds)
    }
    pub fn layouts_built(&self) -> u64 {
        self.layouts_built
    }
    /// A screen/layout replacement retains authored passages and shaped text,
    /// while revoking hit geometry and key capture from the previous screen.
    pub fn invalidate_geometry(&mut self) {
        self.placed.clear();
        self.escape_capture = false;
    }
    pub fn advance(&mut self, elapsed: Duration) {
        self.state.advance(elapsed);
    }
    pub fn receive(&mut self, world: &RuntimeView<'_>, events: &RegionEvents) {
        self.bind_runtime(world);
        if let Some(map) = world.map {
            self.state.receive_for(
                map,
                world.assets,
                &world.time,
                &events.world_text,
                world.player,
            );
        }
    }
    pub(super) fn prepare(
        &mut self,
        world: &RuntimeView<'_>,
        elements: &ScreenElements,
        fonts: &mut GameFonts,
    ) {
        self.bind_runtime(world);
        self.placed.clear();
        self.state.expire(world.map.map(|m| m.id));
        self.prepared
            .retain(|id, _| self.state.passages().iter().any(|p| p.id == *id));
        let Some(map) = world.map else {
            return;
        };
        let screen = Uv::new(
            0.,
            0.,
            elements.layout.viewport.x as f32,
            elements.layout.viewport.y as f32,
        );
        let mut pages = Vec::new();
        for passage in self.state.passages() {
            let Some(position) = anchor_position(&passage.message.anchor, map, world.assets) else {
                continue;
            };
            // Authored order is deterministic when multiple views can show the
            // same target. Hidden views cannot contribute input/paint geometry.
            let target = ordered(&elements.game_widgets).into_iter().find_map(|id| {
                let game = &elements.game_widgets[&id];
                if screen_model::widget_name_is_hidden(&game.name, &elements.widgets_to_hide) {
                    return None;
                }
                let clip = rect(game.rect).intersection(screen);
                if ![clip.x, clip.y, clip.width, clip.height]
                    .iter()
                    .all(|v| v.is_finite())
                    || clip.width < 36.
                    || clip.height < 36.
                {
                    return None;
                }
                let point = project(game, position).or_else(|| {
                    carrier(&passage.message.anchor, map)
                        .filter(|e| {
                            world
                                .player_entity()
                                .is_some_and(|player| player.id == e.id)
                        })
                        .map(|_| {
                            scenevm::prelude::Vec2::new(
                                game.rect.x + game.rect.width * 0.5,
                                game.rect.y + game.rect.height * 0.55,
                            )
                        })
                })?;
                Some((clip, point))
            });
            let Some((clip, point)) = target else {
                continue;
            };
            let max_width = (clip.width - 12.).min(320.);
            let fonts_version = fonts.version();
            let prepared = self.prepared.entry(passage.id).or_insert_with(|| Prepared {
                width: -1.,
                content_width: 0.,
                fonts: fonts_version,
                text: fonts.text.layout("", "Roboto", 15., None),
                footer: None,
            });
            if prepared.width != max_width || prepared.fonts != fonts_version {
                let measure = fonts.text.layout_with_options(
                    &passage.message.text,
                    TextOptions {
                        family: "Roboto",
                        size: 15.,
                        width: Some(max_width - 16.),
                        line_height: Some(19.5),
                    },
                    &[],
                );
                let width = (measure
                    .lines()
                    .iter()
                    .map(|l| l.advance)
                    .fold(0., f32::max)
                    .ceil()
                    + 1.)
                    .clamp(1., max_width - 16.);
                prepared.text = fonts.text.layout_aligned(
                    &passage.message.text,
                    TextOptions {
                        family: "Roboto",
                        size: 15.,
                        width: Some(width),
                        line_height: Some(19.5),
                    },
                    &[],
                    TextAlignment::Center,
                );
                prepared.width = max_width;
                prepared.content_width = width;
                prepared.fonts = fonts_version;
                prepared.footer = None;
                self.layouts_built += 1;
            }
            let line_count = prepared.text.line_count();
            if line_count == 0 {
                continue;
            }
            let bottom = elements
                .button_widgets
                .values()
                .filter(|button| {
                    !screen_model::widget_name_is_hidden(&button.name, &elements.widgets_to_hide)
                        && rect(button.rect).intersection(clip).width > 0.
                        && rect(button.rect).intersection(clip).height > 0.
                        && button.rect.y > clip.y + clip.height * 0.5
                })
                .map(|button| button.rect.y - 4.)
                .fold(clip.y + clip.height - 6., f32::min);
            let min_y = clip.y + 6.;
            let available = (bottom - min_y).max(1.);
            let per_page = (((available - 12. - 19.5) / 19.5).floor() as usize).clamp(1, 4);
            let count = line_count.div_ceil(per_page).max(1);
            let page = passage.page.min(count - 1);
            pages.push((passage.id, count));
            let range = page * per_page..((page + 1) * per_page).min(line_count);
            let top = prepared.text.line(range.start).unwrap().top;
            let text_height = prepared.text.line(range.end - 1).unwrap().bottom - top;
            let footer_height = if count > 1 && available >= 51. {
                19.5
            } else {
                0.
            };
            let width = (prepared.content_width + 16.).clamp(24., max_width);
            let height = (text_height + footer_height + 12.).min(available);
            let max_y = (bottom - height).max(min_y);
            let x = (point.x - width * 0.5).clamp(
                clip.x + 6.,
                (clip.x + clip.width - width - 6.).max(clip.x + 6.),
            );
            let mut bounds = Uv::new(
                x,
                (point.y - height - 10.).clamp(min_y, max_y),
                width,
                height,
            );
            for _ in 0..4 {
                let Some(other) = self.placed.iter().find(|other| {
                    let r = other.bounds.intersection(bounds);
                    r.width > 0. && r.height > 0.
                }) else {
                    break;
                };
                let above = other.bounds.y - height - 6.;
                bounds.y = if above >= min_y {
                    above
                } else {
                    (other.bounds.y + other.bounds.height + 6.).min(max_y)
                };
            }
            if footer_height > 0. {
                if prepared
                    .footer
                    .as_ref()
                    .is_none_or(|(p, c, _)| *p != page || *c != count)
                {
                    prepared.footer = Some((
                        page,
                        count,
                        fonts.text.layout(
                            &format!("{} / {}  ›", page + 1, count),
                            "Roboto",
                            15.,
                            None,
                        ),
                    ));
                }
            } else {
                prepared.footer = None;
            }
            self.placed.push(Placed {
                id: passage.id,
                bounds,
                clip,
                text: Uv::new(
                    bounds.x + 8.,
                    bounds.y + 6.,
                    (bounds.width - 16.).max(0.),
                    (height - footer_height - 12.).max(0.),
                ),
                offset: top,
                lines: range,
            });
        }
        for (id, count) in pages {
            self.state.set_page_count(id, count);
        }
    }
    pub(super) fn paint(&self, list: &mut PaintList) {
        for placed in &self.placed {
            let prepared = &self.prepared[&placed.id];
            list.ops.push(PaintOp::PushClip(placed.clip));
            list.rect(placed.bounds, 0., color([18, 18, 16, 215]));
            list.ops.push(PaintOp::PushClip(placed.text));
            for run in prepared.text.runs_for_lines(placed.lines.clone()) {
                list.ops.push(PaintOp::GlyphRun {
                    run: run.clone(),
                    origin: [placed.text.x, placed.text.y - placed.offset],
                    color: color([229, 229, 1, 255]),
                });
            }
            list.ops.push(PaintOp::PopClip);
            if let Some((_, _, footer)) = &prepared.footer {
                let area = Uv::new(
                    placed.bounds.x + 8.,
                    placed.bounds.y + placed.bounds.height - 25.5,
                    placed.bounds.width - 16.,
                    19.5,
                );
                list.ops.push(PaintOp::PushClip(area));
                for run in &footer.runs {
                    list.ops.push(PaintOp::GlyphRun {
                        run: run.clone(),
                        origin: [area.x + (area.width - footer.width()).max(0.) * 0.5, area.y],
                        color: color([183, 174, 151, 255]),
                    });
                }
                list.ops.push(PaintOp::PopClip);
            }
            list.ops.push(PaintOp::PopClip);
        }
    }
    pub(super) fn input(&mut self, event: &InputEvent) -> bool {
        match event {
            InputEvent::Focus(false) => {
                self.escape_capture = false;
                false
            }
            InputEvent::Key {
                physical: PhysicalKey::Escape,
                pressed,
                repeat,
                ..
            } => {
                if !pressed {
                    return std::mem::take(&mut self.escape_capture);
                }
                if self.escape_capture {
                    return true;
                }
                if !repeat && self.state.clear() {
                    self.escape_capture = true;
                    self.placed.clear();
                    self.prepared.clear();
                    true
                } else {
                    false
                }
            }
            InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: true,
                position,
            } => {
                let Some(id) = self
                    .placed
                    .iter()
                    .rev()
                    .find(|p| p.bounds.contains(*position) && p.clip.contains(*position))
                    .map(|p| p.id)
                else {
                    return false;
                };
                self.state.activate(id)
            }
            InputEvent::PointerMoved { position } => self
                .placed
                .iter()
                .any(|p| p.bounds.contains(*position) && p.clip.contains(*position)),
            _ => false,
        }
    }
}
