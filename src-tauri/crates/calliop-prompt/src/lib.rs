use thiserror::Error;

const THINK_OPEN: &str = concat!("<", "think", ">");
const THINK_CLOSE: &str = concat!("</", "think", ">");

pub const SYSTEM_PROMPT: &str =
    "Tu es un assistant de post-traitement pour une dictée vocale en français. \
Ta tâche : nettoyer la transcription fournie par l'utilisateur. \
Supprime les fillers oraux (euh, bah, heu, du coup, voilà, enfin, etc.). \
Corrige la ponctuation et les majuscules. \
Les commandes de ponctuation et de mise en forme dictées à voix haute ne doivent jamais \
rester sous forme de mots dans le texte final : convertis-les en signes. \
Ponctuation courante : « point » → . ; « virgule » → , ; « point-virgule » / « point virgule » → ; ; \
« deux-points » / « deux points » → : ; « points de suspension » → … ; \
« point d'interrogation » / « point interrogation » → ? ; \
« point d'exclamation » / « point exclamation » → ! ; \
« ouvrez les guillemets » / « fermez les guillemets » → « » ; \
« entre guillemets » entoure le segment suivant avec « », par ex. \
« bonjour entre guillemets » → « bonjour » ou « il dit bonjour entre guillemets » → il dit « bonjour » ; \
« parenthèse ouvrante » / « parenthèse fermante » / « entre parenthèses » → ( ) \
(ex. « Enzo entre parenthèses c'est mon prénom » → Enzo (c'est mon prénom)) ; \
« à la ligne » / « retour à la ligne » → saut de ligne ; « nouveau paragraphe » → double saut de ligne ; \
« tiret » / « trait d'union » → - ; « apostrophe » → ' ; « barre oblique » / « slash » → / ; \
« arobase » / « at » → @ ; « dièse » / « hashtag » → # ; « esperluette » / « et commercial » → & ; \
« pourcent » → % ; « plus » → + ; « moins » → - ; « égal » / « égale » → =. \
Place la ponctuation au bon endroit même si la transcription STT a omis ou décalé les espaces. \
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
    Ok(format!("/no_think\nTranscription brute à nettoyer :\n{raw}"))
}

pub fn validate_cleanup_output(raw: &str, cleaned: &str) -> Result<String, PromptError> {
    let mut cleaned = strip_thinking_blocks(cleaned.trim());
    if cleaned.is_empty() {
        return Err(PromptError::EmptyOutput);
    }

    cleaned = strip_wrapping_quotes(&cleaned);
    cleaned = interpret_oral_punctuation(&cleaned);

    let max_len = raw.len().saturating_mul(3).max(512);
    if cleaned.len() > max_len {
        return Err(PromptError::OutputTooLong);
    }

    Ok(cleaned)
}

