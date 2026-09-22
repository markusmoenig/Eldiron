//! The conversation editor for a `Talk` node.
//!
//! A panel over the graph: the steps on the left, the selected step and its
//! choices on the right. Text is edited with the same overlay the node rows use,
//! so typing, selection, wrapping and the clipboard behave exactly the same here
//! as anywhere else in the editor.
//!
//! A choice does not carry a script. It either moves inside the tree (`go`), ends
//! the conversation, or leaves through one of the node's consequence ports, which
//! the panel shows as a small "then" button and a target.

use super::*;
use rusterix::server::nodes::conversation::{CONSEQUENCE_SLOTS, Choice, Conversation, Step, Then};

const ROW: f32 = 26.;
const HEAD: f32 = 40.;
const LEFT_W: f32 = 196.;
const LABEL_W: f32 = 92.;
const TEXT: f32 = 13.;
const SMALL: f32 = 11.;
/// Air between the panel border and the scrolling content.
const EDGE: f32 = 10.;
/// Comfortable minimum size of the text overlay one field edits.
const EDITOR_MIN_H: f32 = 150.;
const EDITOR_MIN_W: f32 = 340.;

/// One editable piece of the conversation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    StepName(usize),
    StepLine(usize),
    ChoiceLabel(usize, usize),
    ChoiceCondition(usize, usize),
}

/// What a click in the panel meant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    /// Outside the panel, so the caller closes the editor.
    Outside,
    /// Selection or a button, already applied.
    Handled,
    /// The caller should start editing this field.
    Edit(Field),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Field(Field),
    Step(usize),
    Choice(usize),
    Entry,
    AddStep,
    RemoveStep,
    AddChoice,
    RemoveChoice,
    /// Cycles a choice between ending, jumping, and leaving through a port.
    Then(usize, usize),
    /// Cycles the jump step or the output slot of a choice.
    ThenTarget(usize, usize),
    /// Cycles the step an `Out` choice continues at when the chain hands back.
    ThenResume(usize, usize),
}

pub struct ConversationEditor {
    pub node: GraphId,
    panel: GraphRect,
    conversation: Conversation,
    step: usize,
    choice: usize,
    changed: bool,
    /// Vertical scroll of the right, fields column.
    scroll: f32,
    /// Vertical scroll of the left, steps column.
    step_scroll: f32,
}

impl ConversationEditor {
    pub fn new(node: GraphId, conversation: Conversation, view: [f32; 2]) -> Self {
        // Big enough to read, but never larger than the space it is shown in.
        let size = [
            (view[0] - 60.)
                .clamp(320., 900.)
                .min((view[0] - 20.).max(200.)),
            (view[1] - 50.)
                .clamp(240., 680.)
                .min((view[1] - 20.).max(160.)),
        ];
        let panel = GraphRect {
            origin: [
                ((view[0] - size[0]) * 0.5).max(0.),
                ((view[1] - size[1]) * 0.5 + 10.).max(0.),
            ],
            size,
        };
        Self {
            node,
            panel,
            conversation,
            step: 0,
            choice: 0,
            changed: false,
            scroll: 0.,
            step_scroll: 0.,
        }
    }

    /// Top of the scrolling content, below the panel header.
    fn content_top(&self) -> f32 {
        self.panel.origin[1] + HEAD
    }

    /// Bottom of the scrolling content, above the panel border.
    fn content_bottom(&self) -> f32 {
        self.panel.origin[1] + self.panel.size[1] - EDGE
    }

    fn left_x(&self) -> f32 {
        self.panel.origin[0] + 14.
    }

    fn right_x(&self) -> f32 {
        self.panel.origin[0] + LEFT_W + 14.
    }

    fn right_w(&self) -> f32 {
        self.panel.size[0] - LEFT_W - 28.
    }

    /// The field column width, from the label gutter to the panel border.
    fn field_w(&self) -> f32 {
        (self.right_w() - LABEL_W).max(60.)
    }

    /// Scroll the column under a view-space point. Positive `dy` reveals what is
    /// below, which is how the wheel reports scrolling down.
    pub fn scroll_at(&mut self, point: [f32; 2], dy: f32) {
        let left = self.left_x();
        if point[0] >= left && point[0] <= left + LEFT_W - 20. {
            self.step_scroll = (self.step_scroll + dy).clamp(0., self.max_step_scroll());
        } else {
            self.scroll = (self.scroll + dy).clamp(0., self.max_scroll());
        }
    }

