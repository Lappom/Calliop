use thiserror::Error;

const THINK_OPEN: &str = concat!("<", "think", ">");
const THINK_CLOSE: &str = concat!("</", "think", ">");

pub const SYSTEM_PROMPT: &str =
    "Tu es un assistant de post-traitement pour une dictée vocale en français. \
Ta tâche : nettoyer la transcription fournie par l'utilisateur. \
Supprime les fillers oraux (euh, bah, heu, du coup, voilà, enfin, etc.). \
Corrige la ponctuation et les majuscules. \
Interprète les commandes de mise en forme dictées à voix haute : \
« entre parenthèses » ou « parenthèse ouvrante/fermante » → ( ), \
« entre guillemets » → guillemets typographiques, \
« à la ligne » → saut de ligne, \
« point » / « virgule » / « deux-points » → . , :, etc. \
Reformule légèrement si nécessaire pour améliorer la fluidité, sans changer le sens. \
Ne commente pas, ne pose pas de questions, ne reformate pas en liste. \
Réponds uniquement avec le texte nettoyé final, sans guillemets ni préambule.";

/// Fallback Jinja template for Qwen3 GGUF files without embedded chat metadata.
/// Disables thinking mode via an empty thinking block in the generation prompt.
pub const QWEN3_CHAT_TEMPLATE: &str = concat!(
    r#"{%- for message in messages -%}"#,
    "\n",
    r#"{%- if message.role == "system" -%}"#,
    "\n",
    "<|im_start|>system\n",
    r#"{{ message.content }}"#,
    "\n",
    "<|",
    "im_end",
    "|>\n",
    r#"{%- elif message.role == "user" -%}"#,
    "\n",
    "<|im_start|>user\n",
    r#"{{ message.content }}"#,
    "\n",
    "<|",
    "im_end",
    "|>\n",
    r#"{%- elif message.role == "assistant" -%}"#,
    "\n",
    "<|im_start|>assistant\n",
    r#"{{ message.content }}"#,
    "\n",
    "<|",
    "im_end",
    "|>\n",
    r#"{%- endif -%}"#,
    "\n",
    r#"{%- endfor -%}"#,
    "\n",
    r#"{%- if add_generation_prompt -%}"#,
    "\n",
    "<|im_start|>assistant\n",
    "<",
    "think",
    ">\n\n",
    "</",
    "think",
    ">\n\n",
    r#"{%- endif -%}"#,
);

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
    let mut cleaned = strip_thinking_blocks(cleaned.trim());
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

fn strip_thinking_blocks(text: &str) -> String {
    let mut result = text.to_string();
    while let Some(start) = result.find(THINK_OPEN) {
        if let Some(rel_end) = result[start..].find(THINK_CLOSE) {
            let end = start + rel_end + THINK_CLOSE.len();
            result = format!("{}{}", &result[..start], &result[end..]);
        } else {
            result = result[..start].to_string();
            break;
        }
    }
    result.trim().to_string()
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
    fn strips_qwen3_thinking_blocks() {
        let raw_output = format!("{THINK_OPEN}hmm{THINK_CLOSE}Bonjour.");
        let out = validate_cleanup_output("test", &raw_output).unwrap();
        assert_eq!(out, "Bonjour.");
    }

    #[test]
    fn rejects_empty_output() {
        assert!(validate_cleanup_output("test", "   ").is_err());
    }

    #[test]
    fn fallback_template_targets_qwen3_chatml() {
        assert!(QWEN3_CHAT_TEMPLATE.contains("<|im_start|>"));
        assert!(QWEN3_CHAT_TEMPLATE.contains("add_generation_prompt"));
        assert!(QWEN3_CHAT_TEMPLATE.contains(concat!("<|", "im_end", "|>")));
    }
}
