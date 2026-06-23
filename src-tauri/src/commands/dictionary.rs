use tauri::{AppHandle, State};

use crate::dictionary_notify::DictionaryUpdatedPayload;
use crate::store::{is_valid_dictionary_word, normalize_word, DictionarySource};
use crate::user_error::{user_error_string, UserError};
use crate::{
    apply_dictionary_additions, apply_learned_correction, dictionary_word_to_payload,
    refresh_correction_rules, refresh_whisper_prompt_full, AppState, DictionaryWordPayload,
};

#[tauri::command]
pub fn list_dictionary_words(
    state: State<'_, AppState>,
) -> Result<Vec<DictionaryWordPayload>, String> {
    state
        .store
        .list_words()
        .map(|words| words.into_iter().map(dictionary_word_to_payload).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_dictionary_word(
    app: AppHandle,
    state: State<'_, AppState>,
    word: String,
    misspelling: Option<String>,
) -> Result<bool, String> {
    let normalized = normalize_word(&word);
    if normalized.is_empty() {
        return Err(user_error_string(UserError::DictionaryWordEmpty));
    }
    if !is_valid_dictionary_word(&normalized) {
        return Err(user_error_string(UserError::DictionaryWordInvalid));
    }

    let normalized_misspelling = misspelling
        .as_deref()
        .map(normalize_word)
        .filter(|value| !value.is_empty());
    if let Some(ref incorrect) = normalized_misspelling {
        if !is_valid_dictionary_word(incorrect) {
            return Err(user_error_string(UserError::DictionaryMisspellingInvalid));
        }
        if normalized.eq_ignore_ascii_case(incorrect) {
            return Err(user_error_string(UserError::DictionaryMisspellingSame));
        }
    }

    let inserted_id = state
        .store
        .add_word(
            &normalized,
            DictionarySource::Manual,
            normalized_misspelling.as_deref(),
        )
        .map_err(|e| e.to_string())?;

    let Some(inserted_id) = inserted_id else {
        if normalized_misspelling.is_some() {
            return Err(user_error_string(UserError::DictionaryMisspellingExists));
        }
        return Err(user_error_string(UserError::DictionaryWordExists));
    };

    if let Err(err) = apply_dictionary_additions(&state, std::slice::from_ref(&normalized)) {
        let _ = state.store.remove_word(inserted_id);
        return Err(err);
    }
    if let Err(err) = refresh_correction_rules(&state.store, &state.pipeline) {
        let _ = state.store.remove_word(inserted_id);
        return Err(err);
    }

    state.dictionary_notifier.emit_immediate(
        &app,
        DictionaryUpdatedPayload {
            added: vec![normalized],
            removed: vec![],
            source: Some("manual".into()),
        },
    );

    if let Err(err) = state.achievements.on_feature_change(&app) {
        eprintln!("achievement evaluation failed: {err}");
    }

    Ok(true)
}

#[tauri::command]
pub fn update_dictionary_word(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    word: String,
) -> Result<bool, String> {
    let normalized = normalize_word(&word);
    if normalized.is_empty() {
        return Err(user_error_string(UserError::DictionaryWordEmpty));
    }
    if !is_valid_dictionary_word(&normalized) {
        return Err(user_error_string(UserError::DictionaryWordInvalid));
    }

    let previous = state
        .store
        .get_word_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| user_error_string(UserError::DictionaryWordNotFound))?;

    if previous.word == normalized {
        return Ok(true);
    }

    let updated = state
        .store
        .update_word(id, &normalized)
        .map_err(|e| e.to_string())?;

    if updated {
        if let Err(err) = refresh_whisper_prompt_full(&state) {
            let _ = state.store.update_word(id, &previous.word);
            return Err(err);
        }
        state.dictionary_notifier.emit_immediate(
            &app,
            DictionaryUpdatedPayload {
                added: vec![normalized.clone()],
                removed: vec![previous.word],
                source: Some("manual".into()),
            },
        );
    }

    Ok(updated)
}

#[tauri::command]
pub fn remove_dictionary_word(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let entry = state
        .store
        .get_word_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| user_error_string(UserError::DictionaryWordNotFound))?;

    let removed = state.store.remove_word(id).map_err(|e| e.to_string())?;
    if !removed {
        return Err(user_error_string(UserError::DictionaryWordNotFound));
    }

    if let Err(err) = refresh_whisper_prompt_full(&state) {
        let _ = state
            .store
            .add_word(&entry.word, entry.source, entry.misspelling.as_deref());
        return Err(err);
    }

    if let Err(err) = refresh_correction_rules(&state.store, &state.pipeline) {
        let _ = state
            .store
            .add_word(&entry.word, entry.source, entry.misspelling.as_deref());
        return Err(err);
    }

    state.dictionary_notifier.emit_immediate(
        &app,
        DictionaryUpdatedPayload {
            added: vec![],
            removed: vec![entry.word],
            source: None,
        },
    );
    Ok(())
}

#[tauri::command]
pub fn learn_from_correction(
    app: AppHandle,
    state: State<'_, AppState>,
    original: String,
    corrected: String,
) -> Result<Vec<String>, String> {
    apply_learned_correction(&app, &state, &original, &corrected)
}
