use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Writing tone applied during LLM cleanup based on the active application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ToneProfile {
    #[default]
    Default,
    Casual,
    Formal,
    Technical,
}

impl ToneProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Casual => "casual",
            Self::Formal => "formal",
            Self::Technical => "technical",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "default" => Some(Self::Default),
            "casual" => Some(Self::Casual),
            "formal" => Some(Self::Formal),
            "technical" => Some(Self::Technical),
            _ => None,
        }
    }
}

/// Builds the system prompt for LLM cleanup, optionally tuned for the target app tone.
pub fn build_system_prompt(profile: ToneProfile) -> String {
    let suffix = match profile {
        ToneProfile::Default => String::new(),
        ToneProfile::Casual => " Contexte : messagerie ou chat (ex. Slack). \
Ton détendu et direct : phrases courtes, tutoiement si naturel, pas de formules de politesse lourdes."
            .into(),
        ToneProfile::Formal => " Contexte : courriel ou communication professionnelle formelle. \
Vouvoiement si approprié, formules de politesse sobres, structure claire, ton professionnel."
            .into(),
        ToneProfile::Technical => " Contexte : développement logiciel ou terminal. \
Style concis et technique : vocabulaire précis, commits ou commentaires courts si pertinent, \
pas de langage marketing."
            .into(),
    };
    format!("{SYSTEM_PROMPT}{suffix}")
}

const THINK_OPEN: &str = concat!("<", "think", ">");
const THINK_CLOSE: &str = concat!("</", "think", ">");

pub const SYSTEM_PROMPT: &str =
    "Tu es un assistant de post-traitement pour une dictée vocale en français. \
Ta tâche : nettoyer la transcription fournie par l'utilisateur. \
Supprime les fillers oraux (euh, bah, heu, ok, alors, bon, du coup, voilà, enfin, etc.) \
et les amorces de phrase (« ok alors là », « bon ben », etc.). \
Quand l'utilisateur hésite puis reprend (faux départ suivi de « … » ou d'une reformulation), \
ne garde que la formulation la plus complète et cohérente — supprime les fragments abandonnés. \
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
« pourcent » → % ; « signe plus » / « plus » (entre tokens) → + ; \
« signe moins » → - ; « signe égal » / « égale » → =. \
Pour les adresses et chemins, colle les symboles aux tokens voisins \
(ex. « contact at gmail point com » → contact@gmail.com ; « src slash lib » → src/lib). \
Place la ponctuation au bon endroit même si la transcription STT a omis ou décalé les espaces. \
Reformule légèrement si nécessaire pour améliorer la fluidité, sans changer le sens. \
Conserve les jetons ⟦CALLIOP0⟧, ⟦CALLIOP1⟧, etc. inchangés s'ils apparaissent dans la transcription. \
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
    Ok(format!(
        "/no_think\nTranscription brute à nettoyer :\n{raw}"
    ))
}

