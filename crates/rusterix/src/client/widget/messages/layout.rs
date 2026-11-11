//! Renderer-independent message flow measurements and view state.
use super::*;

/// Actual shaped block heights, including portrait reservation. No glyph/font/GPU
/// objects enter the controller; the presentation owner retains their layouts.
pub struct MessageMetrics {
    pub blocks: FxHashMap<Uuid, f32>,
    pub line_height: f32,
    pub block_gap: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct MessageViewState {
    pub scroll_offset: usize,
    pub choices_below_view: bool,
    pub paused: bool,
    pub timed_pause: bool,
    pub command_input_enabled: bool,
    pub line_height: f32,
    pub block_gap: f32,
}
impl MessagesWidget {
    pub fn layout_lines(&self) -> impl Iterator<Item = &MessageLine> {
        self.messages
            .iter()
            .chain(self.pending_messages.iter().flat_map(|pending| {
                match pending {
                    PendingMessage::Line(line) => std::slice::from_ref(line),
                    PendingMessage::ChoiceGroup(lines) => lines.as_slice(),
                    _ => &[],
                }
                .iter()
            }))
    }
    fn use_native_metrics(&mut self, metrics: MessageMetrics) -> Result<(), String> {
        if !metrics.line_height.is_finite()
            || metrics.line_height <= 0.
            || !metrics.block_gap.is_finite()
            || metrics.block_gap < 0.
        {
            return Err("invalid message line height".into());
        }
        for line in self.layout_lines() {
            if !metrics
                .blocks
                .get(&line.id)
                .is_some_and(|h| h.is_finite() && *h > 0.)
            {
                return Err(format!(
                    "missing/invalid measurement for message {}",
                    line.id
                ));
            }
        }
        self.native_metrics = Some(metrics);
        Ok(())
    }
    pub fn reveal_measured(
        &mut self,
        metrics: MessageMetrics,
    ) -> Result<Option<FxHashMap<char, Choice>>, String> {
        self.use_native_metrics(metrics)?;
        self.font = None;
        Ok(self.finish_messages())
    }
    pub(super) fn finish_messages(&mut self) -> Option<FxHashMap<char, Choice>> {
        self.reveal_pending_messages();
        self.purge_old_messages();
        let choices = self.active_choice_map();
        (!choices.is_empty()).then_some(choices)
    }
    pub fn view_state(&self) -> MessageViewState {
        MessageViewState {
            scroll_offset: self.scroll_offset,
            choices_below_view: self.choices_are_below_view(),
            paused: self.paused,
            timed_pause: self.pause_until.is_some(),
            command_input_enabled: self.command_input_enabled(),
            line_height: self
                .native_metrics
                .as_ref()
                .map_or(self.font_size, |m| m.line_height),
            block_gap: self
                .native_metrics
                .as_ref()
                .map_or(self.message_spacing.max(0.), |m| m.block_gap),
        }
    }
    pub fn set_return_to_choices_rect(&mut self, rect: Rect) {
        self.return_to_choices_rect = rect;
    }
    pub(super) fn measured_page_is_full(&self, reserve: bool, metrics: &MessageMetrics) -> bool {
        let prompt = if reserve {
            metrics.line_height + metrics.block_gap
        } else {
            0.
        };
        let available = (self.rect.height - prompt).max(metrics.line_height);
        let mut height = 0.;
        for line in self.messages.iter().skip(self.page_start_index) {
            if height > 0. {
                height += metrics.block_gap;
            }
            height += metrics
                .blocks
                .get(&line.id)
                .copied()
                .unwrap_or(f32::INFINITY);
        }
        height > available
    }
    pub(super) fn measured_choice_scroll_offset(
        &self,
        lines: &[MessageLine],
        metrics: &MessageMetrics,
    ) -> usize {
        let prompt = metrics.line_height + metrics.block_gap;
        let budget = (self.rect.height * 0.55 - prompt)
            .max(metrics.line_height)
            .min(self.rect.height);
        let mut height = 0.;
        let mut visible = 0;
        for line in lines {
            let next = metrics
                .blocks
                .get(&line.id)
                .copied()
                .unwrap_or(f32::INFINITY)
                + if visible == 0 { 0. } else { metrics.block_gap };
            if visible > 0 && height + next > budget {
                break;
            }
            height += next;
            visible += 1;
        }
        lines.len().saturating_sub(visible.max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn measured_flow_reveals_and_continues_without_a_cpu_font_or_buffer() {
        let mut widget = MessagesWidget::new();
        widget.rect = Rect::new(10., 10., 200., 90.);
        widget.press_to_continue = true;
        let assets = Assets::default();
        let map = Map::default();
        let time = TheTime::new();
        let messages =
            ["first", "second", "third"].map(|s| (None, None, 0, s.into(), "message".into()));
        widget.queue_messages(&assets, &map, &time, &messages, &[]);
        assert!(widget.messages.is_empty());
        assert_eq!(widget.layout_lines().count(), 3);
        assert!(
            widget
                .reveal_measured(MessageMetrics {
                    blocks: FxHashMap::default(),
                    line_height: 20.,
                    block_gap: 8.,
                })
                .is_err()
        );
        assert_eq!(
            widget.layout_lines().count(),
            3,
            "invalid measurements cannot consume output"
        );
        let blocks = widget.layout_lines().map(|l| (l.id, 30.)).collect();
        assert!(
            widget
                .reveal_measured(MessageMetrics {
                    blocks,
                    line_height: 20.,
                    block_gap: 8.,
                })
                .unwrap()
                .is_none()
        );
        assert_eq!(widget.messages.len(), 2);
        assert!(widget.view_state().paused);
        assert!(widget.font.is_none() && widget.buffer.pixels().is_empty());
        assert!(matches!(
            widget.touch_down(Vec2::new(20, 20)),
            Some(EntityAction::Off)
        ));
        assert_eq!(widget.messages.len(), 3);
        assert!(!widget.view_state().paused);
        assert!(
            widget.touch_down(Vec2::new(0, 0)).is_none(),
            "inactive empty controls cannot consume origin clicks"
        );
    }
    #[test]
    fn unpainted_or_clipped_choices_are_not_clickable() {
        let mut widget = MessagesWidget::new();
        widget.rect = Rect::new(10., 10., 200., 70.);
        let choices = (0..2)
            .map(|i| MessageLine {
                id: Uuid::new_v4(),
                text: format!("choice {i}"),
                rect: Rect::default(),
                choice: Some(Choice::ScriptChoice(
                    format!("choice {i}"),
                    "choice".into(),
                    1,
                    2,
                    i,
                    i64::MAX,
                    10.,
                )),
                choice_key: Some(char::from(b'1' + i as u8)),
                color: [255; 4],
                sender_entity: None,
            })
            .collect::<Vec<_>>();
        widget
            .pending_messages
            .push_back(PendingMessage::ChoiceGroup(choices));
        let blocks = widget.layout_lines().map(|l| (l.id, 50.)).collect();
        let map = widget
            .reveal_measured(MessageMetrics {
                blocks,
                line_height: 20.,
                block_gap: 8.,
            })
            .unwrap()
            .unwrap();
        assert_eq!(map.len(), 2);
        assert!(widget.view_state().choices_below_view);
        assert!(widget.touch_down(Vec2::new(0, 0)).is_none());
        assert!(
            widget.touch_down(Vec2::new(20, 20)).is_none(),
            "choices need visible layout rectangles"
        );
        widget.messages[0].rect = Rect::new(10., 10., 200., 30.);
        assert!(matches!(
            widget.touch_down(Vec2::new(20, 20)),
            Some(EntityAction::Choice(_))
        ));
        assert!(
            widget.touch_down(Vec2::new(20, 20)).is_none(),
            "a choice is delivered only once"
        );
    }
}
