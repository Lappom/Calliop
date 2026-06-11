use calliop_prompt::ToneProfile;

use crate::store::{exe_names_match, AppContextMatchType, AppContextRule};

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
        AppContextMatchType::Exe => exe_names_match(&rule.pattern, &window.exe_name),
        AppContextMatchType::TitleContains => window
            .title
            .to_ascii_lowercase()
            .contains(&rule.pattern.to_ascii_lowercase()),
    }
}
