use serde_json::Value;

use crate::store::detect_default_ui_language;

const LOCALE_FR: &str = include_str!("../../../locales/fr.json");
const LOCALE_EN: &str = include_str!("../../../locales/en.json");

fn locale_json(ui_language: &str) -> &'static str {
    if ui_language == "en" {
        LOCALE_EN
    } else {
        LOCALE_FR
    }
}

fn lookup_value<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in key.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

pub fn default_ui_language() -> String {
    detect_default_ui_language()
}

pub fn tr(key: &str, ui_language: &str) -> String {
    let json = locale_json(ui_language);
    let value: Value = serde_json::from_str(json).expect("locale json must parse");
    lookup_value(&value, key)
        .and_then(|v| v.as_str())
        .unwrap_or(key)
        .to_string()
}

pub fn tr_with_vars(key: &str, ui_language: &str, vars: &[(&str, &str)]) -> String {
    let mut text = tr(key, ui_language);
    for (name, value) in vars {
        text = text.replace(&format!("{{{{{name}}}}}"), value);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tr_resolves_nested_keys() {
        assert_eq!(tr("tray.open", "fr"), "Ouvrir Calliop");
        assert_eq!(tr("tray.open", "en"), "Open Calliop");
    }

    #[test]
    fn tr_with_vars_replaces_placeholders() {
        let text = tr_with_vars("tray.dictationLanguage", "fr", &[("label", "FR")]);
        assert_eq!(text, "Langue de dictée : FR");
    }

    #[test]
    fn tr_falls_back_to_key_when_missing() {
        assert_eq!(tr("missing.key", "fr"), "missing.key");
    }
}
