use crate::prelude::*;

/// Renderer-independent, structured application feedback.
/// Producers describe meaning; views decide how to present it.
#[derive(Clone, Debug, Default)]
pub struct TheFeedbackDocument {
    pub blocks: Vec<TheFeedbackBlock>,
}

impl TheFeedbackDocument {
    pub fn new(blocks: Vec<TheFeedbackBlock>) -> Self {
        Self { blocks }
    }

    pub fn push(&mut self, block: TheFeedbackBlock) {
        self.blocks.push(block);
    }

    pub fn extend(&mut self, other: TheFeedbackDocument) {
        self.blocks.extend(other.blocks);
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Copy/paste, logging and non-rich-view fallback.
    pub fn plain_text(&self) -> String {
        let mut output = String::new();
        for block in &self.blocks {
            block.write_plain_text(&mut output);
        }
        output.trim_end().to_string()
    }

    pub fn to_text_view_blocks(&self, palette: &TheFeedbackPalette) -> Vec<TheTextViewBlock> {
        let mut output = Vec::new();
        for block in &self.blocks {
            block.render(&mut output, palette);
        }
        output
    }
}

#[derive(Clone, Debug)]
pub enum TheFeedbackBlock {
    Heading {
        level: u8,
        spans: Vec<TheFeedbackSpan>,
    },
    Paragraph(Vec<TheFeedbackSpan>),
    Notice {
        kind: TheFeedbackNoticeKind,
        title: Option<String>,
        spans: Vec<TheFeedbackSpan>,
    },
    CommandList {
        title: Option<String>,
        entries: Vec<TheFeedbackCommand>,
    },
    KeyValueList {
        title: Option<String>,
        entries: Vec<TheFeedbackKeyValue>,
    },
    List {
        title: Option<String>,
        items: Vec<Vec<TheFeedbackSpan>>,
    },
    Code {
        language: Option<String>,
        source: String,
    },
    Divider,
}

impl TheFeedbackBlock {
    pub fn heading(text: impl Into<String>) -> Self {
        Self::Heading {
            level: 1,
            spans: vec![TheFeedbackSpan::new(text, TheFeedbackRole::Heading)],
        }
    }

    pub fn paragraph(text: impl Into<String>) -> Self {
        Self::Paragraph(vec![TheFeedbackSpan::body(text)])
    }

    pub fn notice(kind: TheFeedbackNoticeKind, text: impl Into<String>) -> Self {
        Self::Notice {
            kind,
            title: None,
            spans: vec![TheFeedbackSpan::body(text)],
        }
    }

    fn write_plain_text(&self, output: &mut String) {
        match self {
            Self::Heading { spans, .. } | Self::Paragraph(spans) => {
                write_spans(output, spans);
                output.push_str("\n\n");
            }
            Self::Notice { kind, title, spans } => {
                output.push_str(match kind {
                    TheFeedbackNoticeKind::Info => "Info: ",
                    TheFeedbackNoticeKind::Success => "OK: ",
                    TheFeedbackNoticeKind::Warning => "Warning: ",
                    TheFeedbackNoticeKind::Error => "Error: ",
                });
                if let Some(title) = title {
                    output.push_str(title);
                    output.push_str(": ");
                }
                write_spans(output, spans);
                output.push_str("\n\n");
            }
            Self::CommandList { title, entries } => {
                write_title(output, title);
                for entry in entries {
                    output.push_str(&entry.command);
                    if !entry.description.is_empty() {
                        output.push_str("\n  ");
                        output.push_str(&entry.description);
                    }
                    output.push('\n');
                }
                output.push('\n');
            }
            Self::KeyValueList { title, entries } => {
                write_title(output, title);
                for entry in entries {
                    output.push_str(&entry.key);
                    output.push_str(" = ");
                    write_spans(output, &entry.value);
                    output.push('\n');
                }
                output.push('\n');
            }
            Self::List { title, items } => {
                write_title(output, title);
                for item in items {
                    output.push_str("- ");
                    write_spans(output, item);
                    output.push('\n');
                }
                output.push('\n');
            }
            Self::Code { source, .. } => {
                output.push_str(source.trim_end());
                output.push_str("\n\n");
            }
            Self::Divider => output.push_str("---\n\n"),
        }
    }

