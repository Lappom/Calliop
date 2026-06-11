use calliop_prompt::ToneProfile;

use crate::store::{AppContextMatchType, AppContextRule};

use super::ActiveWindow;

/// Resolves the tone profile for the active window using ordered user rules.
pub fn resolve_tone(window: &ActiveWindow, rules: &[AppContextRule]) -> ToneProfile {
    for rule in rules {
        if rule_matches(window, rule) {
            return rule.tone;
        }
    }
    ToneProfile::Default
}

fn rule_matches(window: &ActiveWindow, rule: &AppContextRule) -> bool {
    match rule.match_type {
        AppContextMatchType::Exe => window.exe_name.eq_ignore_ascii_case(&rule.pattern),
        AppContextMatchType::TitleContains => window
            .title
            .to_ascii_lowercase()
            .contains(&rule.pattern.to_ascii_lowercase()),
    }
}