    fn max_scroll(&self) -> f32 {
        let (_, right, ..) = self.layout();
        let bottom = right
            .last()
            .map(|(rect, _)| rect.origin[1] + rect.size[1])
            .unwrap_or(self.content_top());
        (bottom + EDGE - self.content_bottom()).max(0.)
    }

    fn max_step_scroll(&self) -> f32 {
        let (left, ..) = self.layout();
        let bottom = left
            .last()
            .map(|(rect, _)| rect.origin[1] + rect.size[1])
            .unwrap_or(self.content_top());
        (bottom + EDGE - self.content_bottom()).max(0.)
    }

    /// Editor rectangle for one field: the field's column, made tall enough to
    /// write in, and kept inside the view. A field the panel shows is always in
    /// the laid-out targets, so this does not fall back for a visible field.
    pub fn editor_rect(&self, field: Field, view: [f32; 2]) -> GraphRect {
        let base = self.field_rect(field);
        let width = base.size[0].max(EDITOR_MIN_W).min((view[0] - 40.).max(120.));
        let height = base
            .size[1]
            .max(EDITOR_MIN_H)
            .min((view[1] - 40.).max(80.));
        GraphRect {
            origin: [
                base.origin[0].clamp(20., (view[0] - width - 20.).max(20.)),
                base.origin[1].clamp(20., (view[1] - height - 20.).max(20.)),
            ],
            size: [width, height],
        }
    }

