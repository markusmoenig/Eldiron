use crate::editor::RUSTERIX;
use crate::prelude::*;

const HELP_FEEDBACK_ACTION: &str = "Help Feedback Action";
const HELP_BACK: &str = "Help Back";
const HELP_FORWARD: &str = "Help Forward";
const HELP_OUTPUT: &str = "Help Output";
const HELP_INPUT: &str = "Help Input";
const HELP_CONTENT: &str = "Help Content";
const HELP_ITEM_GALLERY: &str = "Help Item Gallery";

pub struct HelpDock {
    document: TheFeedbackDocument,
    rules_source: String,
    visible_item_ids: Vec<String>,
    history: Vec<Option<String>>,
    history_index: usize,
}

impl HelpDock {
    fn input_id(ui: &mut TheUI) -> Option<TheId> {
        ui.get_widget(HELP_INPUT).map(|widget| widget.id().clone())
    }

    fn effective_rules(project: &Project) -> Result<String, String> {
        shared::rulesets::resolve_project_rules(&project.config, &project.rules)
    }

    fn clear_input(ui: &mut TheUI) {
        if let Some(widget) = ui.get_widget(HELP_INPUT)
            && let Some(edit) = widget.as_text_line_edit()
        {
            edit.set_text(String::new());
        }
    }

    fn focus_input(ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(id) = Self::input_id(ui) {
            ctx.ui.focus = Some(id.clone());
            ctx.ui.keyboard_focus = Some(id.clone());
            ctx.ui.send(TheEvent::GainedFocus(id));
            ui.process_events(ctx);
        }
    }

    fn sync_output(&self, ui: &mut TheUI) {
        if let Some(output) = ui.get_text_view(HELP_OUTPUT) {
            output.set_blocks(
                self.document
                    .to_text_view_blocks(&TheFeedbackPalette::default()),
            );
            output.scroll_to_top();
        }
    }

    fn command_target(command: &str) -> String {
        let Some(head) = command.split_whitespace().next() else {
            return command.to_string();
        };
        if command.contains('<') {
            format!("help {head}")
        } else if command.contains('[') {
            head.to_string()
        } else {
            command.to_string()
        }
    }

    fn value_span(value: impl Into<String>) -> TheFeedbackSpan {
        let value = value.into();
        let trimmed = value.trim();
        let role = if trimmed.eq_ignore_ascii_case("true") || trimmed.eq_ignore_ascii_case("false")
        {
            TheFeedbackRole::BoolValue
        } else if trimmed.parse::<f64>().is_ok()
            || trimmed.ends_with('%')
            || trimmed.ends_with(" XP")
        {
            TheFeedbackRole::NumberValue
        } else if trimmed.starts_with('`') && trimmed.ends_with('`') {
            TheFeedbackRole::Code
        } else {
            TheFeedbackRole::StringValue
        };
        TheFeedbackSpan::new(value, role)
    }

    fn inline_spans(text: &str, role: TheFeedbackRole) -> Vec<TheFeedbackSpan> {
        let mut spans = Vec::new();
        let mut remainder = text;
        while let Some(start) = remainder.find('`') {
            if start > 0 {
                spans.push(TheFeedbackSpan::new(&remainder[..start], role));
            }
            let after_start = &remainder[start + 1..];
            let Some(end) = after_start.find('`') else {
                spans.push(TheFeedbackSpan::new(&remainder[start..], role));
                return spans;
            };
            spans.push(TheFeedbackSpan::new(
                &after_start[..end],
                TheFeedbackRole::Code,
            ));
            remainder = &after_start[end + 1..];
        }
        if !remainder.is_empty() {
            spans.push(TheFeedbackSpan::new(remainder, role));
        }
        spans
    }

    fn value_spans(value: &str) -> Vec<TheFeedbackSpan> {
        if value.contains('`') {
            Self::inline_spans(value, TheFeedbackRole::StringValue)
        } else {
            vec![Self::value_span(value)]
        }
    }

