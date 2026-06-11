use std::fmt;

pub const STT_LANG_AUTO: &str = "auto";
pub const DEFAULT_STT_LANGUAGE: &str = "fr";

/// Languages exposed in the v1 UI.
pub const SUPPORTED_STT_LANGUAGES: &[&str] = &["fr", "en"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SttLanguage {
    Auto,
    Fixed(&'static str),
}

impl SttLanguage {
    pub fn parse(s: &str) -> Option<Self> {
        if s == STT_LANG_AUTO {
            return Some(Self::Auto);
        }
        match s {
            "fr" => Some(Self::Fixed("fr")),
            "en" => Some(Self::Fixed("en")),
            _ => None,
        }
    }

    pub fn default_fixed() -> Self {
        Self::Fixed(DEFAULT_STT_LANGUAGE)
    }

    pub fn cycle(&self) -> Self {
        match self {
            Self::Fixed("fr") => Self::Fixed("en"),
            Self::Fixed(_) => Self::Auto,
            Self::Auto => Self::default_fixed(),
        }
    }

    pub fn as_setting_value(&self) -> String {
        match self {
            Self::Auto => STT_LANG_AUTO.into(),
            Self::Fixed(code) => (*code).to_string(),
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Auto => "AUTO",
            Self::Fixed("fr") => "FR",
            Self::Fixed("en") => "EN",
            Self::Fixed(_) => "??",
        }
    }
}

impl Default for SttLanguage {
    fn default() -> Self {
        Self::default_fixed()
    }
}

impl fmt::Display for SttLanguage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_setting_value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_auto_and_fixed() {
        assert_eq!(SttLanguage::parse("auto"), Some(SttLanguage::Auto));
        assert_eq!(SttLanguage::parse("fr"), Some(SttLanguage::Fixed("fr")));
        assert_eq!(SttLanguage::parse("en"), Some(SttLanguage::Fixed("en")));
        assert_eq!(SttLanguage::parse("de"), None);
    }

    #[test]
    fn cycle_fr_en_auto() {
        let fr = SttLanguage::Fixed("fr");
        let en = fr.cycle();
        assert_eq!(en, SttLanguage::Fixed("en"));
        let auto = en.cycle();
        assert_eq!(auto, SttLanguage::Auto);
        assert_eq!(auto.cycle(), SttLanguage::default_fixed());
    }

    #[test]
    fn as_setting_value_roundtrip() {
        for value in ["fr", "en", "auto"] {
            let lang = SttLanguage::parse(value).expect("valid");
            assert_eq!(lang.as_setting_value(), value);
        }
    }
}