    fn render(&self, output: &mut Vec<TheTextViewBlock>, palette: &TheFeedbackPalette) {
        match self {
            Self::Heading { level, spans } => {
                let mut spans = spans.clone();
                for span in &mut spans {
                    if span.role == TheFeedbackRole::Body {
                        span.role = TheFeedbackRole::Heading;
                    }
                }
                output.push(render_spans(&spans, palette, "\n\n", Some(*level)));
            }
            Self::Paragraph(spans) => output.push(render_spans(spans, palette, "\n\n", None)),
            Self::Notice { kind, title, spans } => {
                let role = match kind {
                    TheFeedbackNoticeKind::Info => TheFeedbackRole::Info,
                    TheFeedbackNoticeKind::Success => TheFeedbackRole::Success,
                    TheFeedbackNoticeKind::Warning => TheFeedbackRole::Warning,
                    TheFeedbackNoticeKind::Error => TheFeedbackRole::Error,
                };
                let mut rendered = vec![TheFeedbackSpan::new(
                    match kind {
                        TheFeedbackNoticeKind::Info => "INFO  ",
                        TheFeedbackNoticeKind::Success => "OK  ",
                        TheFeedbackNoticeKind::Warning => "WARNING  ",
                        TheFeedbackNoticeKind::Error => "ERROR  ",
                    },
                    role,
                )];
                if let Some(title) = title {
                    rendered.push(TheFeedbackSpan::new(title, role));
                    rendered.push(TheFeedbackSpan::body(" — "));
                }
                rendered.extend(spans.clone());
                output.push(render_spans(&rendered, palette, "\n\n", None));
            }
            Self::CommandList { title, entries } => {
                render_title(output, title, palette);
                if entries.is_empty() {
                    output.push(empty_block(palette));
                } else {
                    for (index, entry) in entries.iter().enumerate() {
                        let command_suffix = if entry.description.is_empty() {
                            if index + 1 == entries.len() {
                                "\n\n"
                            } else {
                                "\n"
                            }
                        } else {
                            "\n"
                        };
                        output.push(render_spans(
                            &[TheFeedbackSpan {
                                text: entry.command.clone(),
                                role: TheFeedbackRole::Command,
                                interaction: entry.interaction.clone(),
                            }],
                            palette,
                            command_suffix,
                            None,
                        ));
                        if entry.description.is_empty() {
                            continue;
                        }
                        output.push(render_spans(
                            &[TheFeedbackSpan::new(
                                format!("  {}", entry.description),
                                if entry.available {
                                    TheFeedbackRole::Muted
                                } else {
                                    TheFeedbackRole::Disabled
                                },
                            )],
                            palette,
                            if index + 1 == entries.len() {
                                "\n\n"
                            } else {
                                "\n"
                            },
                            None,
                        ));
                    }
                }
            }
            Self::KeyValueList { title, entries } => {
                render_title(output, title, palette);
                if entries.is_empty() {
                    output.push(empty_block(palette));
                } else {
                    for (index, entry) in entries.iter().enumerate() {
                        let mut spans = vec![TheFeedbackSpan::new(
                            entry.key.clone(),
                            TheFeedbackRole::Key,
                        )];
                        spans.push(TheFeedbackSpan::new(" = ", TheFeedbackRole::Muted));
                        spans.extend(entry.value.clone());
                        output.push(render_spans(
                            &spans,
                            palette,
                            if index + 1 == entries.len() {
                                "\n\n"
                            } else {
                                "\n"
                            },
                            None,
                        ));
                    }
                }
            }
            Self::List { title, items } => {
                render_title(output, title, palette);
                if items.is_empty() {
                    output.push(empty_block(palette));
                } else {
                    for (index, item) in items.iter().enumerate() {
                        let mut spans = vec![TheFeedbackSpan::new("• ", TheFeedbackRole::Muted)];
                        spans.extend(item.clone());
                        output.push(render_spans(
                            &spans,
                            palette,
                            if index + 1 == items.len() {
                                "\n\n"
                            } else {
                                "\n"
                            },
                            None,
                        ));
                    }
                }
            }
            Self::Code { language, source } => {
                if let Some(language) = language {
                    output.push(render_spans(
                        &[TheFeedbackSpan::new(
                            language.to_ascii_uppercase(),
                            TheFeedbackRole::Muted,
                        )],
                        palette,
                        "\n",
                        None,
                    ));
                }
                output.push(render_spans(
                    &[TheFeedbackSpan::new(
                        source.trim_end(),
                        TheFeedbackRole::Code,
                    )],
                    palette,
                    "\n\n",
                    None,
                ));
            }
            Self::Divider => output.push(render_spans(
                &[TheFeedbackSpan::new(
                    "────────────────",
                    TheFeedbackRole::Divider,
                )],
                palette,
                "\n\n",
                None,
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TheFeedbackNoticeKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TheFeedbackRole {
    Body,
    Muted,
    Heading,
    Command,
    StableId,
    Key,
    StringValue,
    NumberValue,
    BoolValue,
    Code,
    Info,
    Success,
    Warning,
    Error,
    Disabled,
    Divider,
}

#[derive(Clone, Debug)]
pub struct TheFeedbackInteraction {
    pub id: TheId,
    pub value: TheValue,
}

impl TheFeedbackInteraction {
    pub fn new(id: impl Into<String>, value: TheValue) -> Self {
        let id = id.into();
        Self {
            id: TheId::named(&id),
            value,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TheFeedbackSpan {
    pub text: String,
    pub role: TheFeedbackRole,
    pub interaction: Option<TheFeedbackInteraction>,
}

impl TheFeedbackSpan {
    pub fn new(text: impl Into<String>, role: TheFeedbackRole) -> Self {
        Self {
            text: text.into(),
            role,
            interaction: None,
        }
    }

    pub fn body(text: impl Into<String>) -> Self {
        Self::new(text, TheFeedbackRole::Body)
    }

    pub fn interactive(mut self, id: impl Into<String>, value: TheValue) -> Self {
        self.interaction = Some(TheFeedbackInteraction::new(id, value));
        self
    }
}

#[derive(Clone, Debug)]
pub struct TheFeedbackCommand {
    pub command: String,
    pub description: String,
    pub available: bool,
    pub interaction: Option<TheFeedbackInteraction>,
}

impl TheFeedbackCommand {
    pub fn new(command: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            description: description.into(),
            available: true,
            interaction: None,
        }
    }

    pub fn interactive(mut self, id: impl Into<String>, value: TheValue) -> Self {
        self.interaction = Some(TheFeedbackInteraction::new(id, value));
        self
    }

    pub fn available(mut self, available: bool) -> Self {
        self.available = available;
        self
    }
}

#[derive(Clone, Debug)]
pub struct TheFeedbackKeyValue {
    pub key: String,
    pub value: Vec<TheFeedbackSpan>,
}

impl TheFeedbackKeyValue {
    pub fn new(key: impl Into<String>, value: Vec<TheFeedbackSpan>) -> Self {
        Self {
            key: key.into(),
            value,
        }
    }

    pub fn text(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(
            key,
            vec![TheFeedbackSpan::new(value, TheFeedbackRole::StringValue)],
        )
    }
}

/// Semantic colors are independent from document production and can be swapped per view/theme.
#[derive(Clone, Debug)]
pub struct TheFeedbackPalette {
    pub body: TheColor,
    pub muted: TheColor,
    pub heading: TheColor,
    pub command: TheColor,
    pub stable_id: TheColor,
    pub key: TheColor,
    pub string_value: TheColor,
    pub number_value: TheColor,
    pub bool_value: TheColor,
    pub code: TheColor,
    pub info: TheColor,
    pub success: TheColor,
    pub warning: TheColor,
    pub error: TheColor,
    pub disabled: TheColor,
    pub divider: TheColor,
    pub interaction_background: TheColor,
}

impl Default for TheFeedbackPalette {
    fn default() -> Self {
        Self {
            body: TheColor::from_hex("#E8E5DE"),
            muted: TheColor::from_hex("#9B968D"),
            heading: TheColor::from_hex("#FFD866"),
            command: TheColor::from_hex("#78DCE8"),
            stable_id: TheColor::from_hex("#AB9DF2"),
            key: TheColor::from_hex("#A9DC76"),
            string_value: TheColor::from_hex("#FFD866"),
            number_value: TheColor::from_hex("#AB9DF2"),
            bool_value: TheColor::from_hex("#FC9867"),
            code: TheColor::from_hex("#D7D3CB"),
            info: TheColor::from_hex("#78DCE8"),
            success: TheColor::from_hex("#A9DC76"),
            warning: TheColor::from_hex("#FFD866"),
            error: TheColor::from_hex("#FF6188"),
            disabled: TheColor::from_hex("#69665F"),
            divider: TheColor::from_hex("#514E49"),
            interaction_background: TheColor::from_hex("#313A43"),
        }
    }
}

impl TheFeedbackPalette {
    fn color(&self, role: TheFeedbackRole) -> TheColor {
        match role {
            TheFeedbackRole::Body => self.body.clone(),
            TheFeedbackRole::Muted => self.muted.clone(),
            TheFeedbackRole::Heading => self.heading.clone(),
            TheFeedbackRole::Command => self.command.clone(),
            TheFeedbackRole::StableId => self.stable_id.clone(),
            TheFeedbackRole::Key => self.key.clone(),
            TheFeedbackRole::StringValue => self.string_value.clone(),
            TheFeedbackRole::NumberValue => self.number_value.clone(),
            TheFeedbackRole::BoolValue => self.bool_value.clone(),
            TheFeedbackRole::Code => self.code.clone(),
            TheFeedbackRole::Info => self.info.clone(),
            TheFeedbackRole::Success => self.success.clone(),
            TheFeedbackRole::Warning => self.warning.clone(),
            TheFeedbackRole::Error => self.error.clone(),
            TheFeedbackRole::Disabled => self.disabled.clone(),
            TheFeedbackRole::Divider => self.divider.clone(),
        }
    }
}

fn empty_block(palette: &TheFeedbackPalette) -> TheTextViewBlock {
    render_spans(
        &[TheFeedbackSpan::new("None", TheFeedbackRole::Muted)],
        palette,
        "\n\n",
        None,
    )
}

fn render_title(
    output: &mut Vec<TheTextViewBlock>,
    title: &Option<String>,
    palette: &TheFeedbackPalette,
) {
    if let Some(title) = title {
        output.push(render_spans(
            &[TheFeedbackSpan::new(title, TheFeedbackRole::Heading)],
            palette,
            "\n",
            Some(2),
        ));
    }
}

fn render_spans(
    spans: &[TheFeedbackSpan],
    palette: &TheFeedbackPalette,
    suffix: &str,
    heading_level: Option<u8>,
) -> TheTextViewBlock {
    let mut block = TheTextViewBlock::default();
    for span in spans {
        let foreground = palette.color(span.role);
        block.spans.push(TheTextViewSpan {
            text: span.text.clone(),
            style: TheTextStyle {
                foreground: Some(foreground.clone()),
                background: None,
                underline: span.interaction.as_ref().map(|_| foreground),
            },
            interaction: span
                .interaction
                .as_ref()
                .map(|interaction| TheTextViewInteraction {
                    id: interaction.id.clone(),
                    value: interaction.value.clone(),
                    hover_background: Some(palette.interaction_background.clone()),
                }),
        });
    }
    if !suffix.is_empty() {
        block.spans.push(TheTextViewSpan {
            text: suffix.to_string(),
            style: TheTextStyle::default(),
            interaction: None,
        });
    }

    // Preserved for future renderers with per-block typography.
    let _ = heading_level;
    block
}

fn write_spans(output: &mut String, spans: &[TheFeedbackSpan]) {
    for span in spans {
        output.push_str(&span.text);
    }
}

fn write_title(output: &mut String, title: &Option<String>) {
    if let Some(title) = title {
        output.push_str(title);
        output.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_a_stable_fallback() {
        let document = TheFeedbackDocument::new(vec![TheFeedbackBlock::CommandList {
            title: Some("Commands".to_string()),
            entries: vec![TheFeedbackCommand::new("show", "Show the focused object")],
        }]);

        assert_eq!(
            document.plain_text(),
            "Commands\nshow\n  Show the focused object"
        );
    }
}