    fn key_value(line: &str) -> Option<(&str, &str, &'static str)> {
        if let Some((key, value)) = line.split_once(": ") {
            Some((key.trim(), value.trim(), ": "))
        } else if let Some((key, value)) = line.split_once(" = ") {
            Some((key.trim(), value.trim(), " = "))
        } else {
            None
        }
    }

    fn split_aligned_columns(line: &str) -> Option<(&str, &str)> {
        let bytes = line.as_bytes();
        let mut start = 0;
        while start < bytes.len() {
            if bytes[start] != b' ' {
                start += 1;
                continue;
            }
            let mut end = start;
            while end < bytes.len() && bytes[end] == b' ' {
                end += 1;
            }
            if end - start >= 2 {
                let left = line[..start].trim();
                let right = line[end..].trim();
                if !left.is_empty() && !right.is_empty() {
                    return Some((left, right));
                }
            }
            start = end;
        }
        None
    }

    fn identity_spans(text: &str) -> Vec<TheFeedbackSpan> {
        if let Some(open) = text.rfind(" (")
            && text.ends_with(')')
        {
            return vec![
                TheFeedbackSpan::body(&text[..open]),
                TheFeedbackSpan::new(&text[open..], TheFeedbackRole::StableId),
            ];
        }
        Self::inline_spans(text, TheFeedbackRole::Body)
    }

    fn list_item_spans(line: &str, nested: bool) -> Vec<TheFeedbackSpan> {
        let mut spans = Vec::new();
        if nested {
            spans.push(TheFeedbackSpan::new("↳ ", TheFeedbackRole::Muted));
        }
        if let Some((key, value, separator)) = Self::key_value(line) {
            spans.push(TheFeedbackSpan::new(key, TheFeedbackRole::Key));
            spans.push(TheFeedbackSpan::new(separator, TheFeedbackRole::Muted));
            spans.extend(Self::value_spans(value));
        } else if let Some((identity, detail)) = line.split_once(" — ") {
            spans.extend(Self::identity_spans(identity));
            spans.push(TheFeedbackSpan::new("  —  ", TheFeedbackRole::Muted));
            spans.extend(Self::inline_spans(detail, TheFeedbackRole::StringValue));
        } else if let Some((key, value)) = Self::split_aligned_columns(line) {
            spans.push(TheFeedbackSpan::new(key, TheFeedbackRole::Key));
            spans.push(TheFeedbackSpan::new("  ", TheFeedbackRole::Muted));
            spans.extend(Self::inline_spans(value, TheFeedbackRole::Body));
        } else if line.starts_with("Level ") {
            spans.push(TheFeedbackSpan::new(line, TheFeedbackRole::NumberValue));
        } else {
            spans.extend(Self::identity_spans(line));
        }
        spans
    }

    fn section_heading(text: &str) -> TheFeedbackBlock {
        TheFeedbackBlock::Heading {
            level: 2,
            spans: vec![TheFeedbackSpan::new(
                text.trim_end_matches(':').trim(),
                TheFeedbackRole::Heading,
            )],
        }
    }

    fn is_section_heading(lines: &[&str], index: usize) -> bool {
        let line = lines[index];
        if line.len() != line.trim_start().len() || Self::key_value(line).is_some() {
            return false;
        }
        if line.trim_end().ends_with(':') {
            return true;
        }
        let Some(next) = lines.get(index + 1) else {
            return false;
        };
        next.len() != next.trim_start().len()
            || (index > 0 && lines[index - 1].trim().is_empty() && Self::key_value(next).is_some())
    }

