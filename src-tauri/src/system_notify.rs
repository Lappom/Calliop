//! OS notifications for model readiness and app updates (works when the window is hidden).

use parking_lot::Mutex;
use crate::store::{self, Store};
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ModelsReadyState {
    whisper_loaded: bool,
    llm_loaded: bool,
    llm_blocks_notify: bool,
    notified: bool,
}

fn should_notify_models_ready(state: &ModelsReadyState) -> bool {
    if state.notified || !state.whisper_loaded {
        return false;
    }
    if state.llm_blocks_notify && !state.llm_loaded {
        return false;
    }
    true
}

fn models_ready_copy(ui_language: &str) -> (&'static str, &'static str) {
    if ui_language.eq_ignore_ascii_case("en") {
        ("Calliop", "Models ready. You can dictate.")
    } else {
        ("Calliop", "Modèles prêts. Vous pouvez dicter.")
    }
}

fn update_ready_copy(ui_language: &str, version: &str) -> (&'static str, String) {
    if ui_language.eq_ignore_ascii_case("en") {
        (
            "Calliop",
            format!("Update {version} ready to install."),
        )
    } else {
        (
            "Calliop",
            format!("Mise à jour {version} prête à installer."),
        )
    }
}

fn ui_language_from_store(store: &Store) -> String {
    store
        .load_settings()
        .map(|settings| settings.ui_language)
        .unwrap_or_else(|_| store::detect_default_ui_language())
}

fn onboarding_done(store: &Store) -> bool {
    store.is_onboarding_done().unwrap_or(false)
}

pub fn show_os_notification(app: &AppHandle, title: &str, body: &str) {
    let _ = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show();
}

pub struct ModelsReadyNotifier {
    state: Mutex<ModelsReadyState>,
}

impl Default for ModelsReadyNotifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelsReadyNotifier {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(ModelsReadyState::default()),
        }
    }

    pub fn reset(&self) {
        *self.state.lock() = ModelsReadyState::default();
    }

    pub fn reset_for_llm_reload(&self) {
        let mut state = self.state.lock();
        state.llm_loaded = false;
    }

    pub fn on_whisper_loaded(
        &self,
        app: &AppHandle,
        store: &Store,
        auto_edit: bool,
        preload_llm: bool,
    ) {
        {
            let mut state = self.state.lock();
            state.whisper_loaded = true;
            state.llm_blocks_notify = auto_edit && preload_llm;
        }
        self.maybe_notify(app, store);
    }

    pub fn on_llm_loaded(&self, app: &AppHandle, store: &Store) {
        self.state.lock().llm_loaded = true;
        self.maybe_notify(app, store);
    }

    fn maybe_notify(&self, app: &AppHandle, store: &Store) {
        if !onboarding_done(store) {
            return;
        }

        let should_notify = {
            let mut state = self.state.lock();
            if !should_notify_models_ready(&state) {
                return;
            }
            state.notified = true;
            true
        };

        if should_notify {
            let ui_language = ui_language_from_store(store);
            let (title, body) = models_ready_copy(&ui_language);
            show_os_notification(app, title, body);
        }
    }
}

pub fn notify_update_ready(app: &AppHandle, store: &Store, version: &str) {
    if cfg!(debug_assertions) {
        return;
    }
    if !onboarding_done(store) {
        return;
    }

    let ui_language = ui_language_from_store(store);
    let (title, body) = update_ready_copy(&ui_language, version);
    show_os_notification(app, title, &body);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whisper_only_notifies_immediately() {
        let state = ModelsReadyState {
            whisper_loaded: true,
            llm_blocks_notify: false,
            ..Default::default()
        };
        assert!(should_notify_models_ready(&state));
    }

    #[test]
    fn whisper_with_blocking_llm_waits_for_llm() {
        let waiting = ModelsReadyState {
            whisper_loaded: true,
            llm_blocks_notify: true,
            ..Default::default()
        };
        assert!(!should_notify_models_ready(&waiting));

        let ready = ModelsReadyState {
            whisper_loaded: true,
            llm_loaded: true,
            llm_blocks_notify: true,
            ..Default::default()
        };
        assert!(should_notify_models_ready(&ready));
    }

    #[test]
    fn already_notified_skips() {
        let state = ModelsReadyState {
            whisper_loaded: true,
            notified: true,
            ..Default::default()
        };
        assert!(!should_notify_models_ready(&state));
    }

    #[test]
    fn whisper_not_loaded_skips() {
        let state = ModelsReadyState {
            llm_loaded: true,
            llm_blocks_notify: true,
            ..Default::default()
        };
        assert!(!should_notify_models_ready(&state));
    }

    #[test]
    fn reload_after_initial_notify_does_not_renotify() {
        let state = ModelsReadyState {
            whisper_loaded: true,
            llm_loaded: false,
            llm_blocks_notify: true,
            notified: true,
        };
        assert!(!should_notify_models_ready(&state));
    }

    #[test]
    fn english_copy_for_models_and_update() {
        let (title, body) = models_ready_copy("en");
        assert_eq!(title, "Calliop");
        assert!(body.contains("Models ready"));

        let (title, body) = update_ready_copy("en", "1.2.3");
        assert_eq!(title, "Calliop");
        assert!(body.contains("1.2.3"));
    }

    #[test]
    fn french_copy_for_models_and_update() {
        let (_, body) = models_ready_copy("fr");
        assert!(body.contains("Modèles prêts"));

        let (_, body) = update_ready_copy("fr", "1.2.3");
        assert!(body.contains("1.2.3"));
    }
}