pub fn validate_cleanup_output(raw: &str, cleaned: &str) -> Result<String, PromptError> {
    let mut cleaned = strip_thinking_blocks(cleaned.trim());
    if cleaned.is_empty() {
        return Err(PromptError::EmptyOutput);
    }

    cleaned = strip_wrapping_quotes(&cleaned);
    cleaned = interpret_oral_punctuation(&cleaned);
    cleaned = polish_llm_output(&cleaned);

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

/// Example dictation baked into Whisper's `initial_prompt` so oral symbols are recognized.
pub const WHISPER_ORAL_VOCABULARY_HINT: &str = "Mon email jean arobase gmail point com, \
dossier src slash lib slash main point rs, tag hashtag rust, version signe plus signe plus.";

pub fn whisper_oral_vocabulary_word_count() -> usize {
    WHISPER_ORAL_VOCABULARY_HINT.split_whitespace().count()
}

/// Joins streaming STT segments without spurious mid-sentence capitals.
pub fn join_transcript_segments(segments: &[impl AsRef<str>]) -> String {
    let mut result = String::new();
    for segment in segments {
        let segment = segment.as_ref().trim();
        if segment.is_empty() {
            continue;
        }
        if result.is_empty() {
            result = segment.to_string();
            continue;
        }
        if !result.ends_with(' ') {
            result.push(' ');
        }
        if should_lowercase_segment_join(&result, segment) {
            result.push_str(&lowercase_first_char(segment));
        } else {
            result.push_str(segment);
        }
    }
    result
}

/// Full transcript cleanup after STT: fix mishearings, oral punctuation, then polish.
pub fn post_process_transcript(text: &str) -> String {
    let text = interpret_oral_punctuation(&normalize_stt_oral_mishearings(text));
    polish_transcript(&text)
}

/// Deterministic polish applied after STT (includes filler stripping).
fn polish_transcript(text: &str) -> String {
    let text = fix_malformed_punctuation(text);
    let text = strip_leading_oral_fillers(&text);
    polish_llm_output(&text)
}

/// Final polish after LLM cleanup (no filler stripping — LLM already handles it).
fn polish_llm_output(text: &str) -> String {
    let text = fix_malformed_punctuation(text);
    let text = collapse_extra_whitespace(&text);
    fix_sentence_capitalization(&text)
}

/// Longest phrases first so « ok alors là » wins over « ok ».
const ORAL_FILLERS: &[&str] = &[
    "ok alors là",
    "ok alors",
    "bon alors",
    "ben voilà",
    "du coup",
    "alors là",
    "enfin bon",
    "alors",
    "enfin",
    "voilà",
    "donc",
    "bon",
    "ben",
    "beh",
    "bah",
    "euh",
    "euhm",
    "hem",
    "hmm",
    "ok",
];

fn strip_leading_oral_fillers(text: &str) -> String {
    let mut result = text.trim().to_string();
    loop {
        let mut stripped = false;
        for filler in ORAL_FILLERS {
            if let Some(rest) = strip_prefix_phrase_ci(&result, filler) {
                result = rest
                    .trim_start_matches([',', ' ', '—', '-'])
                    .trim_start()
                    .to_string();
                stripped = true;
                break;
            }
        }
        if !stripped {
            break;
        }
    }
    result
}

fn strip_prefix_phrase_ci<'a>(text: &'a str, phrase: &str) -> Option<&'a str> {
    let (start, end) = find_phrase_range_ci(text, phrase)?;
    if start != 0 {
        return None;
    }
    Some(text[end..].trim_start())
}

fn should_lowercase_segment_join(previous: &str, next: &str) -> bool {
    let previous = previous.trim_end();
    if previous.is_empty() {
        return false;
    }
    if previous.ends_with("...") || previous.ends_with('…') {
        return next
            .trim()
            .chars()
            .next()
            .is_some_and(|ch| ch.is_uppercase());
    }
    if matches!(previous.chars().last(), Some('.' | '!' | '?' | '…')) {
        return false;
    }
    next.trim()
        .chars()
        .next()
        .is_some_and(|ch| ch.is_uppercase())
}

fn lowercase_first_char(text: &str) -> String {
    let trimmed = text.trim();
    let mut chars = trimmed.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut out = first.to_lowercase().to_string();
            out.extend(chars);
            out
        }
    }
}

fn fix_malformed_punctuation(text: &str) -> String {
    let mut result = text.to_string();
    for broken in ["(, ", "(,", "( ,", "( ,"] {
        result = result.replace(broken, "");
    }
    result = result.replace(".)", ".");
    result = result.replace("()", "");
    result = result.replace("( )", "");
    unwrap_paren_wrapped_period(&result)
}

fn unwrap_paren_wrapped_period(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == '(' {
            if let Some(close) = find_paren_period_close(&chars, index) {
                let inner: String = chars[index + 1..close].iter().collect();
                let inner = inner.trim();
                if !inner.is_empty() && !inner.contains('(') {
                    if !out.is_empty() && !out.ends_with(' ') {
                        out.push(' ');
                    }
                    let body = inner.trim_end_matches('.');
                    out.push_str(body);
                    out.push('.');
                    index = close + 1;
                    continue;
                }
            }
        }
        out.push(chars[index]);
        index += 1;
    }

    out
}

fn find_paren_period_close(chars: &[char], open: usize) -> Option<usize> {
    let mut index = open + 1;
    while index + 1 < chars.len() {
        if chars[index] == '.' && chars[index + 1] == ')' {
            return Some(index + 1);
        }
        index += 1;
    }
    None
}