    fn append_formatted_lines(document: &mut TheFeedbackDocument, lines: &[&str]) {
        let mut index = 0;
        while index < lines.len() {
            if lines[index].trim().is_empty() {
                index += 1;
                continue;
            }

            if Self::is_section_heading(lines, index) {
                document.push(Self::section_heading(lines[index]));
                index += 1;
                continue;
            }

            let indentation = lines[index].len() - lines[index].trim_start().len();
            if indentation > 0 {
                let start = index;
                while index < lines.len()
                    && !lines[index].trim().is_empty()
                    && lines[index].len() != lines[index].trim_start().len()
                {
                    index += 1;
                }
                let base_indent = lines[start..index]
                    .iter()
                    .map(|line| line.len() - line.trim_start().len())
                    .min()
                    .unwrap_or(indentation);
                document.push(TheFeedbackBlock::List {
                    title: None,
                    items: lines[start..index]
                        .iter()
                        .map(|line| {
                            let indent = line.len() - line.trim_start().len();
                            Self::list_item_spans(line.trim(), indent > base_indent)
                        })
                        .collect(),
                });
                continue;
            }

            if Self::key_value(lines[index]).is_some() {
                let start = index;
                while index < lines.len()
                    && !lines[index].trim().is_empty()
                    && lines[index].len() == lines[index].trim_start().len()
                    && Self::key_value(lines[index]).is_some()
                {
                    index += 1;
                }
                document.push(TheFeedbackBlock::KeyValueList {
                    title: None,
                    entries: lines[start..index]
                        .iter()
                        .filter_map(|line| Self::key_value(line))
                        .map(|(key, value, _)| {
                            TheFeedbackKeyValue::new(key, Self::value_spans(value))
                        })
                        .collect(),
                });
                continue;
            }

            let start = index;
            index += 1;
            while index < lines.len()
                && !lines[index].trim().is_empty()
                && lines[index].len() == lines[index].trim_start().len()
                && Self::key_value(lines[index]).is_none()
                && !Self::is_section_heading(lines, index)
            {
                index += 1;
            }
            let paragraph = lines[start..index]
                .iter()
                .map(|line| line.trim())
                .collect::<Vec<_>>()
                .join(" ");
            document.push(TheFeedbackBlock::Paragraph(Self::inline_spans(
                &paragraph,
                TheFeedbackRole::Body,
            )));
        }
    }

    fn append_response_output(
        document: &mut TheFeedbackDocument,
        output: &str,
        command: Option<&str>,
    ) {
        let lines = output.lines().collect::<Vec<_>>();
        let Some(start) = lines.iter().position(|line| !line.trim().is_empty()) else {
            return;
        };
        let end = lines
            .iter()
            .rposition(|line| !line.trim().is_empty())
            .map(|index| index + 1)
            .unwrap_or(start + 1);
        let lines = &lines[start..end];

        if lines.len() == 1 {
            document.push(TheFeedbackBlock::Paragraph(Self::inline_spans(
                lines[0].trim(),
                TheFeedbackRole::Body,
            )));
            return;
        }

        document.push(TheFeedbackBlock::heading(lines[0].trim()));
        let remainder = &lines[1..];
        if command.is_some_and(|command| {
            command
                .split_whitespace()
                .next()
                .is_some_and(|head| matches!(head, "show" | "get"))
        }) {
            let source = remainder.join("\n").trim().to_string();
            if !source.is_empty() {
                document.push(TheFeedbackBlock::Code {
                    language: Some("toml".to_string()),
                    source,
                });
            }
        } else {
            Self::append_formatted_lines(document, remainder);
        }
    }

    fn response_document(
        response: &shared::rulesets::RulesetHelpResponse,
        command: Option<&str>,
    ) -> TheFeedbackDocument {
        let mut document = TheFeedbackDocument::default();
        if let Some(command) = command {
            document.push(TheFeedbackBlock::Paragraph(vec![
                TheFeedbackSpan::new("Rules  ›  ", TheFeedbackRole::Muted),
                TheFeedbackSpan::new(command, TheFeedbackRole::Command),
            ]));
        }

        Self::append_response_output(&mut document, &response.output, command);

        if !response.commands.is_empty() {
            document.push(TheFeedbackBlock::CommandList {
                title: response.commands_title.clone(),
                entries: response
                    .commands
                    .iter()
                    .map(|entry| {
                        TheFeedbackCommand::new(&entry.command, &entry.description).interactive(
                            HELP_FEEDBACK_ACTION,
                            TheValue::Text(Self::command_target(&entry.command)),
                        )
                    })
                    .collect(),
            });
        }

        if !response.suggestions.is_empty() {
            document.push(TheFeedbackBlock::CommandList {
                title: Some("Try next".to_string()),
                entries: response
                    .suggestions
                    .iter()
                    .map(|suggestion| {
                        TheFeedbackCommand::new(suggestion, "")
                            .interactive(HELP_FEEDBACK_ACTION, TheValue::Text(suggestion.clone()))
                    })
                    .collect(),
            });
        }
        document
    }

