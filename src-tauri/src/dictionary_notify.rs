//! Debounced OS notifications and IPC events for dictionary mutations.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

const DEBOUNCE_MS: u64 = 400;

#[derive(Debug, Clone, Serialize, Default)]
pub struct DictionaryUpdatedPayload {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

pub struct DictionaryNotifier {
    pending: Mutex<DictionaryUpdatedPayload>,
    generation: AtomicU64,
}

impl Default for DictionaryNotifier {
    fn default() -> Self {
        Self {
            pending: Mutex::new(DictionaryUpdatedPayload::default()),
            generation: AtomicU64::new(0),
        }
    }
}

impl DictionaryNotifier {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn queue_added(self: &Arc<Self>, app: &AppHandle, words: Vec<String>) {
        if words.is_empty() {
            return;
        }
        {
            let mut pending = self.pending.lock();
            for word in words {
                if !pending.added.iter().any(|w| w.eq_ignore_ascii_case(&word)) {
                    pending.added.push(word);
                }
            }
        }
        self.schedule_flush(app.clone(), Arc::clone(self));
    }

    pub fn queue_removed(self: &Arc<Self>, app: &AppHandle, words: Vec<String>) {
        if words.is_empty() {
            return;
        }
        {
            let mut pending = self.pending.lock();
            pending.removed.extend(words);
        }
        self.schedule_flush(app.clone(), Arc::clone(self));
    }

    pub fn emit_immediate(self: &Arc<Self>, app: &AppHandle, payload: DictionaryUpdatedPayload) {
        if payload.added.is_empty() && payload.removed.is_empty() {
            return;
        }
        let _ = app.emit("dictionary-updated", payload);
    }

    fn schedule_flush(self: &Arc<Self>, app: AppHandle, notifier: Arc<Self>) {
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_millis(DEBOUNCE_MS)).await;
            if notifier.generation.load(Ordering::SeqCst) != generation {
                return;
            }
            notifier.flush(&app);
        });
    }

    fn flush(self: &Arc<Self>, app: &AppHandle) {
        let payload = {
            let mut pending = self.pending.lock();
            if pending.added.is_empty() && pending.removed.is_empty() {
                return;
            }
            std::mem::take(&mut *pending)
        };

        if !payload.added.is_empty() {
            send_dictionary_notification(app, &payload.added);
        }
        let payload = DictionaryUpdatedPayload {
            source: Some("learned".into()),
            ..payload
        };
        let _ = app.emit("dictionary-updated", payload);
    }
}

fn send_dictionary_notification(app: &AppHandle, words: &[String]) {
    if words.is_empty() {
        return;
    }

    let body = if words.len() == 1 {
        format!("Mot ajouté au dictionnaire : {}", words[0])
    } else {
        format!(
            "{} mots ajoutés au dictionnaire : {}",
            words.len(),
            words.join(", ")
        )
    };

    let _ = app
        .notification()
        .builder()
        .title("Calliop")
        .body(body)
        .show();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_serializes() {
        let payload = DictionaryUpdatedPayload {
            added: vec!["Calliop".into()],
            removed: vec![],
            source: None,
        };
        let json = serde_json::to_string(&payload).expect("json");
        assert!(json.contains("Calliop"));
    }
}
