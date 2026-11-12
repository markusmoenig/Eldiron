//! A project-scoped text owner survives screen navigation. Creator chrome uses
//! its own registry; font blob identities keep shared GPU glyph caches distinct.
use crate::screen::ContentRevision;
use eldiron_runtime::RuntimeView;
use rustc_hash::FxHasher;
use std::{
    hash::{Hash, Hasher},
    sync::atomic::{AtomicU64, Ordering},
};
use theframework::text::TextSystem;
use theframework_core::Uuid;
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameFontVersion {
    owner: u64,
    revision: u64,
    text_revision: u64,
}
pub struct GameFonts {
    pub(super) text: TextSystem,
    diagnostics: Vec<String>,
    owner: u64,
    revision: u64,
    checked: Option<(Uuid, ContentRevision)>,
    fingerprint: Option<u64>,
}
fn builtin_fonts() -> TextSystem {
    let mut text = TextSystem::default();
    for (family, file) in [
        ("Roboto", "fonts/Roboto-Bold.ttf"),
        ("Source Code Pro", "fonts/SourceCodePro-Bold.ttf"),
    ] {
        if let Some(font) = theframework::Embedded::get(file) {
            text.register_font_named(family, font.data.to_vec());
        }
    }
    text
}
impl Default for GameFonts {
    fn default() -> Self {
        Self {
            text: builtin_fonts(),
            diagnostics: Vec::new(),
            owner: NEXT_OWNER.fetch_add(1, Ordering::Relaxed),
            revision: 0,
            checked: None,
            fingerprint: None,
        }
    }
}
impl GameFonts {
    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }
    pub fn text(&self) -> &TextSystem {
        &self.text
    }
    /// An application editing fonts explicitly invalidates retained screen
    /// layouts. Internal layout/paint borrows do not change this generation.
    pub fn text_mut(&mut self) -> &mut TextSystem {
        self.revision = self.revision.wrapping_add(1);
        &mut self.text
    }
    pub fn version(&self) -> GameFontVersion {
        GameFontVersion {
            owner: self.owner,
            revision: self.revision,
            text_revision: self.text.font_revision(),
        }
    }
    /// Check raw bytes only when content/publication changes. Rebuilding on a
    /// real font edit also removes deleted project families; unchanged assets
    /// and screen navigation preserve blob identities and GPU glyph cache hits.
    pub fn refresh(
        &mut self,
        world: &RuntimeView<'_>,
        revision: ContentRevision,
    ) -> GameFontVersion {
        let publication = (world.session_id, revision);
        if self.checked == Some(publication) {
            return self.version();
        }
        self.checked = Some(publication);
        let mut hash = FxHasher::default();
        for font in world.fonts() {
            (font.id, font.name, font.bytes).hash(&mut hash);
        }
        let fingerprint = hash.finish();
        if self.fingerprint == Some(fingerprint) {
            return self.version();
        }
        if self.fingerprint.is_some() {
            self.text = builtin_fonts();
        }
        self.fingerprint = Some(fingerprint);
        self.diagnostics.clear();
        for font in world.fonts() {
            if !self
                .text
                .register_font_named(font.name, font.bytes.to_vec())
            {
                self.diagnostics.push(format!(
                    "Invalid font asset '{}'; using fallback fonts",
                    font.name
                ));
            }
        }
        self.revision = self.revision.wrapping_add(1);
        self.version()
    }
}
