use i18n_embed::{
    DesktopLanguageRequester, I18nEmbedError, LanguageLoader,
    fluent::{FluentLanguageLoader, fluent_language_loader},
    unic_langid::LanguageIdentifier,
};
use lazy_static::lazy_static;
use rust_embed::RustEmbed;

macro_rules! lang {
    ($id:tt, $label:tt) => {
        Language {
            id: $id,
            label: $label,
        }
    };
}

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

lazy_static! {
    pub static ref LANGUAGE_LOADER: FluentLanguageLoader = {
        let loader: FluentLanguageLoader = fluent_language_loader!();

        loader.load_fallback_language(&Localizations).unwrap();
        // TheFramework's bitmap text renderer does not interpret Fluent's invisible
        // bidirectional-isolation markers and renders them as missing glyphs instead.
        loader.set_use_isolating(false);

        loader
    };
}

pub struct Language {
    pub id: &'static str,
    pub label: &'static str,
}

const_array!(
    pub LANGUAGES: Language [
        // lang!("ar", "العربية (Arabic)"),
        // lang!("de", "Deutsch (German)"),
        lang!("en", "English (English)"),
        // lang!("es", "Español (Spanish)"),
        // lang!("fr", "Français (French)"),
        // lang!("it", "Italiano (Italian)"),
        // lang!("ja", "日本語 (Japanese)"),
        // lang!("ko", "한국어 (Korean)"),
        // lang!("pt", "Português (Portuguese)"),
        // lang!("ru", "Русский (Russian)"),
        lang!("zh_CN", "简体中文 (Simplified Chinese)"),
        lang!("zh_TW", "繁體中文 (Traditional Chinese)")
    ]
);

pub fn select_locales(
    request_languages: &[&'static str],
) -> Result<Vec<LanguageIdentifier>, I18nEmbedError> {
    let requested_languages: Vec<LanguageIdentifier> = request_languages
        .iter()
        .filter_map(|raw| raw.parse().ok())
        .collect();

    let selected = i18n_embed::select(&*LANGUAGE_LOADER, &Localizations, &requested_languages)?;
    LANGUAGE_LOADER.set_use_isolating(false);
    Ok(selected)
}

pub fn select_system_locales() -> Result<Vec<LanguageIdentifier>, I18nEmbedError> {
    let requested_languages = DesktopLanguageRequester::requested_languages();

    let selected = i18n_embed::select(&*LANGUAGE_LOADER, &Localizations, &requested_languages)?;
    LANGUAGE_LOADER.set_use_isolating(false);
    Ok(selected)
}
