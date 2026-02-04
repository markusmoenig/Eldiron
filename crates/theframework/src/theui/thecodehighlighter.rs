use lazy_static::lazy_static;
use std::sync::Arc;
use syntect::{
    easy::HighlightLines,
    highlighting::{Theme, ThemeSet},
    parsing::{SyntaxDefinition, SyntaxReference, SyntaxSet},
};
use unicode_segmentation::UnicodeSegmentation;

use crate::prelude::*;

lazy_static! {
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

pub struct TheCodeHighlighter {
    syntax_set: SyntaxSet,
    syntax: Arc<SyntaxReference>,
    theme: Theme,
    using_custom_theme: bool,
}

impl Default for TheCodeHighlighter {
    fn default() -> Self {
        let syntax_set = SyntaxSet::load_defaults_nonewlines();
        Self {
            syntax: Arc::new(syntax_set.find_syntax_plain_text().clone()),
            theme: THEME_SET.themes["Solarized (light)"].clone(),
            syntax_set,
            using_custom_theme: false,
        }
    }
}

pub trait TheCodeHighlighterTrait: Send {
    fn syntax(&self) -> &str;

    fn syntect_syntax(&self) -> &SyntaxReference;
    fn syntect_theme(&self) -> &Theme;

    fn set_syntax_by_name(&mut self, name: &str);
    fn set_theme(&mut self, theme: &str);
    fn add_syntax_from_string(&mut self, syntax_str: &str) -> Result<(), String>;
    fn add_theme_from_string(&mut self, theme_str: &str) -> Result<(), String>;

    fn background(&self) -> Option<TheColor>;
    fn caret(&self) -> Option<TheColor>;
    fn guide(&self) -> Option<TheColor>;
    fn active_guide(&self) -> Option<TheColor>;
    fn selection_background(&self) -> Option<TheColor>;
    fn match_background(&self) -> Option<TheColor>;
    fn active_match_background(&self) -> Option<TheColor>;
    fn misspelling(&self) -> Option<TheColor>;

    fn highlight_line(
        &self,
        line: &str,
        h: &mut HighlightLines,
    ) -> Vec<(TheColor, TheColor, usize)>;
}

impl TheCodeHighlighterTrait for TheCodeHighlighter {
    fn syntax(&self) -> &str {
        &self.syntax.name
    }

    fn syntect_syntax(&self) -> &SyntaxReference {
        &self.syntax
    }

    fn syntect_theme(&self) -> &Theme {
        &self.theme
    }

    fn set_syntax_by_name(&mut self, name: &str) {
        if let Some(syntax) = self.syntax_set.find_syntax_by_name(name) {
            self.syntax = Arc::new(syntax.clone());
        }
    }

    fn set_theme(&mut self, theme: &str) {
        if let Some(theme) = THEME_SET.themes.get(theme) {
            self.theme = theme.clone();
            self.using_custom_theme = false;
        }
    }

    fn add_syntax_from_string(&mut self, syntax_str: &str) -> Result<(), String> {
        let mut builder = SyntaxSet::load_defaults_newlines().into_builder();

        // Parse the new syntax from the provided string
        match SyntaxDefinition::load_from_str(syntax_str, true, None) {
            Ok(syntax) => {
                builder.add(syntax); // Correct method
                self.syntax_set = builder.build();
                Ok(())
            }
            Err(e) => Err(format!("Failed to load syntax: {}", e)),
        }
    }

    fn add_theme_from_string(&mut self, theme_str: &str) -> Result<(), String> {
        use std::io::Cursor;

        // Parse the theme from the provided string (expects .tmTheme XML format)
        match ThemeSet::load_from_reader(&mut Cursor::new(theme_str)) {
            Ok(theme) => {
                self.theme = theme;
                self.using_custom_theme = true;
                Ok(())
            }
            Err(e) => Err(format!("Failed to load theme: {}", e)),
        }
    }

    fn background(&self) -> Option<TheColor> {
        self.theme
            .settings
            .background
            .map(|color| TheColor::from_u8(color.r, color.g, color.b, color.a))
    }

    fn caret(&self) -> Option<TheColor> {
        self.theme
            .settings
            .caret
            .map(|color| TheColor::from_u8(color.r, color.g, color.b, color.a))
    }

    fn guide(&self) -> Option<TheColor> {
        self.theme
            .settings
            .guide
            .map(|color| TheColor::from_u8(color.r, color.g, color.b, color.a))
    }

    fn active_guide(&self) -> Option<TheColor> {
        self.theme
            .settings
            .active_guide
            .map(|color| TheColor::from_u8(color.r, color.g, color.b, color.a))
    }

    fn selection_background(&self) -> Option<TheColor> {
        self.theme
            .settings
            .selection
            .map(|color| TheColor::from_u8(color.r, color.g, color.b, color.a))
    }

    fn match_background(&self) -> Option<TheColor> {
        self.theme
            .settings
            .inactive_selection
            .map(|color| TheColor::from_u8(color.r, color.g, color.b, color.a))
    }

    fn active_match_background(&self) -> Option<TheColor> {
        self.theme
            .settings
            .find_highlight
            .map(|color| TheColor::from_u8(color.r, color.g, color.b, color.a))
    }

    fn misspelling(&self) -> Option<TheColor> {
        self.theme
            .settings
            .misspelling
            .map(|color| TheColor::from_u8(color.r, color.g, color.b, color.a))
    }

    fn highlight_line(
        &self,
        line: &str,
        h: &mut HighlightLines,
    ) -> Vec<(TheColor, TheColor, usize)> {
        h.highlight_line(line, &self.syntax_set)
            .map(|ranges| {
                ranges
                    .iter()
                    .map(|(style, token)| {
                        (
                            TheColor::from_u8(
                                style.foreground.r,
                                style.foreground.g,
                                style.foreground.b,
                                style.foreground.a,
                            ),
                            TheColor::from_u8(
                                style.background.r,
                                style.background.g,
                                style.background.b,
                                style.background.a,
                            ),
                            token.graphemes(true).count(),
                        )
                    })
                    .collect::<Vec<(TheColor, TheColor, usize)>>()
            })
            .unwrap_or(vec![(
                TheColor::default(),
                TheColor::default().lighten_darken(0.1),
                line.len(),
            )])
    }
}
