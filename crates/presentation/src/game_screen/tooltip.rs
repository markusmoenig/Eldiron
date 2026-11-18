//! Retained, measured tooltip text with a host-driven clock and bounded storage.
use super::*;
use rusterix::client::rules_ui::{CommandState, RulesDescription};
use std::time::Duration;
use theframework::text::TextLayout;
#[derive(Clone)]
pub(super) struct Content {
    pub key: (world_pointer::Binding, String),
    pub description: RulesDescription,
    pub state: Option<CommandState>,
    pub anchor: Rect,
    pub below: bool,
}
struct Block {
    text: String,
    layout: Arc<TextLayout>,
    size: f32,
    y: f32,
    color: [u8; 4],
}
#[derive(Default)]
pub(super) struct Tooltips {
    now: Duration,
    since: Duration,
    delay: Duration,
    content: Option<Content>,
    blocks: Vec<Block>,
    bounds: Option<Rect>,
    font: Option<GameFontVersion>,
    family: String,
    width: f32,
    visible: bool,
    builds: u64,
    pending_redraw: bool,
}
impl Tooltips {
    pub fn clear(&mut self) {
        self.content = None;
        self.bounds = None;
        self.visible = false;
        self.pending_redraw = false;
    }
    pub fn advance(&mut self, elapsed: Duration) -> bool {
        let was_due = self.content.is_some() && self.now.saturating_sub(self.since) >= self.delay;
        self.now = self.now.saturating_add(elapsed);
        std::mem::take(&mut self.pending_redraw)
            || (self.content.is_some()
                && !was_due
                && self.now.saturating_sub(self.since) >= self.delay)
    }
    pub fn wake_after(&self) -> Option<Duration> {
        if self.pending_redraw {
            return Some(Duration::ZERO);
        }
        self.content
            .as_ref()
            .filter(|_| !self.visible)
            .map(|_| {
                self.delay
                    .saturating_sub(self.now.saturating_sub(self.since))
            })
            .filter(|remaining| !remaining.is_zero())
    }
    pub fn bounds(&self) -> Option<Rect> {
        self.bounds.filter(|_| self.visible)
    }
    /// Observe only a current acknowledged source. This arms deadlines on the
    /// first presentation receipt, without needing another visual frame/font job.
    pub fn observe(&mut self, content: Option<Content>, preferences: screen_model::UiPreferences) {
        let Some(content) = content.filter(|_| preferences.tooltips_enabled) else {
            self.clear();
            return;
        };
        let delay = Duration::from_millis(preferences.tooltip_delay.milliseconds());
        let changed = self
            .content
            .as_ref()
            .is_none_or(|old| old.key != content.key || old.anchor != content.anchor)
            || self.delay != delay;
        if changed {
            self.since = self.now;
            self.bounds = None;
        }
        self.delay = delay;
        self.visible = self.now.saturating_sub(self.since) >= delay;
        self.pending_redraw |= changed && self.visible;
        self.content = Some(content);
    }
    pub fn prepare(
        &mut self,
        content: Option<Content>,
        preferences: screen_model::UiPreferences,
        fonts: &mut GameFonts,
        family: &str,
        viewport: (i32, i32),
    ) {
        self.observe(
            content.filter(|_| viewport.0 > 4 && viewport.1 > 4),
            preferences,
        );
        let Some(content) = self.content.clone().filter(|_| self.visible) else {
            self.bounds = None;
            return;
        };
        // This paint already satisfies an immediate tooltip redraw request.
        self.pending_redraw = false;
        let family = if family.trim().is_empty() {
            "Roboto"
        } else {
            family
        };
        let width = 244f32.min((viewport.0 - 18).max(1) as f32);
        let compatible =
            self.font == Some(fonts.version()) && self.family == family && self.width == width;
        let old = std::mem::take(&mut self.blocks);
        let mut rows = vec![(
            content.description.title.clone(),
            15.,
            [222, 214, 190, 255],
            0f32,
        )];
        if let Some(subtitle) = content
            .description
            .subtitle
            .filter(|s| !s.trim().is_empty())
        {
            rows.push((subtitle, 12., [156, 149, 130, 255], 2.));
        }
        for line in content.description.lines.into_iter().take(7) {
            rows.push((line, 13., [222, 214, 190, 255], 4.));
        }
        if let Some(reason) = content
            .state
            .filter(|state| !state.enabled)
            .and_then(|state| state.disabled_reason)
        {
            rows.push((reason, 12., [230, 160, 120, 255], 6.));
        }
        let mut y = 0.;
        let mut measured = 0f32;
        for (text, size, color, gap) in rows {
            y += gap;
            let layout = old
                .iter()
                .find(|block| compatible && block.text == text && block.size == size)
                .map(|block| block.layout.clone())
                .unwrap_or_else(|| {
                    self.builds += 1;
                    Arc::new(fonts.text.layout(&text, family, size, Some(width)))
                });
            measured = measured.max(layout.width());
            self.blocks.push(Block {
                text,
                layout: layout.clone(),
                size,
                y,
                color,
            });
            y += layout.height() + 3.;
        }
        self.font = Some(fonts.version());
        self.family = family.into();
        self.width = width;
        self.bounds = Some(screen_model::tooltip_bounds(
            content.anchor,
            (measured + 14.).min(viewport.0 as f32 - 4.),
            (y + 14.).min(viewport.1 as f32 - 4.),
            scenevm::prelude::Vec2::new(viewport.0, viewport.1),
            content.below,
        ));
    }
    pub fn paint(&self, p: &mut ScreenPainter<'_>) {
        let Some(bounds) = self.bounds() else {
            return;
        };
        p.fill(bounds, [8, 10, 10, 246]);
        p.border(bounds, 1., [104, 88, 55, 255]);
        let inner = bounds.with_border(7.);
        p.list.ops.push(PaintOp::PushClip(rect(inner)));
        for block in &self.blocks {
            if inner.y + block.y >= inner.y + inner.height {
                continue;
            }
            for run in &block.layout.runs {
                p.list.ops.push(PaintOp::GlyphRun {
                    run: run.clone(),
                    origin: [inner.x, inner.y + block.y],
                    color: color(block.color),
                });
            }
        }
        p.list.ops.push(PaintOp::PopClip);
    }
}
impl GameScreenController {
    pub fn tooltip_bounds(&self) -> Option<Rect> {
        self.tooltips.bounds()
    }
    pub fn tooltip_layout_builds(&self) -> u64 {
        self.tooltips.builds
    }
    pub fn advance_presentation(&mut self, elapsed: Duration) -> bool {
        self.tooltips.advance(elapsed)
    }
    pub fn presentation_wake_after(&self) -> Option<Duration> {
        self.tooltips.wake_after()
    }
    pub fn set_tooltip_shortcuts(&mut self, shortcuts: FxHashMap<String, Vec<String>>) {
        self.tooltip_shortcuts = shortcuts;
    }
    pub fn tooltip_commands(&self, world: &RuntimeView<'_>) -> Vec<String> {
        screen_model::ordered_button_ids(&self.elements.button_widgets)
            .into_iter()
            .filter_map(|id| {
                let w = &self.elements.button_widgets[&id];
                let map = world.map.unwrap_or(&self.empty_map);
                screen_model::resolved_widget_command(
                    w,
                    world.assets,
                    world.party_entity(map, w.party.as_deref()),
                    &self.elements.ui_state,
                )
            })
            .collect()
    }
}
