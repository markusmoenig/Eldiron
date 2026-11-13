//! Startup font source policy shared by framework controls and authored game text.
use super::*;
impl TextSystem {
    /// Existing UI/code faces remain primary. Browser fallback uses supplied
    /// font bytes; shaping/rasterization never depends on DOM text or CSS fonts.
    pub fn register_bundled_fonts(&mut self) {
        for (family, path) in [
            ("Roboto", "fonts/Roboto-Bold.ttf"),
            ("Source Code Pro", "fonts/SourceCodePro-Bold.ttf"),
        ] {
            if let Some(font) = crate::Embedded::get(path) {
                self.register_font_named(family, font.data.to_vec());
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let language = web_sys::window().and_then(|window| window.navigator().language());
            self.register_cjk_fallback(cjk_data(), language.as_deref());
            self.register_arabic_fallback(arabic_data());
        }
    }
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn register_arabic_fallback(&mut self, data: Arc<[u8]>) {
        let data = self.font_blob(data);
        if self.fonts.collection.register_fonts(data, None).is_empty() {
            return;
        }
        if let Some(family) = self.fonts.collection.family_by_name("Noto Sans Arabic") {
            self.fonts.collection.set_fallbacks(
                "Arab".parse::<parley::fontique::Script>().unwrap(),
                std::iter::once(family.id()),
            );
        }
        self.fonts_changed();
    }
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn register_cjk_fallback(&mut self, data: Arc<[u8]>, language: Option<&str>) {
        let data = self.font_blob(data);
        if self.fonts.collection.register_fonts(data, None).is_empty() {
            return;
        }
        let language = language.unwrap_or("en").to_ascii_lowercase();
        let han = if language.starts_with("ja") {
            "Noto Sans CJK JP"
        } else if language.starts_with("ko") {
            "Noto Sans CJK KR"
        } else if language.starts_with("zh") && (language.contains("hk") || language.contains("mo"))
        {
            "Noto Sans CJK HK"
        } else if language.starts_with("zh")
            && (language.contains("hant") || language.contains("tw"))
        {
            "Noto Sans CJK TC"
        } else {
            "Noto Sans CJK SC"
        };
        for (script, family) in [
            ("Hani", han),
            ("Cyrl", han),
            ("Hira", "Noto Sans CJK JP"),
            ("Kana", "Noto Sans CJK JP"),
            ("Hang", "Noto Sans CJK KR"),
            ("Bopo", "Noto Sans CJK TC"),
        ] {
            if let Some(family) = self.fonts.collection.family_by_name(family) {
                let script: parley::fontique::Script = script.parse().unwrap();
                self.fonts
                    .collection
                    .set_fallbacks(script, std::iter::once(family.id()));
            }
        }
        self.fonts_changed();
    }
}
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn cjk_data() -> Arc<[u8]> {
    static DATA: std::sync::OnceLock<Arc<[u8]>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        Arc::from(include_bytes!("../../webfonts/NotoSansCJK-Regular.ttc").as_slice())
    })
    .clone()
}

#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn arabic_data() -> Arc<[u8]> {
    static DATA: std::sync::OnceLock<Arc<[u8]>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        Arc::from(include_bytes!("../../webfonts/NotoSansArabic-Regular.ttf").as_slice())
    })
    .clone()
}