    pub fn conversation(&self) -> &Conversation {
        &self.conversation
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    /// The step the right column shows. Used by tests and diagnostics.
    #[cfg(test)]
    pub fn selected_step(&self) -> usize {
        self.step
    }

    /// Text of one field, for the overlay to edit.
    pub fn text(&self, field: Field) -> Option<String> {
        match field {
            Field::StepName(index) => Some(self.step_at(index)?.name.clone()),
            Field::StepLine(index) => Some(self.step_at(index)?.text.clone()),
            Field::ChoiceLabel(step, index) => {
                Some(self.choice_at(step, index)?.label.clone())
            }
            Field::ChoiceCondition(step, index) => {
                Some(self.choice_at(step, index)?.condition.clone())
            }
        }
    }

    /// Write an edited field back into the conversation.
    pub fn set_text(&mut self, field: Field, text: String) {
        let slot = match field {
            Field::StepName(index) => self
                .conversation
                .steps
                .get_mut(index)
                .map(|step| &mut step.name),
            Field::StepLine(index) => self
                .conversation
                .steps
                .get_mut(index)
                .map(|step| &mut step.text),
            Field::ChoiceLabel(step, index) => self
                .conversation
                .steps
                .get_mut(step)
                .and_then(|step| step.choices.get_mut(index))
                .map(|choice| &mut choice.label),
            Field::ChoiceCondition(step, index) => self
                .conversation
                .steps
                .get_mut(step)
                .and_then(|step| step.choices.get_mut(index))
                .map(|choice| &mut choice.condition),
        };
        if let Some(slot) = slot {
            let text = text.trim().to_string();
            if *slot != text {
                *slot = text;
                self.changed = true;
            }
        }
    }

    fn step_at(&self, index: usize) -> Option<&Step> {
        self.conversation.steps.get(index)
    }

    fn choice_at(&self, step: usize, index: usize) -> Option<&Choice> {
        self.conversation.steps.get(step)?.choices.get(index)
    }

    fn then_at(&self, step: usize, index: usize) -> Option<&Then> {
        self.choice_at(step, index).map(|choice| &choice.then)
    }

    fn set_then(&mut self, step: usize, index: usize, value: Then) {
        if let Some(choice) = self
            .conversation
            .steps
            .get_mut(step)
            .and_then(|step| step.choices.get_mut(index))
            && choice.then != value
        {
            choice.then = value;
            self.changed = true;
        }
    }

    /// A step a choice can jump to: another step when there is one, so the first
    /// click lands somewhere valid instead of on the line it is already on.
    fn default_go_step(&self, step: usize) -> String {
        let current = self
            .conversation
            .steps
            .get(step)
            .map(|step| step.name.clone())
            .unwrap_or_default();
        self.conversation
            .step_names()
            .into_iter()
            .find(|name| name.trim() != current.trim())
            .unwrap_or(current.as_str())
            .to_string()
    }

    /// End -> jump -> output -> end. One button, no scripting.
    fn cycle_then(&mut self, step: usize, index: usize) {
        let Some(then) = self.then_at(step, index).cloned() else {
            return;
        };
        let next = match then {
            Then::End => Then::Go {
                step: self.default_go_step(step),
            },
            Then::Go { .. } => Then::Out {
                slot: 0,
                resume: String::new(),
            },
            Then::Out { .. } => Then::End,
        };
        self.set_then(step, index, next);
    }

    /// Cycle the jump target step or the output slot.
    fn cycle_then_target(&mut self, step: usize, index: usize) {
        let Some(then) = self.then_at(step, index).cloned() else {
            return;
        };
        let next = match then {
            Then::Go { step: target } => {
                let names = self.conversation.step_names();
                let target = target.trim();
                let next = match names.iter().position(|name| name.trim() == target) {
                    Some(position) if !names.is_empty() => {
                        names[(position + 1) % names.len()].to_string()
                    }
                    _ => names.first().copied().unwrap_or_default().to_string(),
                };
                Then::Go { step: next }
            }
            Then::Out { slot, resume } => Then::Out {
                slot: (slot + 1) % CONSEQUENCE_SLOTS,
                resume,
            },
            Then::End => return,
        };
        self.set_then(step, index, next);
    }

    /// Cycle where an `Out` choice continues when the chain hands the flow back:
    /// end, then each step.
    fn cycle_then_resume(&mut self, step: usize, index: usize) {
        let Some(then) = self.then_at(step, index).cloned() else {
            return;
        };
        let Then::Out { slot, resume } = then else {
            return;
        };
        let names = self.conversation.step_names();
        let current = names
            .iter()
            .position(|name| name.trim() == resume.trim())
            .map(|position| position + 1)
            .unwrap_or(0);
        let next = match names.get(current) {
            Some(name) => Then::Out {
                slot,
                resume: name.to_string(),
            },
            None => Then::Out {
                slot,
                resume: String::new(),
            },
        };
        self.set_then(step, index, next);
    }

    fn then_kind_label(&self, step: usize, index: usize) -> String {
        match self.then_at(step, index) {
            Some(Then::Go { .. }) => fl!("node_talk_then_go"),
            Some(Then::Out { .. }) => fl!("node_talk_then_out"),
            _ => fl!("node_talk_then_end"),
        }
    }

    fn then_target_label(&self, step: usize, index: usize) -> String {
        match self.then_at(step, index) {
            Some(Then::Go { step }) if !step.trim().is_empty() => step.clone(),
            Some(Then::Go { .. }) => "—".to_string(),
            Some(Then::Out { slot, .. }) => format!("{} {}", fl!("node_talk_out"), slot + 1),
            _ => String::new(),
        }
    }

    fn then_target_title(&self, step: usize, index: usize) -> String {
        match self.then_at(step, index) {
            Some(Then::Out { .. }) => fl!("node_talk_out"),
            _ => fl!("node_talk_step_target"),
        }
    }

    /// Where an `Out` choice continues when the chain hands the flow back:
    /// a step name, or "End" when the conversation stops there.
    fn then_resume_label(&self, step: usize, index: usize) -> String {
        match self.then_at(step, index) {
            Some(Then::Out { resume, .. }) if !resume.trim().is_empty() => resume.clone(),
            _ => fl!("node_talk_then_end"),
        }
    }

    /// A short outcome for a choice list row, so a long list stays readable.
    fn choice_outcome_label(&self, step: usize, index: usize) -> String {
        match self.then_at(step, index) {
            Some(Then::Go { step }) if !step.trim().is_empty() => format!("→ {}", step.trim()),
            Some(Then::Go { .. }) => "→ ?".to_string(),
            Some(Then::Out { slot, resume }) => {
                let mut shown = format!("→ {} {}", fl!("node_talk_out"), slot + 1);
                if !resume.trim().is_empty() {
                    shown.push_str(&format!(" · {}", resume.trim()));
                }
                shown
            }
            _ => format!("→ {}", fl!("node_talk_then_end")),
        }
    }

    /// Every target, unscrolled: the left steps column and the right fields
    /// column. Positions are top-down from the content top; scrolling and
    /// clipping happen in `targets`.
    fn layout(&self) -> (
        Vec<(GraphRect, Target)>,
        Vec<(GraphRect, Target)>,
    ) {
        let mut left_targets = Vec::new();
        let left = self.left_x();
        let right = self.right_x();
        let right_w = self.right_w();
        let mut y = self.content_top();

        for index in 0..self.conversation.steps.len() {
            left_targets.push((
                GraphRect {
                    origin: [left, y],
                    size: [LEFT_W - 20., ROW],
                },
                Target::Step(index),
            ));
            y += ROW;
        }
        for target in [Target::AddStep, Target::RemoveStep] {
            left_targets.push((
                GraphRect {
                    origin: [left, y],
                    size: [LEFT_W - 20., ROW],
                },
                target,
            ));
            y += ROW;
        }

        let mut targets = Vec::new();
        let mut y = self.content_top();
        let field = |targets: &mut Vec<_>, height: f32, target: Target, y: &mut f32| {
            targets.push((
                GraphRect {
                    origin: [right + LABEL_W, *y],
                    size: [self.field_w(), height],
                },
                target,
            ));
            *y += height + 4.;
        };
        let button = |targets: &mut Vec<_>, target: Target, y: &mut f32| {
            targets.push((
                GraphRect {
                    origin: [right + LABEL_W, *y],
                    size: [self.field_w(), ROW],
                },
                target,
            ));
            *y += ROW + 4.;
        };

        if self.conversation.steps.get(self.step).is_some() {
            field(&mut targets, ROW, Target::Field(Field::StepName(self.step)), &mut y);
            field(
                &mut targets,
                ROW * 2.,
                Target::Field(Field::StepLine(self.step)),
                &mut y,
            );
            targets.push((
                GraphRect {
                    origin: [right + LABEL_W, y],
                    size: [140., ROW],
                },
                Target::Entry,
            ));
            y += ROW + 10.;

            let choices = self
                .conversation
                .steps
                .get(self.step)
                .map(|step| step.choices.len())
                .unwrap_or(0);
            for index in 0..choices {
                targets.push((
                    GraphRect {
                        origin: [right, y],
                        size: [right_w, ROW],
                    },
                    Target::Choice(index),
                ));
                y += ROW;
            }
            let button_w = (right_w - 8.) * 0.5;
            targets.push((
                GraphRect {
                    origin: [right, y],
                    size: [button_w, ROW],
                },
                Target::AddChoice,
            ));
            targets.push((
                GraphRect {
                    origin: [right + button_w + 8., y],
                    size: [button_w, ROW],
                },
                Target::RemoveChoice,
            ));
            y += ROW + 8.;

            if self.choice_at(self.step, self.choice).is_some() {
                field(
                    &mut targets,
                    ROW,
                    Target::Field(Field::ChoiceLabel(self.step, self.choice)),
                    &mut y,
                );
                field(
                    &mut targets,
                    ROW,
                    Target::Field(Field::ChoiceCondition(self.step, self.choice)),
                    &mut y,
                );
                button(&mut targets, Target::Then(self.step, self.choice), &mut y);
                match self.then_at(self.step, self.choice) {
                    Some(Then::Go { .. }) => {
                        button(
                            &mut targets,
                            Target::ThenTarget(self.step, self.choice),
                            &mut y,
                        );
                    }
                    Some(Then::Out { .. }) => {
                        button(
                            &mut targets,
                            Target::ThenTarget(self.step, self.choice),
                            &mut y,
                        );
                        button(
                            &mut targets,
                            Target::ThenResume(self.step, self.choice),
                            &mut y,
                        );
                    }
                    _ => {}
                }
            }
        }
        (left_targets, targets)
    }

    /// The laid-out targets with the two scroll offsets applied, still without
    /// any clipping. Hit testing and `field_rect` use this one.
    fn scrolled(&self) -> (
        Vec<(GraphRect, Target)>,
        Vec<(GraphRect, Target)>,
    ) {
        let (mut left, mut right) = self.layout();
        for (rect, _) in left.iter_mut() {
            rect.origin[1] -= self.step_scroll;
        }
        for (rect, _) in right.iter_mut() {
            rect.origin[1] -= self.scroll;
        }
        (left, right)
    }

    /// The targets a click may actually hit: scrolled, and clipped to the panel
    /// so nothing outside the visible content area is ever hittable. A click
    /// inside the panel that misses all of them is `Handled`, never `Outside`.
    fn targets(&self) -> Vec<(GraphRect, Target)> {
        let (left, right) = self.scrolled();
        let top = self.content_top();
        let bottom = self.content_bottom();
        left.into_iter()
            .chain(right)
            .filter(|(rect, _)| {
                rect.origin[1] >= top - 0.5 && rect.origin[1] + rect.size[1] <= bottom + 0.5
            })
            .collect()
    }

    pub fn pointer_down(&mut self, point: [f32; 2]) -> Click {
        if !self.panel.contains(point) {
            return Click::Outside;
        }
        for (rect, target) in self.targets() {
            if !rect.contains(point) {
                continue;
            }
            match target {
                Target::Step(index) => {
                    self.step = index;
                    self.choice = 0;
                }
                Target::Choice(index) => self.choice = index,
                Target::Entry => {
                    if let Some(step) = self.conversation.steps.get(self.step) {
                        let name = step.name.clone();
                        self.conversation.entry = name;
                        self.changed = true;
                    }
                }
                Target::AddStep => {
                    self.conversation.steps.push(Step {
                        name: format!("step{}", self.conversation.steps.len() + 1),
                        text: String::new(),
                        choices: Vec::new(),
                    });
                    self.step = self.conversation.steps.len() - 1;
                    self.choice = 0;
                    self.changed = true;
                }
                Target::RemoveStep => {
                    if self.conversation.steps.len() > 1 {
                        self.conversation.steps.remove(self.step);
                        self.step = self.step.min(self.conversation.steps.len() - 1);
                        self.choice = 0;
                        self.changed = true;
                    }
                }
                Target::AddChoice => {
                    if let Some(step) = self.conversation.steps.get_mut(self.step) {
                        step.choices.push(Choice {
                            label: "Leave".into(),
                            condition: String::new(),
                            then: Then::End,
                        });
                        self.choice = step.choices.len() - 1;
                        self.changed = true;
                    }
                }
                Target::RemoveChoice => {
                    if let Some(step) = self.conversation.steps.get_mut(self.step)
                        && self.choice < step.choices.len()
                    {
                        step.choices.remove(self.choice);
                        self.choice = self.choice.saturating_sub(1);
                        self.changed = true;
                    }
                }
                Target::Then(step, index) => self.cycle_then(step, index),
                Target::ThenTarget(step, index) => self.cycle_then_target(step, index),
                Target::ThenResume(step, index) => self.cycle_then_resume(step, index),
                Target::Field(field) => return Click::Edit(field),
            }
            // A selection or an add/remove changes the content height, so the
            // two scroll offsets have to be pulled back into range.
            self.scroll = self.scroll.min(self.max_scroll());
            self.step_scroll = self.step_scroll.min(self.max_step_scroll());
            return Click::Handled;
        }
        Click::Handled
    }

    /// The region the text overlay should cover while a field is edited. This
    /// follows the scroll, so an editor open on a field keeps tracking it.
    pub fn field_rect(&self, field: Field) -> GraphRect {
        let (_, right) = self.scrolled();
        for (rect, target) in right {
            if target == Target::Field(field) {
                return GraphRect {
                    origin: [rect.origin[0], rect.origin[1] - 4.],
                    size: [rect.size[0], rect.size[1] + 8.],
                };
            }
        }
        GraphRect {
            origin: [self.right_x() + LABEL_W, self.content_top()],
            size: [self.field_w(), ROW * 2.],
        }
    }

    /// The scrolled rectangle of a choice's `then` button, for tests.
    #[cfg(test)]
    pub fn then_rect(&self, step: usize, index: usize) -> Option<GraphRect> {
        self.scrolled()
            .1
            .into_iter()
            .find(|(_, target)| *target == Target::Then(step, index))
            .map(|(rect, _)| rect)
    }

    /// The scrolled rectangle of a choice's "continue at" button, for tests.
    #[cfg(test)]
    pub fn then_resume_rect(&self, step: usize, index: usize) -> Option<GraphRect> {
        self.scrolled()
            .1
            .into_iter()
            .find(|(_, target)| *target == Target::ThenResume(step, index))
            .map(|(rect, _)| rect)
    }

    pub fn paint(&mut self, painter: &mut dyn GraphPainter, theme: &GraphTheme) {
        painter.round_rect(self.panel, 12., theme.background);
        painter.round_rect(
            GraphRect {
                origin: [self.panel.origin[0] + 1., self.panel.origin[1] + 1.],
                size: [self.panel.size[0] - 2., self.panel.size[1] - 2.],
            },
            11.,
            theme.body,
        );
        painter.text(
            GraphRect {
                origin: [self.panel.origin[0] + 16., self.panel.origin[1] + 10.],
                size: [self.panel.size[0] - 32., 22.],
            },
            &fl!("node_talk_editor"),
            TEXT + 3.,
            theme.text,
        );
        painter.text(
            GraphRect {
                origin: [self.panel.origin[0] + LEFT_W + 14., self.panel.origin[1] + 14.],
                size: [self.panel.size[0] - LEFT_W - 30., 18.],
            },
            &fl!("node_talk_editor_hint"),
            SMALL,
            theme.muted,
        );

        for (rect, target) in self.targets() {
            match target {
                Target::Step(index) => {
                    let selected = index == self.step;
                    if selected {
                        painter.round_rect(rect, 5., theme.control);
                    }
                    let name = self
                        .conversation
                        .steps
                        .get(index)
                        .map(|step| step.name.clone())
                        .unwrap_or_default();
                    let entry = self.conversation.entry.trim() == name.trim();
                    let label = if entry {
                        format!("• {name}")
                    } else {
                        name
                    };
                    painter.text(
                        GraphRect {
                            origin: [rect.origin[0] + 8., rect.origin[1] + 4.],
                            size: [rect.size[0] - 12., rect.size[1]],
                        },
                        &label,
                        TEXT,
                        if selected { theme.active } else { theme.text },
                    );
                }
                Target::AddStep | Target::RemoveStep => {
                    let label = if target == Target::AddStep {
                        fl!("node_talk_add_step")
                    } else {
                        fl!("node_talk_remove_step")
                    };
                    painter.text(
                        GraphRect {
                            origin: [rect.origin[0] + 8., rect.origin[1] + 4.],
                            size: [rect.size[0] - 12., rect.size[1]],
                        },
                        &label,
                        TEXT,
                        theme.muted,
                    );
                }
                Target::Field(field) => {
                    painter.round_rect(rect, 5., theme.control);
                    painter.text(
                        GraphRect {
                            origin: [rect.origin[0] + 8., rect.origin[1] + 4.],
                            size: [rect.size[0] - 16., rect.size[1]],
                        },
                        &self.field_label(field),
                        TEXT,
                        theme.text,
                    );
                    // The label sits to the left of the box.
                    painter.text(
                        GraphRect {
                            origin: [rect.origin[0] - LABEL_W + 6., rect.origin[1] + 4.],
                            size: [LABEL_W - 10., rect.size[1]],
                        },
                        &self.field_title(field),
                        SMALL,
                        theme.muted,
                    );
                }
                Target::Entry => {
                    painter.round_rect(rect, 5., theme.control);
                    let label = if self
                        .conversation
                        .steps
                        .get(self.step)
                        .is_some_and(|step| step.name.trim() == self.conversation.entry.trim())
                    {
                        fl!("node_talk_entry")
                    } else {
                        fl!("node_talk_set_entry")
                    };
                    painter.text(
                        GraphRect {
                            origin: [rect.origin[0] + 8., rect.origin[1] + 4.],
                            size: [rect.size[0] - 12., rect.size[1]],
                        },
                        &label,
                        SMALL,
                        theme.muted,
                    );
                }
                Target::Choice(index) => {
                    let selected = index == self.choice;
                    if selected {
                        painter.round_rect(rect, 5., theme.control);
                    }
                    let label = self
                        .choice_at(self.step, index)
                        .map(|choice| choice.label.clone())
                        .unwrap_or_default();
                    let condition = self
                        .choice_at(self.step, index)
                        .map(|choice| choice.condition.clone())
                        .unwrap_or_default();
                    let outcome = self.choice_outcome_label(self.step, index);
                    let mut shown = label;
                    if !condition.is_empty() {
                        shown = if shown.is_empty() {
                            format!("({condition})")
                        } else {
                            format!("{shown}   ({condition})")
                        };
                    }
                    let shown = format!("{shown}   {outcome}");
                    painter.text(
                        GraphRect {
                            origin: [rect.origin[0] + 8., rect.origin[1] + 4.],
                            size: [rect.size[0] - 12., rect.size[1]],
                        },
                        &shown,
                        TEXT,
                        if selected { theme.active } else { theme.text },
                    );
                }
                Target::AddChoice | Target::RemoveChoice => {
                    let label = if target == Target::AddChoice {
                        fl!("node_talk_add_choice")
                    } else {
                        fl!("node_talk_remove_choice")
                    };
                    painter.text(
                        GraphRect {
                            origin: [rect.origin[0] + 8., rect.origin[1] + 4.],
                            size: [rect.size[0] - 12., rect.size[1]],
                        },
                        &label,
                        SMALL,
                        theme.muted,
                    );
                }
                Target::Then(step, index) => {
                    painter.round_rect(rect, 5., theme.control);
                    painter.text(
                        GraphRect {
                            origin: [rect.origin[0] + 8., rect.origin[1] + 4.],
                            size: [rect.size[0] - 16., rect.size[1]],
                        },
                        &self.then_kind_label(step, index),
                        TEXT,
                        theme.text,
                    );
                    painter.text(
                        GraphRect {
                            origin: [rect.origin[0] - LABEL_W + 6., rect.origin[1] + 4.],
                            size: [LABEL_W - 10., rect.size[1]],
                        },
                        &fl!("node_talk_then"),
                        SMALL,
                        theme.muted,
                    );
                }
                Target::ThenTarget(step, index) => {
                    painter.round_rect(rect, 5., theme.control);
                    painter.text(
                        GraphRect {
                            origin: [rect.origin[0] + 8., rect.origin[1] + 4.],
                            size: [rect.size[0] - 16., rect.size[1]],
                        },
                        &self.then_target_label(step, index),
                        TEXT,
                        theme.active,
                    );
                    painter.text(
                        GraphRect {
                            origin: [rect.origin[0] - LABEL_W + 6., rect.origin[1] + 4.],
                            size: [LABEL_W - 10., rect.size[1]],
                        },
                        &self.then_target_title(step, index),
                        SMALL,
                        theme.muted,
                    );
                }
                Target::ThenResume(step, index) => {
                    painter.round_rect(rect, 5., theme.control);
                    painter.text(
                        GraphRect {
                            origin: [rect.origin[0] + 8., rect.origin[1] + 4.],
                            size: [rect.size[0] - 16., rect.size[1]],
                        },
                        &self.then_resume_label(step, index),
                        TEXT,
                        theme.active,
                    );
                    painter.text(
                        GraphRect {
                            origin: [rect.origin[0] - LABEL_W + 6., rect.origin[1] + 4.],
                            size: [LABEL_W - 10., rect.size[1]],
                        },
                        &fl!("node_talk_then_resume"),
                        SMALL,
                        theme.muted,
                    );
                }
            }
        }
        self.paint_scrollbars(painter, theme);
    }

    /// A slim bar on each column when its content does not all fit, so a field
    /// that is scrolled out of view is visibly reachable with the wheel.
    fn paint_scrollbars(&self, painter: &mut dyn GraphPainter, theme: &GraphTheme) {
        let top = self.content_top();
        let bottom = self.content_bottom();
        let track = bottom - top;
        for (x, scroll, max) in [
            (
                self.left_x() + LEFT_W - 20. + 4.,
                self.step_scroll,
                self.max_step_scroll(),
            ),
            (
                self.panel.origin[0] + self.panel.size[0] - 12.,
                self.scroll,
                self.max_scroll(),
            ),
        ] {
            if max <= 0. {
                continue;
            }
            let thumb = (track * track / (track + max)).max(24.);
            let y = top + (track - thumb) * (scroll / max);
            painter.round_rect(
                GraphRect {
                    origin: [x, top],
                    size: [3., track],
                },
                1.5,
                theme.control,
            );
            painter.round_rect(
                GraphRect {
                    origin: [x, y],
                    size: [3., thumb],
                },
                1.5,
                theme.muted,
            );
        }
    }

    fn field_title(&self, field: Field) -> String {
        match field {
            Field::StepName(_) => fl!("node_talk_step"),
            Field::StepLine(_) => fl!("node_talk_line"),
            Field::ChoiceLabel(_, _) => fl!("node_talk_label"),
            Field::ChoiceCondition(_, _) => fl!("node_talk_condition"),
        }
    }

    fn field_label(&self, field: Field) -> String {
        let text = self.text(field).unwrap_or_default();
        let first = text.lines().next().unwrap_or_default();
        if text.lines().count() > 1 {
            format!("{first} …")
        } else {
            first.to_string()
        }
    }
}