    fn error_document(command: Option<&str>, error: impl Into<String>) -> TheFeedbackDocument {
        let mut document = TheFeedbackDocument::default();
        if let Some(command) = command {
            document.push(TheFeedbackBlock::Paragraph(vec![
                TheFeedbackSpan::new("Rules  ›  ", TheFeedbackRole::Muted),
                TheFeedbackSpan::new(command, TheFeedbackRole::Command),
            ]));
        }
        document.push(TheFeedbackBlock::notice(
            TheFeedbackNoticeKind::Error,
            error,
        ));
        document
    }

    fn item_gallery_items(
        rules_source: &str,
        item_ids: &[String],
    ) -> Result<Vec<TheIconGridItem>, String> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }
        let templates = shared::rulesets::ruleset_item_templates_from_source(rules_source)?;
        let templates = templates
            .into_iter()
            .map(|template| (template.id.clone(), template))
            .collect::<std::collections::BTreeMap<_, _>>();
        let runtime = RUSTERIX.read().unwrap();
        let mut items = Vec::new();
        for item_id in item_ids {
            let Some(template) = templates.get(item_id) else {
                continue;
            };
            let mut runtime_item = rusterix::Item::default();
            rusterix::server::data::apply_item_data(&mut runtime_item, &template.data);
            let icon = rusterix::client::widget::Widget::item_generated_icon_square(
                &runtime.assets,
                &runtime_item,
            )
            .map(|(size, pixels)| TheRGBABuffer::from(pixels, size, size))
            .or_else(|| Self::bundled_item_icon(&runtime_item));
            items.push(TheIconGridItem {
                label: template.name.clone(),
                status: format!("{} ({})", template.name, template.id),
                icon,
            });
        }
        Ok(items)
    }

    fn bundled_item_icon(item: &rusterix::Item) -> Option<TheRGBABuffer> {
        let icon_id = item
            .attributes
            .get_str("icon")
            .or_else(|| item.attributes.get_str("icon_template"))?;
        let asset = shared::rulesets::bundled_texture_assets()
            .iter()
            .find(|asset| asset.id == icon_id)?;
        rusterix::Texture::from_image_safe(asset.source).map(|texture| texture.to_rgba())
    }

    fn sync_item_gallery(&self, ui: &mut TheUI, ctx: &mut TheContext, rules_source: &str) {
        let items = Self::item_gallery_items(rules_source, &self.visible_item_ids)
            .unwrap_or_else(|_| Vec::new());
        let show_gallery = !items.is_empty();
        if let Some(gallery) = ui.get_icon_grid_view(HELP_ITEM_GALLERY) {
            gallery.set_items(items);
            gallery.set_selected(None);
        }
        if let Some(layout) = ui.get_sharedvlayout(HELP_CONTENT) {
            layout.set_mode(if show_gallery {
                TheSharedVLayoutMode::Shared
            } else {
                TheSharedVLayoutMode::Top
            });
        }
        ctx.ui.relayout = true;
        ctx.ui.redraw_all = true;
    }

    fn install_response(
        &mut self,
        response: shared::rulesets::RulesetHelpResponse,
        command: Option<&str>,
        rules_source: String,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        self.rules_source = rules_source;
        if response.clear {
            self.document = TheFeedbackDocument::default();
            self.visible_item_ids.clear();
        } else {
            self.visible_item_ids = response.item_ids.clone();
            self.document = Self::response_document(&response, command);
        }
        self.sync_output(ui);
        self.sync_item_gallery(ui, ctx, &self.rules_source);
    }

    fn sync_history_controls(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(back) = ui.get_widget(HELP_BACK) {
            back.set_disabled(self.history_index == 0);
        }
        if let Some(forward) = ui.get_widget(HELP_FORWARD) {
            forward.set_disabled(self.history_index + 1 >= self.history.len());
        }
        ctx.ui.redraw_all = true;
    }

    fn reset_history(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.history.clear();
        self.history.push(None);
        self.history_index = 0;
        self.sync_history_controls(ui, ctx);
    }

    fn record_history(&mut self, command: String, ui: &mut TheUI, ctx: &mut TheContext) {
        let entry = Some(command);
        if self.history.get(self.history_index) == Some(&entry) {
            self.sync_history_controls(ui, ctx);
            return;
        }
        self.history.truncate(self.history_index.saturating_add(1));
        self.history.push(entry);
        self.history_index = self.history.len() - 1;
        self.sync_history_controls(ui, ctx);
    }

    fn execute_command(
        &mut self,
        command: &str,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
    ) {
        match Self::effective_rules(project) {
            Ok(rules_source) => {
                match shared::rulesets::execute_ruleset_help(&rules_source, command) {
                    Ok(response) => {
                        self.install_response(response, Some(command), rules_source, ui, ctx)
                    }
                    Err(error) => {
                        self.rules_source = rules_source;
                        self.visible_item_ids.clear();
                        self.document = Self::error_document(Some(command), error);
                        self.sync_output(ui);
                        self.sync_item_gallery(ui, ctx, &self.rules_source);
                    }
                }
            }
            Err(error) => {
                self.visible_item_ids.clear();
                self.document = Self::error_document(Some(command), error);
                self.sync_output(ui);
                self.sync_item_gallery(ui, ctx, "");
            }
        }
    }

    fn navigate_to_command(
        &mut self,
        command: &str,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
    ) {
        self.execute_command(command, ui, ctx, project);
        self.record_history(command.to_string(), ui, ctx);
    }

    fn navigate_history(
        &mut self,
        offset: isize,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
    ) {
        let target = self.history_index as isize + offset;
        if target < 0 || target >= self.history.len() as isize {
            return;
        }
        self.history_index = target as usize;
        let command = self.history[self.history_index].clone();
        if let Some(command) = command {
            self.execute_command(&command, ui, ctx, project);
        } else {
            match Self::effective_rules(project) {
                Ok(rules_source) => match shared::rulesets::ruleset_help_intro(&rules_source) {
                    Ok(response) => self.install_response(response, None, rules_source, ui, ctx),
                    Err(error) => {
                        self.document = Self::error_document(None, error);
                        self.visible_item_ids.clear();
                        self.sync_output(ui);
                        self.sync_item_gallery(ui, ctx, "");
                    }
                },
                Err(error) => {
                    self.document = Self::error_document(None, error);
                    self.visible_item_ids.clear();
                    self.sync_output(ui);
                    self.sync_item_gallery(ui, ctx, "");
                }
            }
        }
        self.sync_history_controls(ui, ctx);
    }
}

