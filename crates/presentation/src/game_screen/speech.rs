//! Simulation-clocked speech, projected independently in every game view. The
//! shared CPU owner receives Say batches; paint never drains or redelivers them.
use super::*;
use rusterix::{
    ServerState,
    speech::{SpeechMessages, Utterance, style},
    world_text::{anchor_position, project},
};
use theframework::text::{TextAlignment, TextLayout, TextOptions};

struct ViewStyle {
    source: String,
    say: Option<toml::Table>,
}
struct Prepared {
    fonts: GameFontVersion,
    text: TextLayout,
    width: f32,
}
struct Placed {
    view: Uuid,
    key: (Uuid, u32),
    bounds: Uv,
    clip: Uv,
    text: Uv,
    lines: usize,
    foreground: rusterix::Pixel,
    background: Option<rusterix::Pixel>,
}
#[derive(Default)]
pub struct SpeechOverlay {
    state: SpeechMessages,
    source: String,
    config: toml::Table,
    styles: FxHashMap<Uuid, ViewStyle>,
    prepared: FxHashMap<(Uuid, u32), Prepared>,
    placed: Vec<Placed>,
    layouts_built: u64,
}
impl SpeechOverlay {
    pub fn messages(&self) -> &[Utterance] {
        self.state.messages()
    }
    pub fn visible_bounds(&self) -> impl Iterator<Item = (Uuid, Uv)> + '_ {
        self.placed.iter().map(|p| (p.view, p.bounds))
    }
    pub fn layouts_built(&self) -> u64 {
        self.layouts_built
    }
    pub fn invalidate_geometry(&mut self) {
        self.placed.clear();
    }
    pub fn clear(&mut self) {
        self.state.clear();
        self.placed.clear();
        self.prepared.clear();
    }
    pub fn receive(&mut self, world: &RuntimeView<'_>, events: &RegionEvents) {
        if world.state == ServerState::Off {
            return;
        }
        if let Some(map) = world.map {
            self.state.receive(map, &world.time, &events.says);
        }
    }
    pub(super) fn prepare(
        &mut self,
        world: &RuntimeView<'_>,
        elements: &ScreenElements,
        fonts: &mut GameFonts,
    ) {
        self.placed.clear();
        self.state.synchronize(world.map, &world.time);
        if world.state == ServerState::Off {
            self.clear();
            return;
        }
        if self.source != world.config() {
            self.source = world.config().into();
            self.config = self.source.parse().unwrap_or_default();
        }
        self.styles
            .retain(|id, _| elements.game_widgets.contains_key(id));
        let mut maximum = if elements.game_widgets.is_empty() {
            style::duration_ticks(&self.config, None)
        } else {
            1
        };
        for (id, game) in &elements.game_widgets {
            let view = self.styles.entry(*id).or_insert_with(|| ViewStyle {
                source: String::new(),
                say: None,
            });
            if view.source != game.toml_str {
                view.source = game.toml_str.clone();
                view.say =
                    game.toml_str.parse::<toml::Table>().ok().and_then(|table| {
                        table.get("say").and_then(toml::Value::as_table).cloned()
                    });
            }
            maximum = maximum.max(style::duration_ticks(&self.config, view.say.as_ref()));
        }
        let rate = style::ticks_per_minute(&self.config);
        self.state.expire(maximum, rate);
        let mut used = std::collections::HashSet::new();
        let screen = Uv::new(
            0.,
            0.,
            elements.layout.viewport.x as f32,
            elements.layout.viewport.y as f32,
        );
        if let Some(map) = world.map {
            for id in ordered(&elements.game_widgets) {
                let game = &elements.game_widgets[&id];
                if screen_model::widget_name_is_hidden(&game.name, &elements.widgets_to_hide) {
                    continue;
                }
                let clip = rect(game.rect).intersection(screen);
                if ![clip.x, clip.y, clip.width, clip.height]
                    .iter()
                    .all(|v| v.is_finite())
                    || clip.width < 36.
                    || clip.height < 36.
                {
                    continue;
                }
                let local = self.styles[&id].say.as_ref();
                let duration = style::duration_ticks(&self.config, local);
                for message in self.state.messages() {
                    if !self.state.is_live(message, duration, rate) {
                        continue;
                    }
                    let Some(mut position) =
                        anchor_position(&message.target.anchor(), map, world.assets)
                    else {
                        continue;
                    };
                    // Flat 2D labels retain the reference one-cell head offset.
                    // Isometric/3D views use the actual elevated world anchor.
                    if screen_model::is_2d_camera(&game.camera) && !game.camera.is_isometric_2d() {
                        position.y = 1.;
                    }
                    let Some(point) = project(game, position) else {
                        continue;
                    };
                    let width = clip.width - 22.;
                    let key = (message.id, width.to_bits());
                    used.insert(key);
                    let version = fonts.version();
                    if self.prepared.get(&key).is_none_or(|p| p.fonts != version) {
                        let measured = fonts.text.layout(&message.text, "Roboto", 15., Some(width));
                        let width = (measured
                            .lines()
                            .iter()
                            .map(|line| line.advance)
                            .fold(0., f32::max)
                            .ceil()
                            + 1.)
                            .clamp(1., width);
                        let text = fonts.text.layout_aligned(
                            &message.text,
                            TextOptions {
                                family: "Roboto",
                                size: 15.,
                                width: Some(width),
                                line_height: Some(19.5),
                            },
                            &[],
                            TextAlignment::Center,
                        );
                        self.prepared.insert(
                            key,
                            Prepared {
                                fonts: version,
                                text,
                                width,
                            },
                        );
                        self.layouts_built += 1;
                    }
                    let prepared = &self.prepared[&key];
                    let width = prepared.width + 10.;
                    let height = (prepared.text.height() + 6.).min(clip.height - 12.);
                    let x = (point.x - width * 0.5)
                        .clamp(clip.x + 6., clip.x + clip.width - width - 6.);
                    let y = (point.y - height - 4.)
                        .clamp(clip.y + 6., clip.y + clip.height - height - 6.);
                    let bounds = Uv::new(x, y, width, height);
                    let text = Uv::new(x + 5., y + 3., width - 10., height - 6.);
                    let lines = prepared
                        .text
                        .visible_lines(0., text.height)
                        .last()
                        .map_or(0, |(index, _)| index + 1);
                    self.placed.push(Placed {
                        view: id,
                        key,
                        bounds,
                        clip,
                        text,
                        lines,
                        foreground: style::color(
                            &self.config,
                            local,
                            &message.category,
                            [229, 229, 1, 255],
                        ),
                        background: style::background_enabled(&self.config, local)
                            .then(|| style::background_color(&self.config, local)),
                    });
                }
            }
        }
        self.prepared.retain(|key, _| used.contains(key));
    }
    pub(super) fn paint_view(&self, view: Uuid, list: &mut PaintList) {
        for placed in self.placed.iter().filter(|placed| placed.view == view) {
            list.ops.push(PaintOp::PushClip(placed.clip));
            if let Some(background) = placed.background {
                list.rect(placed.bounds, 0., color(background));
            }
            list.ops.push(PaintOp::PushClip(placed.text));
            for run in self.prepared[&placed.key]
                .text
                .runs_for_lines(0..placed.lines)
            {
                list.ops.push(PaintOp::GlyphRun {
                    run: run.clone(),
                    origin: [placed.text.x, placed.text.y],
                    color: color(placed.foreground),
                });
            }
            list.ops.push(PaintOp::PopClip);
            list.ops.push(PaintOp::PopClip);
        }
    }
}