fn collapse_extra_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn fix_sentence_capitalization(text: &str) -> String {
    if text.contains('@') || text.contains('/') {
        return text.to_string();
    }
    let mut chars = text.chars();
    match chars.next() {
        None => String::new(),
        Some(first) if first.is_alphabetic() => {
            let mut out = first.to_uppercase().to_string();
            out.extend(chars);
            out
        }
        Some(first) => {
            let mut out = String::from(first);
            out.extend(chars);
            out
        }
    }
}

/// Fixes frequent Whisper mis-transcriptions of spoken symbol commands (before punctuation pass).
pub fn normalize_stt_oral_mishearings(text: &str) -> String {
    let mut result = text.to_string();
    for (phrase, replacement) in STT_ORAL_MISHEARING_FIXES {
        result = replace_phrase_ci(&result, phrase, replacement);
    }
    let result = fix_misheard_arobase_phrase(&result, "a base", "arobase");
    fix_misheard_arobase_phrase(&result, "à base", "arobase")
}

const STT_ORAL_MISHEARING_FIXES: &[(&str, &str)] = &[
    ("arrobasse", "arobase"),
    ("arrobase", "arobase"),
    ("arro basse", "arobase"),
    ("arro bas", "arobase"),
    ("à robase", "arobase"),
    ("a robase", "arobase"),
    ("a robe base", "arobase"),
    ("at sign", "at"),
    ("bar oblique", "barre oblique"),
    ("slache", "slash"),
    ("slach", "slash"),
    ("hashtague", "hashtag"),
    ("hash tag", "hashtag"),
    ("esperluete", "esperluette"),
    ("et comercial", "et commercial"),
    ("signe egale", "signe égal"),
];

/// Replaces « a base » with « arobase » only in email/path dictation context.
fn fix_misheard_arobase_phrase(text: &str, misheard: &str, replacement: &str) -> String {
    let misheard_chars: Vec<char> = misheard.chars().collect();
    let misheard_lower: Vec<char> = misheard.to_lowercase().chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut index = 0;

    while index < text_chars.len() {
        if matches_phrase_ci(&text_chars, index, &misheard_lower)
            && !is_word_char(at_char(&text_chars, index.wrapping_sub(1)))
            && !is_word_char(at_char(
                &text_chars,
                index.saturating_add(misheard_chars.len()),
            ))
        {
            let after = index + misheard_chars.len();
            if is_arobase_mishearing_context(&text_chars, index, after) {
                out.push_str(replacement);
            } else {
                out.push_str(&text_chars[index..after].iter().collect::<String>());
            }
            index = after;
        } else {
            out.push(text_chars[index]);
            index += 1;
        }
    }

    out
}

fn is_arobase_mishearing_context(
    text_chars: &[char],
    phrase_start: usize,
    phrase_end: usize,
) -> bool {
    if !has_identifier_token_before(text_chars, phrase_start) {
        return false;
    }
    let rest: String = text_chars[phrase_end..].iter().collect();
    let rest_lower = rest.trim_start().to_lowercase();
    if rest_lower.starts_with("de ") {
        return false;
    }
    rest_lower.contains(" point ")
        || rest_lower.contains(" slash ")
        || rest_lower.contains(" barre oblique ")
}

fn has_identifier_token_before(text_chars: &[char], index: usize) -> bool {
    let mut cursor = index;
    while cursor > 0 && text_chars[cursor - 1].is_whitespace() {
        cursor -= 1;
    }
    if cursor == 0 {
        return false;
    }
    while cursor > 0 && is_identifier_char(text_chars[cursor - 1]) {
        cursor -= 1;
    }
    cursor < index
}