impl Dock for HelpDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            document: TheFeedbackDocument::default(),
            rules_source: String::new(),
            visible_item_ids: Vec::new(),
            history: Vec::new(),
            history_index: 0,
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut output_canvas = TheCanvas::new();
        let mut output = TheTextView::new(TheId::named(HELP_OUTPUT));
        output.set_font_size(13.0);
        output.set_font_preference(TheFontPreference::Default);
        output.set_word_wrap(true);
        output.set_padding((10, 8, 10, 8));
        output.set_selectable(true);
        output.draw_background(true);
        output_canvas.set_widget(output);

        let mut gallery_canvas = TheCanvas::new();
        let mut gallery = TheIconGridView::new(TheId::named(HELP_ITEM_GALLERY));
        gallery.set_cell_size(72);
        gallery.set_icon_size(50);
        gallery.set_icon_padding(5);
        gallery.set_spacing(5);
        gallery.set_content_padding(6);
        gallery.set_show_labels(true);
        gallery_canvas.set_widget(gallery);

        let mut shared = TheSharedVLayout::new(TheId::named(HELP_CONTENT));
        shared.add_canvas(output_canvas);
        shared.add_canvas(gallery_canvas);
        shared.set_mode(TheSharedVLayoutMode::Top);
        shared.set_shared_ratio(0.62);
        shared.set_margin(Vec4::zero());
        shared.set_padding(1);

        let mut canvas = TheCanvas::new();
        canvas.set_layout(shared);

        let mut navigation_canvas = TheCanvas::default();
        navigation_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut navigation = TheHLayout::new(TheId::named("Help Navigation"));
        navigation.set_background_color(None);
        navigation.set_margin(Vec4::new(5, 1, 5, 1));
        navigation.set_padding(3);

        let mut back = TheTraybarButton::new(TheId::named(HELP_BACK));
        back.set_text("<".to_string());
        back.set_fixed_size(true);
        back.set_status_text(&fl!("status_help_back"));
        back.limiter_mut().set_max_size(Vec2::new(24, 20));
        navigation.add_widget(Box::new(back));

        let mut forward = TheTraybarButton::new(TheId::named(HELP_FORWARD));
        forward.set_text(">".to_string());
        forward.set_fixed_size(true);
        forward.set_status_text(&fl!("status_help_forward"));
        forward.limiter_mut().set_max_size(Vec2::new(24, 20));
        navigation.add_widget(Box::new(forward));

        navigation_canvas.set_layout(navigation);
        canvas.set_top(navigation_canvas);

        let mut input_canvas = TheCanvas::default();
        let mut input = TheTextLineEdit::new(TheId::named(HELP_INPUT));
        input.set_status_text(&fl!("status_help_input"));
        input.set_font_size(12.5);
        input.limiter_mut().set_max_height(24);
        input_canvas.set_widget(input);
        canvas.set_bottom(input_canvas);
        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        _server_ctx: &mut ServerContext,
    ) {
        match Self::effective_rules(project) {
            Ok(rules_source) => {
                if self.document.is_empty() || self.rules_source != rules_source {
                    match shared::rulesets::ruleset_help_intro(&rules_source) {
                        Ok(response) => {
                            self.install_response(response, None, rules_source, ui, ctx)
                        }
                        Err(error) => {
                            self.document = Self::error_document(None, error);
                            self.visible_item_ids.clear();
                            self.sync_output(ui);
                            self.sync_item_gallery(ui, ctx, "");
                        }
                    }
                    self.reset_history(ui, ctx);
                } else {
                    self.sync_output(ui);
                    self.sync_item_gallery(ui, ctx, &rules_source);
                }
            }
            Err(error) => {
                self.document = Self::error_document(None, error);
                self.visible_item_ids.clear();
                self.sync_output(ui);
                self.sync_item_gallery(ui, ctx, "");
                self.reset_history(ui, ctx);
            }
        }
        self.sync_history_controls(ui, ctx);
        Self::clear_input(ui);
        Self::focus_input(ui, ctx);
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        if let TheEvent::Custom(id, TheValue::Text(command)) = event
            && id.name == HELP_FEEDBACK_ACTION
        {
            self.navigate_to_command(command, ui, ctx, project);
            Self::clear_input(ui);
            Self::focus_input(ui, ctx);
            return true;
        }

        if let TheEvent::StateChanged(id, TheWidgetState::Clicked) = event {
            if id.name == HELP_BACK {
                self.navigate_history(-1, ui, ctx, project);
                Self::clear_input(ui);
                Self::focus_input(ui, ctx);
                return true;
            }
            if id.name == HELP_FORWARD {
                self.navigate_history(1, ui, ctx, project);
                Self::clear_input(ui);
                Self::focus_input(ui, ctx);
                return true;
            }
        }

        if let TheEvent::IndexChanged(id, index) = event
            && id.name == HELP_ITEM_GALLERY
            && let Some(item_id) = self.visible_item_ids.get(*index).cloned()
        {
            let command = format!("item {item_id}");
            self.navigate_to_command(&command, ui, ctx, project);
            Self::clear_input(ui);
            Self::focus_input(ui, ctx);
            return true;
        }

        if let TheEvent::ValueChanged(id, value) = event
            && id.name == HELP_INPUT
        {
            let command = value.to_string().unwrap_or_default().trim().to_string();
            if command.is_empty() {
                Self::clear_input(ui);
                return false;
            }
            self.navigate_to_command(&command, ui, ctx, project);
            Self::clear_input(ui);
            Self::focus_input(ui, ctx);
            return true;
        }
        false
    }

    fn supports_actions(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruleset_response_becomes_structured_feedback_with_commands() {
        let response = shared::rulesets::execute_ruleset_help(
            shared::rulesets::latest_official_ruleset(),
            "progression Cleric",
        )
        .unwrap();
        let document = HelpDock::response_document(&response, Some("progression Cleric"));
        let text = document.plain_text();
        assert!(text.contains("Rules  ›  progression Cleric"));
        assert!(text.contains("Cleric progression"));
        assert!(text.contains("Try next"));
    }

    #[test]
    fn grant_results_use_structured_lists_without_layout_indentation() {
        let response = shared::rulesets::execute_ruleset_help(
            shared::rulesets::latest_official_ruleset(),
            "spells Cleric",
        )
        .unwrap();
        let document = HelpDock::response_document(&response, Some("spells Cleric"));

        assert!(document.blocks.iter().any(|block| matches!(
            block,
            TheFeedbackBlock::List { items, .. }
                if items.len() >= 2
                    && items.iter().flatten().any(|span| span.role == TheFeedbackRole::StableId)
        )));
        assert!(document.blocks.iter().any(|block| matches!(
            block,
            TheFeedbackBlock::CommandList { title: Some(title), entries }
                if title == "Try next"
                    && entries.iter().all(|entry| entry.description.is_empty())
        )));
    }

    #[test]
    fn raw_show_results_use_a_code_block() {
        let response = shared::rulesets::execute_ruleset_help(
            shared::rulesets::latest_official_ruleset(),
            "show combat",
        )
        .unwrap();
        let document = HelpDock::response_document(&response, Some("show combat"));

        assert!(document.blocks.iter().any(|block| matches!(
            block,
            TheFeedbackBlock::Code { language: Some(language), source }
                if language == "toml" && !source.is_empty()
        )));
    }

    #[test]
    fn intro_uses_a_real_feedback_command_list() {
        let response =
            shared::rulesets::ruleset_help_intro(shared::rulesets::latest_official_ruleset())
                .unwrap();
        let document = HelpDock::response_document(&response, None);

        assert!(document.blocks.iter().any(|block| matches!(
            block,
            TheFeedbackBlock::CommandList { title: Some(title), entries }
                if title == "Valid commands"
                    && entries.iter().any(|entry| entry.command == "progression [class]")
        )));
    }

    #[test]
    fn item_queries_request_icon_gallery_content() {
        let response = shared::rulesets::execute_ruleset_help(
            shared::rulesets::latest_official_ruleset(),
            "item hand_axe",
        )
        .unwrap();
        assert_eq!(response.item_ids, vec!["hand_axe"]);
    }

    #[test]
    fn bundled_item_icon_fallback_is_headless() {
        let template = shared::rulesets::ruleset_item_templates_from_source(
            shared::rulesets::latest_official_ruleset(),
        )
        .unwrap()
        .into_iter()
        .find(|template| template.id == "hand_axe")
        .unwrap();
        let mut item = rusterix::Item::default();
        rusterix::server::data::apply_item_data(&mut item, &template.data);
        assert!(HelpDock::bundled_item_icon(&item).is_some());
    }
}
