// use crate::editor::RUSTERIX;
use crate::prelude::*;
use theframework::prelude::*;

pub struct LogDock;

impl LogDock {
    fn display_text(log: &str) -> String {
        log.split_inclusive('\n')
            .map(|line| {
                let (line, newline) = line
                    .strip_suffix('\n')
                    .map_or((line, ""), |line| (line, "\n"));
                let trimmed = line.trim_start();
                let indentation = &line[..line.len() - trimmed.len()];
                let marker_len = ["[warning]", "[warn]", "[error]", "[err]"]
                    .into_iter()
                    .find(|marker| {
                        trimmed
                            .get(..marker.len())
                            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(marker))
                    })
                    .map_or(0, str::len);

                if marker_len == 0 {
                    format!("{line}{newline}")
                } else {
                    format!(
                        "{indentation}{}{newline}",
                        trimmed[marker_len..].trim_start()
                    )
                }
            })
            .collect()
    }

    pub(crate) fn set_output(log: &str, ui: &mut TheUI, _ctx: &mut TheContext) {
        if let Some(output) = ui.get_text_area_edit("LogEdit") {
            output.set_value(TheValue::Text(Self::display_text(log)));
            output.set_highlight_source(Some(log.to_string()));
        }
    }
}

impl Dock for LogDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut textedit = TheTextAreaEdit::new(TheId::named("LogEdit"));

        if let Some(bytes) = crate::Embedded::get("parser/gruvbox-dark.tmTheme") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                textedit.add_theme_from_string(source);
                textedit.set_code_theme("Gruvbox Dark");
            }
        }

        if let Some(bytes) = crate::Embedded::get("parser/log.sublime-syntax") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                textedit.add_syntax_from_string(source);
                textedit.set_code_type("Eldiron Log");
            }
        }

        textedit.set_continuous(true);
        textedit.display_line_number(false);
        textedit.use_global_statusbar(true);
        textedit.set_font_size(12.5);
        textedit.set_word_wrap(true);
        // Handled manually, but this dock is read-only
        textedit.set_supports_undo(false);
        textedit.readonly(true);

        center.set_widget(textedit);

        center
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &Project,
        _server_ctx: &mut ServerContext,
    ) {
        if let Some(widget) = ui.get_widget("LogEdit") {
            ctx.ui.set_focus(widget.id());
        }
    }

    fn supports_actions(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_markers_are_not_part_of_displayed_log_text() {
        let raw = "Started\n[warning] Missing entrance\n  [ERROR] Setup failed\n";
        assert_eq!(
            LogDock::display_text(raw),
            "Started\nMissing entrance\n  Setup failed\n"
        );
    }
}
