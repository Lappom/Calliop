//! Active-window detection and tone resolution for per-app dictation context.

mod matcher;

#[cfg(windows)]
mod detect_win;

pub use matcher::resolve_tone;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActiveWindow {
    pub title: String,
    #[serde(rename = "exeName")]
    pub exe_name: String,
    #[serde(rename = "exePath")]
    pub exe_path: Option<String>,
}

/// Returns the foreground window outside Calliop, if detectable.
pub fn get_active_window() -> Option<ActiveWindow> {
    #[cfg(windows)]
    {
        detect_win::get_active_window()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

pub fn module_name() -> &'static str {
    "app_context"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{AppContextMatchType, AppContextRule};
    use calliop_prompt::ToneProfile;

    #[test]
    fn exposes_module_name() {
        assert_eq!(module_name(), "app_context");
    }

    #[test]
    fn resolve_tone_with_exe_rule() {
        let window = ActiveWindow {
            title: "General - Slack".into(),
            exe_name: "slack.exe".into(),
            exe_path: None,
        };
        let rules = vec![AppContextRule {
            id: 1,
            pattern: "slack.exe".into(),
            match_type: AppContextMatchType::Exe,
            tone: ToneProfile::Casual,
            created_at: String::new(),
        }];
        assert_eq!(resolve_tone(&window, &rules), ToneProfile::Casual);
    }

    #[test]
    fn resolve_tone_falls_back_to_default() {
        let window = ActiveWindow {
            title: "Untitled".into(),
            exe_name: "notepad.exe".into(),
            exe_path: None,
        };
        assert_eq!(resolve_tone(&window, &[]), ToneProfile::Default);
    }

    #[test]
    fn resolve_tone_exe_rule_without_exe_suffix() {
        let window = ActiveWindow {
            title: "VS Code".into(),
            exe_name: "code.exe".into(),
            exe_path: None,
        };
        let rules = vec![AppContextRule {
            id: 1,
            pattern: "code".into(),
            match_type: AppContextMatchType::Exe,
            tone: ToneProfile::Technical,
            created_at: String::new(),
        }];
        assert_eq!(resolve_tone(&window, &rules), ToneProfile::Technical);
    }

    #[test]
    fn resolve_tone_prefers_newer_rule() {
        let window = ActiveWindow {
            title: "General - Slack".into(),
            exe_name: "slack.exe".into(),
            exe_path: None,
        };
        let rules = vec![
            AppContextRule {
                id: 2,
                pattern: "slack.exe".into(),
                match_type: AppContextMatchType::Exe,
                tone: ToneProfile::Formal,
                created_at: String::new(),
            },
            AppContextRule {
                id: 1,
                pattern: "slack.exe".into(),
                match_type: AppContextMatchType::Exe,
                tone: ToneProfile::Casual,
                created_at: String::new(),
            },
        ];
        assert_eq!(resolve_tone(&window, &rules), ToneProfile::Formal);
    }

    #[test]
    fn resolve_tone_title_contains_rule() {
        let window = ActiveWindow {
            title: "Boîte de réception - Outlook".into(),
            exe_name: "olk.exe".into(),
            exe_path: None,
        };
        let rules = vec![AppContextRule {
            id: 1,
            pattern: "outlook".into(),
            match_type: AppContextMatchType::TitleContains,
            tone: ToneProfile::Formal,
            created_at: String::new(),
        }];
        assert_eq!(resolve_tone(&window, &rules), ToneProfile::Formal);
    }
}