/// Converts spoken French punctuation commands still present in text (e.g. « virgule » → `,`).
pub fn interpret_oral_punctuation(text: &str) -> String {
    let mut result = interpret_oral_wrap_commands(text);
    for (phrase, replacement) in ORAL_PUNCTUATION_PHRASES {
        result = replace_phrase_ci(&result, phrase, replacement);
    }
    result = replace_spoken_at(&result);
    result = replace_spoken_hashtag(&result);
    result = replace_spoken_point(&result);
    result = normalize_punctuation_spacing(&result);
    result = collapse_spaced_dots_in_identifiers(&result);
    collapse_technical_symbol_spacing(&result)
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
    while let Some((phrase, open, close)) = find_next_wrap_command(&result) {
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
        .min_by_key(|(phrase, _, _)| {
            find_phrase_range_ci(text, phrase)
                .map(|(start, _)| start)
                .unwrap_or(usize::MAX)
        })
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
    ("signe égal", "="),
    ("signe egal", "="),
    ("signe plus", "+"),
    ("signe moins", "-"),
    ("barre oblique", "/"),
    ("virgule", ","),
    ("apostrophe", "'"),
    ("esperluette", "&"),
    ("pourcent", "%"),
    ("tiret", "-"),
    ("arobase", "@"),
    ("slash", "/"),
];

const HASHTAG_PHRASES: &[&str] = &["hashtag", "dièse", "diese"];

const POINT_DETERMINERS: &[&str] = &[
    "mon", "ton", "son", "ma", "ta", "sa", "mes", "tes", "ses", "notre", "votre", "leur", "leurs",
    "le", "la", "les", "un", "une", "des", "ce", "cet", "cette", "ces", "du", "de",
];

const TAG_STOPWORDS: &[&str] = &[
    "le", "la", "les", "un", "une", "des", "de", "du", "en", "et", "ou", "au", "aux", "pour",
    "par", "sur", "sous", "dans", "avec", "sans", "the", "a", "an", "for", "to", "of",
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

fn replace_spoken_at(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut index = 0;

    while index < chars.len() {
        if matches_phrase_ci(&chars, index, &['a', 't'])
            && !is_word_char(at_char(&chars, index.wrapping_sub(1)))
            && !is_word_char(at_char(&chars, index + 2))
            && is_at_symbol_context(&chars, index + 2)
        {
            out.push('@');
            index += 2;
        } else {
            out.push(chars[index]);
            index += 1;
        }
    }

    out
}

fn is_at_symbol_context(chars: &[char], after_index: usize) -> bool {
    if after_index >= chars.len() {
        return false;
    }
    let rest: String = chars[after_index..].iter().collect();
    let rest_lower = rest.to_lowercase();
    rest_lower.contains(" point ")
        || rest_lower.contains(" slash ")
        || rest_lower.contains(" barre oblique ")
}

fn replace_spoken_hashtag(text: &str) -> String {
    let mut result = text.to_string();
    for phrase in HASHTAG_PHRASES {
        result = replace_phrase_with_context(&result, phrase, "#", is_hashtag_symbol_context);
    }
    result
}

fn replace_spoken_point(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut index = 0;

    while index < chars.len() {
        if matches_phrase_ci(&chars, index, &['p', 'o', 'i', 'n', 't'])
            && !is_word_char(at_char(&chars, index.wrapping_sub(1)))
            && !is_word_char(at_char(&chars, index + 5))
            && !is_point_noun_usage(&chars, index)
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

fn is_point_noun_usage(chars: &[char], point_start: usize) -> bool {
    is_point_noun_phrase(chars, point_start + 5) || has_point_determiner_before(chars, point_start)
}

fn has_point_determiner_before(chars: &[char], point_start: usize) -> bool {
    word_before(chars, point_start)
        .is_some_and(|word| POINT_DETERMINERS.contains(&word.to_ascii_lowercase().as_str()))
}

fn is_hashtag_symbol_context(chars: &[char], phrase_start: usize, phrase_end: usize) -> bool {
    match (
        word_before(chars, phrase_start).as_deref(),
        word_after(chars, phrase_end).as_deref(),
    ) {
        (Some(prev), Some(next)) if is_tag_token(prev) && is_tag_token(next) => true,
        (None, Some(next)) if is_tag_token(next) => true,
        (Some(prev), None) if is_tag_token(prev) => true,
        _ => false,
    }
}

fn is_tag_token(word: &str) -> bool {
    let word = word.trim();
    if word.is_empty()
        || !word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return false;
    }
    !TAG_STOPWORDS.contains(&word.to_ascii_lowercase().as_str())
}

fn word_before(chars: &[char], index: usize) -> Option<String> {
    if index == 0 {
        return None;
    }
    let before: String = chars[..index].iter().collect();
    before
        .split_whitespace()
        .next_back()
        .map(|word| word.to_string())
}

fn word_after(chars: &[char], index: usize) -> Option<String> {
    if index >= chars.len() {
        return None;
    }
    let after: String = chars[index..].iter().collect();
    after.split_whitespace().next().map(|word| word.to_string())
}

fn replace_phrase_with_context(
    text: &str,
    phrase: &str,
    replacement: &str,
    context: fn(&[char], usize, usize) -> bool,
) -> String {
    let phrase_chars: Vec<char> = phrase.chars().collect();
    let phrase_lower: Vec<char> = phrase.to_lowercase().chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut index = 0;

    while index < text_chars.len() {
        let after = index + phrase_chars.len();
        if matches_phrase_ci(&text_chars, index, &phrase_lower)
            && !is_word_char(at_char(&text_chars, index.wrapping_sub(1)))
            && !is_word_char(at_char(&text_chars, after))
            && context(&text_chars, index, after)
        {
            out.push_str(replacement);
            index = after;
        } else {
            out.push(text_chars[index]);
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

/// Removes spaces around technical symbols when they sit between tokens.
fn collapse_technical_symbol_spacing(text: &str) -> String {
    let symbols = ['@', '/', '#', '+', '='];
    symbols.iter().fold(text.to_string(), |current, symbol| {
        collapse_around_symbol(&current, *symbol)
    })
}

fn collapse_around_symbol(text: &str, symbol: char) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == symbol {
            if should_collapse_around_symbol(&out, &chars, index, symbol) {
                while out.ends_with(' ') {
                    out.pop();
                }
                out.push(symbol);
                index += 1;
                while index < chars.len() && chars[index] == ' ' {
                    index += 1;
                }
            } else {
                out.push(symbol);
                index += 1;
            }
        } else {
            out.push(chars[index]);
            index += 1;
        }
    }

    out
}

fn leading_non_space_char(chars: &[char], mut index: usize) -> Option<char> {
    while index < chars.len() && chars[index].is_whitespace() {
        index += 1;
    }
    chars.get(index).copied()
}

fn should_collapse_around_symbol(out: &str, chars: &[char], index: usize, symbol: char) -> bool {
    if symbol == '#' {
        return matches!(
            (
                trailing_word(out).as_deref(),
                leading_word(chars, index + 1).as_deref(),
            ),
            (Some(prev), Some(next)) if is_tag_token(prev) && is_tag_token(next)
        );
    }

    let prev = out.chars().rev().find(|ch| !ch.is_whitespace());
    let next = leading_non_space_char(chars, index + 1);
    let prev_ok = prev.is_some_and(|ch| is_identifier_char(ch) || ch == symbol);
    let next_ok = match next {
        Some(ch) if is_identifier_char(ch) || ch == symbol => true,
        None => prev == Some(symbol),
        _ => false,
    };
    prev_ok && next_ok
}

fn trailing_word(text: &str) -> Option<String> {
    text.split_whitespace().next_back().map(str::to_string)
}

fn leading_word(chars: &[char], mut index: usize) -> Option<String> {
    while index < chars.len() && chars[index].is_whitespace() {
        index += 1;
    }
    if index >= chars.len() {
        return None;
    }
    let rest: String = chars[index..].iter().collect();
    rest.split_whitespace().next().map(str::to_string)
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '-' || ch == '_'
}

/// TLD tokens allowed when collapsing spaced dots in email/URL dictation.
const KNOWN_TLDS: &[&str] = &[
    "com", "fr", "org", "net", "io", "dev", "co", "uk", "de", "eu", "app", "rs", "ts", "js", "md",
    "info", "biz", "me", "tv", "cc",
];

/// French function words that must never be glued after a period.
const DOT_RIGHT_BLOCKWORDS: &[&str] = &[
    "a", "à", "le", "la", "les", "des", "sur", "et", "un", "une", "de", "du", "en", "au", "aux",
    "pour", "par", "avec", "sans", "dans", "ou", "on", "il", "je", "se", "ce", "sa", "son", "ses",
    "leur", "nos", "vos", "qui", "que", "est", "sont", "pas", "mais", "donc", "car", "ni", "ne",
    "the", "a", "an", "to", "of", "in", "on", "at", "by", "for",
];

/// Common French words that must not form fake domains with a TLD (e.g. « comme . com »).
const DOT_LEFT_BLOCKWORDS: &[&str] = &[
    "comme",
    "partie",
    "statistique",
    "affichage",
    "graphique",
    "sections",
    "problemes",
    "problèmes",
    "largeur",
    "barre",
    "attence",
    "latence",
    "donc",
    "alors",
    "aussi",
    "encore",
    "tres",
    "très",
    "bien",
    "tout",
    "tous",
    "toute",
    "cette",
    "cela",
    "ceci",
    "notre",
    "votre",
];

fn is_domain_label(token: &str) -> bool {
    let token = token.trim();
    token.len() >= 2
        && token.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && token.chars().any(|c| c.is_ascii_alphabetic())
}

/// Whether spaced or glued dots between two tokens should collapse (e.g. gmail . com).
fn should_collapse_dot(left: &str, right: &str, preceding: &str) -> bool {
    let left = left.trim();
    let right = right.trim();
    if left.is_empty() || right.is_empty() {
        return false;
    }

    let left_lower = left.to_ascii_lowercase();
    let right_lower = right.to_ascii_lowercase();

    if DOT_RIGHT_BLOCKWORDS.contains(&right_lower.as_str()) {
        return false;
    }

    if right.chars().all(|c| c.is_ascii_digit()) {
        return left.chars().last().is_some_and(|c| c.is_ascii_digit())
            || (left_lower.starts_with('v') && left.len() <= 4);
    }

    if preceding.contains('@') {
        return is_domain_label(left)
            && (KNOWN_TLDS.contains(&right_lower.as_str()) || is_domain_label(right));
    }

    if KNOWN_TLDS.contains(&right_lower.as_str()) {
        return is_domain_label(left) && !DOT_LEFT_BLOCKWORDS.contains(&left_lower.as_str());
    }

    false
}

fn token_before_dot(chars: &[char], dot_index: usize) -> Option<String> {
    let mut end = dot_index;
    while end > 0 && chars[end - 1].is_whitespace() {
        end -= 1;
    }
    if end == 0 || !is_identifier_char(chars[end - 1]) {
        return None;
    }
    let mut start = end;
    while start > 0 && is_identifier_char(chars[start - 1]) {
        start -= 1;
    }
    Some(chars[start..end].iter().collect())
}

fn token_after_dot(chars: &[char], dot_index: usize) -> Option<String> {
    let mut start = dot_index + 1;
    while start < chars.len() && chars[start].is_whitespace() {
        start += 1;
    }
    if start >= chars.len() || !is_identifier_char(chars[start]) {
        return None;
    }
    let mut end = start + 1;
    while end < chars.len() && is_identifier_char(chars[end]) {
        end += 1;
    }
    Some(chars[start..end].iter().collect())
}

/// Collapses spaced dots between identifier tokens (e.g. « gmail . com » → gmail.com).
fn collapse_spaced_dots_in_identifiers(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == '.'
            && index > 0
            && index + 1 < chars.len()
            && is_identifier_char(chars[index - 1])
        {
            if let (Some(left), Some(right)) = (
                token_before_dot(&chars, index),
                token_after_dot(&chars, index),
            ) {
                let preceding: String = out.chars().collect();
                if should_collapse_dot(&left, &right, &preceding) {
                    while out.ends_with(' ') {
                        out.pop();
                    }
                    out.push('.');
                    let mut right_start = index + 1;
                    while right_start < chars.len() && chars[right_start].is_whitespace() {
                        right_start += 1;
                    }
                    index = right_start;
                    continue;
                }
            }
        }

        out.push(chars[index]);
        index += 1;
    }

    out
}

fn normalize_punctuation_spacing(text: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];
        if matches!(ch, ',' | ';' | ':' | '.' | '!' | '?' | '…') && out.ends_with(' ') {
            out.pop();
        }

        if matches!(ch, ')' | '»' | '!' | '?' | '.' | ',' | ';' | ':' | '…') && out.ends_with(' ')
        {
            out.pop();
        }

        out.push(ch);

        if ch == '(' || ch == '«' {
            if index + 1 < chars.len() && chars[index + 1] == ' ' {
                index += 1;
            }
        } else if ch == '.'
            && index + 1 < chars.len()
            && is_identifier_char(chars[index + 1])
            && out.chars().next_back().is_some_and(is_identifier_char)
        {
            if let (Some(left), Some(right)) = (
                token_before_dot(&chars, index),
                token_after_dot(&chars, index),
            ) {
                if !should_collapse_dot(&left, &right, &out) {
                    out.push(' ');
                }
            }
        } else if matches!(ch, ',' | ';' | ':' | '.' | '!' | '?' | '…')
            && index + 1 < chars.len()
            && chars[index + 1] != ' '
            && chars[index + 1] != '\n'
            && !matches!(
                chars[index + 1],
                ')' | '»' | '!' | '?' | '.' | ',' | ';' | ':' | '…'
            )
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
        assert_eq!(out, "Hmm still thinking");
    }

    #[test]
    fn prefers_text_after_last_thinking_block() {
        let raw_output = format!("{THINK_OPEN}plan{THINK_CLOSE}Bonjour.");
        let out = validate_cleanup_output("test", &raw_output).unwrap();
        assert_eq!(out, "Bonjour.");
    }

    #[test]
    fn converts_spoken_comma_and_question_mark() {
        let out =
            interpret_oral_punctuation("Bonjour virgule comment allez-vous point d'interrogation");
        assert_eq!(out, "Bonjour, comment allez-vous?");
    }

    #[test]
    fn converts_spoken_exclamation_and_semicolon() {
        let out =
            interpret_oral_punctuation("Attention point virgule c'est urgent point d'exclamation");
        assert_eq!(out, "Attention; c'est urgent!");
    }

    #[test]
    fn preserves_point_in_noun_phrase() {
        let out = interpret_oral_punctuation("Le point de vue est clair");
        assert_eq!(out, "Le point de vue est clair");
    }

    #[test]
    fn preserves_point_noun_at_end_of_utterance() {
        let out = post_process_transcript("voici mon point");
        assert_eq!(out, "Voici mon point");

        let out = post_process_transcript("bonjour point");
        assert_eq!(out, "Bonjour.");
    }

    #[test]
    fn join_transcript_segments_lowercases_mid_sentence_capital() {
        let joined = join_transcript_segments(&["de voir de...", "Pour faire un test"]);
        assert_eq!(joined, "de voir de... pour faire un test");
    }

    #[test]
    fn join_transcript_segments_keeps_capital_after_sentence_end() {
        let joined = join_transcript_segments(&["Première phrase.", "Deuxième phrase"]);
        assert_eq!(joined, "Première phrase. Deuxième phrase");
    }

    #[test]
    fn strips_leading_oral_fillers() {
        let out = post_process_transcript("ok alors là je teste");
        assert_eq!(out, "Je teste");
    }

    #[test]
    fn fixes_malformed_trailing_punctuation() {
        let out = post_process_transcript("assez rapidement (, Calliop.)");
        assert_eq!(out, "Assez rapidement Calliop.");
    }

    #[test]
    fn preserves_hashtag_word_in_prose() {
        let out = interpret_oral_punctuation("le hashtag pour rust");
        assert_eq!(out, "le hashtag pour rust");
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
    fn normalizes_whisper_arobase_mishearings() {
        let out = post_process_transcript("contact arrobase gmail point com");
        assert_eq!(out, "contact@gmail.com");

        let out = post_process_transcript("contact a base gmail point com");
        assert_eq!(out, "contact@gmail.com");

        let out = post_process_transcript("contact à Robase gmail point com");
        assert_eq!(out, "contact@gmail.com");

        let out = normalize_stt_oral_mishearings("recette à base de riz");
        assert_eq!(out, "recette à base de riz");
    }

    #[test]
    fn whisper_hint_covers_oral_symbols() {
        assert!(WHISPER_ORAL_VOCABULARY_HINT.contains("arobase"));
        assert!(WHISPER_ORAL_VOCABULARY_HINT.contains("slash"));
        assert!(whisper_oral_vocabulary_word_count() > 0);
    }

    #[test]
    fn converts_spoken_at_and_slash() {
        let out = interpret_oral_punctuation("contact at gmail point com");
        assert_eq!(out, "contact@gmail.com");

        let out = interpret_oral_punctuation("ouvre src slash lib slash main point rs");
        assert_eq!(out, "ouvre src/lib/main.rs");
    }

    #[test]
    fn preserves_english_at_preposition_outside_email_context() {
        let out = interpret_oral_punctuation("look at this");
        assert_eq!(out, "look at this");
    }

    #[test]
    fn preserves_french_egale_verb() {
        let out = interpret_oral_punctuation("deux plus deux egale quatre");
        assert_eq!(out, "deux plus deux egale quatre");
    }

    #[test]
    fn converts_signe_egal_phrase() {
        let out = interpret_oral_punctuation("résultat signe egal dix");
        assert!(out.contains('='));
        assert!(!out.to_lowercase().contains("signe"));
    }

    #[test]
    fn preserves_a_base_outside_email_context() {
        let out = normalize_stt_oral_mishearings("recette à base de riz");
        assert_eq!(out, "recette à base de riz");

        let out = normalize_stt_oral_mishearings("je suis à base");
        assert_eq!(out, "je suis à base");
    }

    #[test]
    fn does_not_collapse_symbols_without_identifier_neighbors() {
        let out = interpret_oral_punctuation("arobase gmail");
        assert_eq!(out, "@ gmail");
    }

    #[test]
    fn converts_arobase_and_barre_oblique() {
        let out = interpret_oral_punctuation("mon mail arobase example point fr");
        assert_eq!(out, "mon mail@example.fr");
    }

    #[test]
    fn converts_signe_plus_and_hashtag() {
        let out = interpret_oral_punctuation("version signe plus signe plus");
        assert_eq!(out, "version++");

        let out = interpret_oral_punctuation("tag hashtag rust");
        assert_eq!(out, "tag#rust");
    }

    #[test]
    fn system_prompt_covers_oral_punctuation_commands() {
        assert!(SYSTEM_PROMPT.contains("virgule"));
        assert!(SYSTEM_PROMPT.contains("point d'interrogation"));
        assert!(SYSTEM_PROMPT.contains("point d'exclamation"));
        assert!(SYSTEM_PROMPT.contains("entre parenthèses"));
        assert!(SYSTEM_PROMPT.contains("entre guillemets"));
        assert!(SYSTEM_PROMPT.contains("arobase"));
        assert!(SYSTEM_PROMPT.contains("slash"));
        assert!(SYSTEM_PROMPT.contains("ne doivent jamais"));
    }

    #[test]
    fn fallback_template_targets_qwen3_chatml() {
        assert!(QWEN3_CHAT_TEMPLATE.contains("<|im_start|>"));
        assert!(QWEN3_CHAT_TEMPLATE.contains("add_generation_prompt"));
        assert!(QWEN3_CHAT_TEMPLATE.contains(concat!("<|", "im_end", "|>")));
    }

    #[test]
    fn default_system_prompt_matches_base() {
        assert_eq!(build_system_prompt(ToneProfile::Default), SYSTEM_PROMPT);
    }

    #[test]
    fn tone_profiles_extend_system_prompt() {
        let casual = build_system_prompt(ToneProfile::Casual);
        assert!(casual.starts_with(SYSTEM_PROMPT));
        assert!(casual.contains("Slack"));

        let formal = build_system_prompt(ToneProfile::Formal);
        assert!(formal.contains("courriel"));

        let technical = build_system_prompt(ToneProfile::Technical);
        assert!(technical.contains("développement"));
    }

    #[test]
    fn tone_profile_roundtrip_serde() {
        let json = serde_json::to_string(&ToneProfile::Formal).unwrap();
        assert_eq!(json, "\"formal\"");
        let parsed: ToneProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ToneProfile::Formal);
    }

    #[test]
    fn does_not_collapse_dot_before_french_article() {
        let out = post_process_transcript("partie statistique point a des problèmes");
        assert_eq!(out, "Partie statistique. a des problèmes");
        assert!(!out.contains("statistique.a"));
    }

    #[test]
    fn does_not_collapse_dot_before_preposition() {
        let out = post_process_transcript("problèmes d'affichage point sur la largeur");
        assert_eq!(out, "Problèmes d'affichage. sur la largeur");
        assert!(!out.contains("affichage.sur"));
    }

    #[test]
    fn does_not_form_fake_domain_from_common_word_and_tld() {
        let out = interpret_oral_punctuation("comme point com");
        assert_eq!(out, "comme. com");
    }

    #[test]
    fn splits_glued_punctuation_before_word() {
        let out = interpret_oral_punctuation("affichage.sur la largeur");
        assert_eq!(out, "affichage. sur la largeur");
    }

    #[test]
    fn splits_glued_punctuation_before_digit() {
        let out = interpret_oral_punctuation("bar affiché.3 sections");
        assert_eq!(out, "bar affiché. 3 sections");
    }

    #[test]
    fn keeps_version_number_dots() {
        let out = interpret_oral_punctuation("version v1.2");
        assert_eq!(out, "version v1.2");
    }

    #[test]
    fn still_collapses_email_domain_dots() {
        let out = post_process_transcript("contact arobase gmail point com");
        assert_eq!(out, "contact@gmail.com");
    }
}
