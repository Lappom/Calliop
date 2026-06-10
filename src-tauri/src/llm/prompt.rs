use thiserror::Error;

pub const SYSTEM_PROMPT: &str =
    "Tu es un assistant de post-traitement pour une dictée vocale en français. \
Ta tâche : nettoyer la transcription fournie par l'utilisateur. \
Supprime les fillers oraux (euh, bah, heu, du coup, voilà, enfin, etc.). \
Corrige la ponctuation et les majuscules. \
Reformule légèrement si nécessaire pour améliorer la fluidité, sans changer le sens. \
Ne commente pas, ne pose pas de questions, ne reformate pas en liste. \
Réponds uniquement avec le texte nettoyé final, sans guillemets ni préambule.";

#[derive(Debug, Error)]
pub enum PromptError {
    #[error("empty input text")]
    EmptyInput,
    #[error("empty model output")]
    EmptyOutput,
    #[error("model output too long")]
    OutputTooLong,
}

pub fn build_cleanup_user_message(raw: &str) -> Result<String, PromptError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(PromptError::EmptyInput);
    }
    Ok(format!("Transcription brute à nettoyer :\n{raw}"))
}

pub fn validate_cleanup_output(raw: &str, cleaned: &str) -> Result<String, PromptError> {
    let mut cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() {
        return Err(PromptError::EmptyOutput);
    }

    cleaned = strip_wrapping_quotes(&cleaned);

    let max_len = raw.len().saturating_mul(3).max(512);
    if cleaned.len() > max_len {
        return Err(PromptError::OutputTooLong);
    }

    Ok(cleaned)
}

fn strip_wrapping_quotes(text: &str) -> String {
    let trimmed = text.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('«') && trimmed.ends_with('»'))
    {
        trimmed[1..trimmed.len() - 1].trim().to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_includes_raw_text() {
        let msg = build_cleanup_user_message("euh bonjour").unwrap();
        assert!(msg.contains("euh bonjour"));
    }

    #[test]
    fn rejects_empty_input() {
        assert!(build_cleanup_user_message("  ").is_err());
    }

    #[test]
    fn strips_wrapping_quotes_from_output() {
        let out = validate_cleanup_output("euh bonjour", "\"Bonjour.\"").unwrap();
        assert_eq!(out, "Bonjour.");
    }

    #[test]
    fn rejects_empty_output() {
        assert!(validate_cleanup_output("test", "   ").is_err());
    }
}