fn strip_thinking_blocks(text: &str) -> String {
    if let Some(pos) = text.rfind(THINK_CLOSE) {
        let after = text[pos + THINK_CLOSE.len()..].trim();
        if !after.is_empty() {
            return after.to_string();
        }
    }

    let mut result = text.to_string();
    while let Some(start) = result.find(THINK_OPEN) {
        if let Some(rel_end) = result[start..].find(THINK_CLOSE) {
            let end = start + rel_end + THINK_CLOSE.len();
            result = format!("{}{}", &result[..start], &result[end..]);
        } else {
            result = result[start + THINK_OPEN.len()..].trim().to_string();
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

/// Converts spoken French punctuation commands still present in text (e.g. « virgule » → `,`).
pub fn interpret_oral_punctuation(text: &str) -> String {
    let mut result = interpret_oral_wrap_commands(text);
    for (phrase, replacement) in ORAL_PUNCTUATION_PHRASES {
        result = replace_phrase_ci(&result, phrase, replacement);
    }
    result = replace_spoken_point(&result);
    normalize_punctuation_spacing(&result)
}

const ORAL_WRAP_COMMANDS: &[(&str, &str, &str)] = &[
    ("entre parenthèses", "(", ")"),
    ("entre parenthese", "(", ")"),
    ("entre parentheses", "(", ")"),
    ("entre guillemets", "«", "»"),
    ("entre guillemet", "«", "»"),
];

fn interpret_oral_wrap_commands(text: &str) -> String {
    let mut result = text.to_string();
    loop {
        let Some((phrase, open, close)) = find_next_wrap_command(&result) else {
            break;
        };
        let Some((before, after)) = split_first_phrase_ci(&result, phrase) else {
            break;
        };
        let before = before.trim_end();
        let after = after.trim_start();
        let wrapped = if !after.is_empty() {
            if before.is_empty() {
                format!("{open}{after}{close}")
            } else {
                format!("{before} {open}{after}{close}")
            }
        } else if !before.is_empty() {
            let (prefix, quoted) = quoted_segment_before_command(before);
            if quoted.is_empty() {
                format!("{open}{before}{close}")
            } else if prefix.is_empty() {
                format!("{open}{quoted}{close}")
            } else {
                format!("{prefix} {open}{quoted}{close}")
            }
        } else {
            break;
        };
        result = wrapped;
    }
    result
}

fn quoted_segment_before_command(before: &str) -> (String, String) {
    let trimmed = before.trim();
    let Some(space) = trimmed.rfind(' ') else {
        return (String::new(), trimmed.to_string());
    };
    (
        trimmed[..space].trim_end().to_string(),
        trimmed[space + 1..].trim_start().to_string(),
    )
}

fn find_next_wrap_command(text: &str) -> Option<(&'static str, &'static str, &'static str)> {
    ORAL_WRAP_COMMANDS
        .iter()
        .copied()
        .filter_map(|entry| find_phrase_range_ci(text, entry.0).map(|_| entry))
        .min_by_key(|(phrase, _, _)| find_phrase_range_ci(text, phrase).map(|(start, _)| start).unwrap_or(usize::MAX))
}

fn split_first_phrase_ci(text: &str, phrase: &str) -> Option<(String, String)> {
    let (start, end) = find_phrase_range_ci(text, phrase)?;
    Some((text[..start].to_string(), text[end..].to_string()))
}

fn find_phrase_range_ci(text: &str, phrase: &str) -> Option<(usize, usize)> {
    let phrase_chars: Vec<char> = phrase.chars().collect();
    let phrase_lower: Vec<char> = phrase.to_lowercase().chars().collect();
    let text_chars: Vec<char> = text.chars().collect();

    for (char_index, (byte_start, _)) in text.char_indices().enumerate() {
        if matches_phrase_ci(&text_chars, char_index, &phrase_lower)
            && !is_word_char(at_char(&text_chars, char_index.wrapping_sub(1)))
            && !is_word_char(at_char(
                &text_chars,
                char_index.saturating_add(phrase_chars.len()),
            ))
        {
            let byte_end = text
                .char_indices()
                .nth(char_index + phrase_chars.len())
                .map(|(index, _)| index)
                .unwrap_or(text.len());
            return Some((byte_start, byte_end));
        }
    }

    None
}

/// Longest phrases first so « point d'interrogation » wins over « point ».
const ORAL_PUNCTUATION_PHRASES: &[(&str, &str)] = &[
    ("points de suspension", "…"),
    ("point d'interrogation", "?"),
    ("point d interrogation", "?"),
    ("point interrogation", "?"),
    ("point d'exclamation", "!"),
    ("point d exclamation", "!"),
    ("point exclamation", "!"),
    ("point-virgule", ";"),
    ("point virgule", ";"),
    ("deux-points", ":"),
    ("deux points", ":"),
    ("retour à la ligne", "\n"),
    ("retour a la ligne", "\n"),
    ("nouveau paragraphe", "\n\n"),
    ("à la ligne", "\n"),
    ("a la ligne", "\n"),
    ("parenthèse ouvrante", "("),
    ("parenthese ouvrante", "("),
    ("parenthèse fermante", ")"),
    ("parenthese fermante", ")"),
    ("ouvrez les guillemets", "«"),
    ("fermez les guillemets", "»"),
    ("trait d'union", "-"),
    ("trait d union", "-"),
    ("et commercial", "&"),
    ("barre oblique", "/"),
    ("virgule", ","),
    ("apostrophe", "'"),
    ("esperluette", "&"),
    ("pourcent", "%"),
    ("tiret", "-"),
    ("arobase", "@"),
    ("dièse", "#"),
    ("diese", "#"),
    ("hashtag", "#"),
];

fn replace_phrase_ci(text: &str, phrase: &str, replacement: &str) -> String {
    let phrase_chars: Vec<char> = phrase.chars().collect();
    let phrase_lower: Vec<char> = phrase.to_lowercase().chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut index = 0;

    while index < text_chars.len() {
        if matches_phrase_ci(&text_chars, index, &phrase_lower)
            && !is_word_char(at_char(&text_chars, index.wrapping_sub(1)))
            && !is_word_char(at_char(
                &text_chars,
                index.saturating_add(phrase_chars.len()),
            ))
        {
            out.push_str(replacement);
            index += phrase_chars.len();
        } else {
            out.push(text_chars[index]);
            index += 1;
        }
    }

    out
}

fn replace_spoken_point(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut index = 0;

    while index < chars.len() {
        if matches_phrase_ci(&chars, index, &['p', 'o', 'i', 'n', 't'])
            && !is_word_char(at_char(&chars, index.wrapping_sub(1)))
            && !is_word_char(at_char(&chars, index + 5))
            && !is_point_noun_phrase(&chars, index + 5)
        {
            out.push('.');
            index += 5;
        } else {
            out.push(chars[index]);
            index += 1;
        }
    }

    out
}

fn is_point_noun_phrase(chars: &[char], index: usize) -> bool {
    let rest: String = chars[index..].iter().collect();
    let rest = rest.trim_start();
    rest.starts_with("de ")
        || rest.starts_with("du ")
        || rest.starts_with("des ")
        || rest.starts_with("d'")
        || rest.starts_with("d ")
}

fn matches_phrase_ci(text: &[char], start: usize, phrase_lower: &[char]) -> bool {
    if start + phrase_lower.len() > text.len() {
        return false;
    }
    text[start..start + phrase_lower.len()]
        .iter()
        .zip(phrase_lower.iter())
        .all(|(left, right)| left.to_lowercase().eq(right.to_lowercase()))
}

fn at_char(chars: &[char], index: usize) -> Option<char> {
    if index >= chars.len() {
        None
    } else {
        Some(chars[index])
    }
}

fn is_word_char(ch: Option<char>) -> bool {
    ch.is_some_and(|c| c.is_alphanumeric() || c == '\'' || c == '_')
}

fn normalize_punctuation_spacing(text: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];
        if matches!(ch, ',' | ';' | ':' | '.' | '!' | '?' | '…')
            && out.ends_with(' ')
        {
            out.pop();
        }

        if matches!(ch, ')' | '»' | '!' | '?' | '.' | ',' | ';' | ':' | '…')
            && out.ends_with(' ')
        {
            out.pop();
        }

        out.push(ch);

        if ch == '(' || ch == '«' {
            if index + 1 < chars.len() && chars[index + 1] == ' ' {
                index += 1;
            }
        } else if matches!(ch, ',' | ';' | ':' | '.' | '!' | '?' | '…')
            && index + 1 < chars.len()
            && chars[index + 1] != ' '
            && chars[index + 1] != '\n'
            && !matches!(chars[index + 1], ')' | '»' | '!' | '?' | '.' | ',' | ';' | ':' | '…')
        {
            out.push(' ');
        }

        index += 1;
    }

    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_includes_raw_text() {
        let msg = build_cleanup_user_message("euh bonjour").unwrap();
        assert!(msg.contains("euh bonjour"));
        assert!(msg.starts_with("/no_think"));
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
    fn unclosed_thinking_block_keeps_trailing_text() {
        let raw_output = format!("{THINK_OPEN}hmm still thinking");
        let out = validate_cleanup_output("test", &raw_output).unwrap();
        assert_eq!(out, "hmm still thinking");
    }

    #[test]
    fn prefers_text_after_last_thinking_block() {
        let raw_output = format!("{THINK_OPEN}plan{THINK_CLOSE}Bonjour.");
        let out = validate_cleanup_output("test", &raw_output).unwrap();
        assert_eq!(out, "Bonjour.");
    }

    #[test]
    fn converts_spoken_comma_and_question_mark() {
        let out = interpret_oral_punctuation(
            "Bonjour virgule comment allez-vous point d'interrogation",
        );
        assert_eq!(out, "Bonjour, comment allez-vous?");
    }

    #[test]
    fn converts_spoken_exclamation_and_semicolon() {
        let out = interpret_oral_punctuation("Attention point virgule c'est urgent point d'exclamation");
        assert_eq!(out, "Attention; c'est urgent!");
    }

    #[test]
    fn preserves_point_in_noun_phrase() {
        let out = interpret_oral_punctuation("Le point de vue est clair");
        assert_eq!(out, "Le point de vue est clair");
    }

    #[test]
    fn validate_output_applies_oral_punctuation() {
        let out = validate_cleanup_output(
            "test",
            "Bonjour virgule comment allez-vous point d'interrogation",
        )
        .unwrap();
        assert_eq!(out, "Bonjour, comment allez-vous?");
    }

    #[test]
    fn wraps_text_after_entre_guillemets() {
        let out = interpret_oral_punctuation("il dit bonjour entre guillemets c'est bien");
        assert_eq!(out, "il dit bonjour «c'est bien»");
    }

    #[test]
    fn wraps_last_word_before_entre_guillemets() {
        let out = interpret_oral_punctuation("il dit bonjour entre guillemets");
        assert_eq!(out, "il dit «bonjour»");
    }

    #[test]
    fn wraps_single_word_entre_guillemets() {
        let out = interpret_oral_punctuation("citation entre guillemets");
        assert_eq!(out, "«citation»");
    }

    #[test]
    fn wraps_text_after_entre_parentheses() {
        let out =
            interpret_oral_punctuation("Je m'appelle Enzo entre parenthèses c'est mon prénom");
        assert_eq!(out, "Je m'appelle Enzo (c'est mon prénom)");
    }

    #[test]
    fn system_prompt_covers_oral_punctuation_commands() {
        assert!(SYSTEM_PROMPT.contains("virgule"));
        assert!(SYSTEM_PROMPT.contains("point d'interrogation"));
        assert!(SYSTEM_PROMPT.contains("point d'exclamation"));
        assert!(SYSTEM_PROMPT.contains("entre parenthèses"));
        assert!(SYSTEM_PROMPT.contains("entre guillemets"));
        assert!(SYSTEM_PROMPT.contains("ne doivent jamais"));
    }

    #[test]
    fn fallback_template_targets_qwen3_chatml() {
        assert!(QWEN3_CHAT_TEMPLATE.contains("<|im_start|>"));
        assert!(QWEN3_CHAT_TEMPLATE.contains("add_generation_prompt"));
        assert!(QWEN3_CHAT_TEMPLATE.contains(concat!("<|", "im_end", "|>")));
    }
}
