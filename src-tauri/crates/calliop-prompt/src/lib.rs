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
Quand l'utilisateur se corrige (« en fait », « non plutôt », « pardon », « je veux dire », \
« non attends », reformulation), ne garde que la version finale — \
ex. « on se voit à 14h en fait 15h » → « On se voit à 15h. » ; \
« rendez-vous à 2 heures non plutôt 3 heures » → « Rendez-vous à 3 heures. ». \
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
Si l'utilisateur dicte une énumération explicite (numéros, « premièrement/deuxièmement », « tiret » répété), \
formate en liste numérotée ou à puces — un item par ligne. \
Si l'utilisateur énumère implicitement plusieurs éléments sans numéros ni « tiret » \
(après « pour », « avec », « comprenant », « : », ou une série de noms séparés par des pauses), \
formate aussi en liste — ne te contente jamais de virgules. \
Si l'utilisateur dicte une liste de tâches ou de features séparées par des virgules \
(instructions impératives courtes), formate en liste numérotée — un item par ligne. \
Ex. « aller au magasin pour 1 pommes 2 bananes 3 oranges » → \
« Aller au magasin pour :\n1. Pommes\n2. Bananes\n3. Oranges » ; \
« aller au magasin pour pommes bananes oranges » → \
« Aller au magasin pour :\n1. Pommes\n2. Bananes\n3. Oranges » ; \
« courses : lait oeufs pain beurre » → \
« Courses :\n1. Lait\n2. Oeufs\n3. Pain\n4. Beurre » ; \
« ajouter un bouton, mettre un fond blanc, clignoter la page » → \
« 1. Ajouter un bouton\n2. Mettre un fond blanc\n3. Clignoter la page ». \
Si le texte contient déjà une liste formatée (lignes « 1. … », « - … », sauts de ligne), \
conserve sa structure — ne fusionne pas les items sur une seule ligne et ne change pas le nombre. \
Ne commente pas, ne pose pas de questions. \
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
    let hint = cleanup_user_hint(raw);
    Ok(format!(
        "/no_think\nTranscription brute à nettoyer :\n{raw}{hint}"
    ))
}

fn cleanup_user_hint(raw: &str) -> &'static str {
    if raw.contains('\n') && raw.lines().any(looks_like_list_line) {
        return "\nConserve la structure de liste (sauts de ligne, numéros ou tirets).\n";
    }
    if looks_like_implicit_enumeration(raw) {
        return "\nÉnumération détectée : formate en liste numérotée (un item par ligne), pas en virgules.\n";
    }
    ""
}

pub fn looks_like_implicit_enumeration(text: &str) -> bool {
    try_format_implicit_noun_list(text).is_some() || try_format_comma_separated_list(text).is_some()
}

fn looks_like_list_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("- ")
        || trimmed.starts_with("* ")
        || trimmed.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

pub fn validate_cleanup_output(raw: &str, cleaned: &str) -> Result<String, PromptError> {
    let mut cleaned = strip_thinking_blocks(cleaned.trim());

    cleaned = strip_wrapping_quotes(&cleaned);
    cleaned = interpret_oral_punctuation(&cleaned);
    cleaned = polish_llm_output(&cleaned);
    cleaned = prefer_list_structure_over_comma_flattening(raw, &cleaned);

    if cleaned.trim().is_empty() {
        let fallback = post_process_transcript(raw);
        if fallback.trim().is_empty() {
            return Err(PromptError::EmptyOutput);
        }
        cleaned = fallback;
    }

    let max_len = raw.len().saturating_mul(3).max(512);
    if cleaned.len() > max_len {
        return Err(PromptError::OutputTooLong);
    }

    Ok(cleaned)
}

fn prefer_list_structure_over_comma_flattening(raw: &str, cleaned: &str) -> String {
    if raw.contains('\n') && !cleaned.contains('\n') {
        let list_lines = raw
            .lines()
            .filter(|line| looks_like_list_line(line))
            .count();
        if list_lines >= 2 {
            return raw.to_string();
        }
    }

    if !cleaned.contains('\n') {
        if let Some(recovered) = try_recover_merged_numbered_list_line(cleaned) {
            return recovered;
        }
    }

    if let Some(formatted) = try_format_implicit_noun_list(raw) {
        if formatted.contains('\n') && !cleaned.contains('\n') {
            return formatted;
        }
    }

    if let Some(formatted) = try_format_comma_separated_list(raw) {
        if formatted.contains('\n') && !cleaned.contains('\n') {
            return formatted;
        }
    }

    cleaned.to_string()
}

/// LLMs often keep only the first list marker and merge items with commas on one line.
fn try_recover_merged_numbered_list_line(text: &str) -> Option<String> {
    if text.contains('\n') {
        return None;
    }

    let trimmed = text.trim();
    let dot_space = trimmed.find(". ")?;
    let prefix = trimmed[..dot_space].trim();
    if prefix.is_empty() || !prefix.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }

    let remainder = trimmed[dot_space + 2..].trim();
    try_format_comma_separated_list(remainder)
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

/// Pause below this threshold inserts a comma between segments (when Whisper omitted punctuation).
pub const PAUSE_COMMA_THRESHOLD_MS: u32 = 700;

/// Pause at or above this threshold inserts a period between segments.
pub const PAUSE_PERIOD_THRESHOLD_MS: u32 = 700;

/// Pause before a segment that follows a sentence end — candidate for pipelined LLM freeze.
pub const FROZEN_BOUNDARY_PAUSE_MS: u32 = 1500;

/// Sidecar context window (`calliop-llm-worker`).
pub const LLM_CLEANUP_CONTEXT_TOKENS: u32 = 2048;

/// Conservative input budget for the user transcript (system prompt + template overhead reserved).
pub const LLM_CLEANUP_INPUT_TOKEN_BUDGET: usize = 1200;

/// Rough token estimate for cleanup budgeting (French-heavy text ≈ 3 chars/token).
pub fn estimate_cleanup_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(3)
}

/// Whether `raw` transcript text fits in one sidecar cleanup request.
pub fn fits_llm_cleanup_budget(raw: &str) -> bool {
    const TEMPLATE_OVERHEAD_TOKENS: usize = 150;
    estimate_cleanup_tokens(raw).saturating_add(TEMPLATE_OVERHEAD_TOKENS)
        <= LLM_CLEANUP_INPUT_TOKEN_BUDGET
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

/// Joins streaming segments using VAD pause duration when Whisper omitted punctuation.
pub fn join_transcript_segments_with_pauses(segments: &[(impl AsRef<str>, u32)]) -> String {
    let mut result = String::new();
    for (index, (segment, leading_silence_ms)) in segments.iter().enumerate() {
        let segment = segment.as_ref().trim();
        if segment.is_empty() {
            continue;
        }
        if result.is_empty() {
            result = segment.to_string();
            continue;
        }

        if !segment_has_trailing_punctuation(&result) {
            if *leading_silence_ms >= PAUSE_PERIOD_THRESHOLD_MS {
                append_sentence_break(&mut result);
            } else if *leading_silence_ms > 0 {
                append_comma_break(&mut result);
            } else if !result.ends_with(' ') {
                result.push(' ');
            }
        } else if !result.ends_with(' ') {
            result.push(' ');
        }

        if should_lowercase_segment_join(&result, segment) {
            result.push_str(&lowercase_first_char(segment));
        } else if ends_with_sentence_punctuation(&result)
            && segment
                .chars()
                .next()
                .is_some_and(|ch| ch.is_lowercase() && ch.is_alphabetic())
        {
            result.push_str(&capitalize_first_char(segment));
        } else {
            result.push_str(segment);
        }

        let _ = index;
    }
    result
}

/// Index of the last segment included in the latest frozen prefix, if any.
pub fn find_latest_frozen_boundary(segments: &[(impl AsRef<str>, u32)]) -> Option<usize> {
    if segments.len() < 2 {
        return None;
    }
    let mut latest = None;
    for index in 1..segments.len() {
        if segments[index].1 < FROZEN_BOUNDARY_PAUSE_MS {
            continue;
        }
        let prefix: Vec<(String, u32)> = segments[..=index - 1]
            .iter()
            .map(|(text, pause)| (text.as_ref().to_string(), *pause))
            .collect();
        let joined = join_transcript_segments_with_pauses(&prefix);
        if ends_with_sentence_punctuation(&joined) {
            latest = Some(index - 1);
        }
    }
    latest
}

fn segment_has_trailing_punctuation(text: &str) -> bool {
    let trimmed = text.trim_end();
    trimmed.ends_with(['.', '!', '?', '…', ',', ';', ':'])
}

fn ends_with_sentence_punctuation(text: &str) -> bool {
    let trimmed = text.trim_end();
    trimmed.ends_with(['.', '!', '?', '…'])
}

fn append_comma_break(result: &mut String) {
    while result.ends_with(' ') {
        result.pop();
    }
    if !result.ends_with(',') {
        result.push(',');
    }
    result.push(' ');
}

fn append_sentence_break(result: &mut String) {
    while result.ends_with(' ') {
        result.pop();
    }
    if !ends_with_sentence_punctuation(result) {
        result.push('.');
    }
    result.push(' ');
}

fn capitalize_first_char(text: &str) -> String {
    let trimmed = text.trim();
    let mut chars = trimmed.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut out = first.to_uppercase().to_string();
            out.extend(chars);
            out
        }
    }
}

/// Full transcript cleanup after STT: fix mishearings, oral punctuation, then polish.
pub fn post_process_transcript(text: &str) -> String {
    let text = normalize_stt_oral_mishearings(text);
    let text = format_spoken_lists(&text);
    let text = interpret_oral_punctuation(&text);
    // Second pass: oral « virgule » is converted to commas only after the punctuation pass.
    let text = format_spoken_lists(&text);
    polish_transcript(&text)
}

/// Deterministic polish applied after STT (includes filler stripping).
fn polish_transcript(text: &str) -> String {
    let text = fix_malformed_punctuation(text);
    let text = strip_leading_oral_fillers(&text);
    let text = strip_inline_fillers(&text);
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

/// Hesitation tokens removable anywhere in the utterance (word-boundary isolated).
const INLINE_HESITATION_FILLERS: &[&str] = &[
    "euh", "heu", "euhm", "hum", "hem", "hmm", "bah", "ben", "beh", "um", "uh", "uhm",
];

fn strip_inline_fillers(text: &str) -> String {
    let mut result = text.to_string();
    for filler in INLINE_HESITATION_FILLERS {
        result = remove_isolated_word_ci(&result, filler);
    }
    cleanup_orphan_commas(&result)
}

fn remove_isolated_word_ci(text: &str, word: &str) -> String {
    let word_chars: Vec<char> = word.chars().collect();
    let word_lower: Vec<char> = word.to_lowercase().chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut index = 0;

    while index < text_chars.len() {
        if matches_phrase_ci(&text_chars, index, &word_lower)
            && !is_word_char(at_char(&text_chars, index.wrapping_sub(1)))
            && !is_word_char(at_char(&text_chars, index.saturating_add(word_chars.len())))
        {
            index += word_chars.len();
            while index < text_chars.len() && text_chars[index].is_whitespace() {
                index += 1;
            }
        } else {
            out.push(text_chars[index]);
            index += 1;
        }
    }

    out
}

fn cleanup_orphan_commas(text: &str) -> String {
    let mut result = text.to_string();
    for broken in [", ,", ",,", " ,", ",  ,"] {
        while result.contains(broken) {
            result = result.replace(broken, ", ");
        }
    }
    result = result.replace(" ,", ",");
    result.trim().to_string()
}

/// Converts spoken enumerations into formatted lists before oral punctuation runs.
pub fn format_spoken_lists(text: &str) -> String {
    if let Some(formatted) = try_format_bullet_list(text) {
        return formatted;
    }
    if let Some(formatted) = try_format_numbered_list(text) {
        return formatted;
    }
    if let Some(formatted) = try_format_comma_separated_list(text) {
        return formatted;
    }
    try_format_implicit_noun_list(text).unwrap_or_else(|| text.to_string())
}

const COMMA_LIST_IMPERATIVE_STARTERS: &[&str] = &[
    "ajouter",
    "ajoute",
    "ajoutez",
    "mettre",
    "mets",
    "mettez",
    "supprimer",
    "supprime",
    "supprimez",
    "créer",
    "creer",
    "crée",
    "cree",
    "créez",
    "creez",
    "modifier",
    "modifie",
    "modifiez",
    "changer",
    "change",
    "changez",
    "clignoter",
    "clignote",
    "clignotez",
    "afficher",
    "affiche",
    "affichez",
    "masquer",
    "cache",
    "cacher",
    "cachez",
    "ouvrir",
    "ouvre",
    "ouvrez",
    "fermer",
    "ferme",
    "fermez",
    "déplacer",
    "deplacer",
    "déplace",
    "deplace",
    "déplacez",
    "deplacez",
    "activer",
    "active",
    "activez",
    "désactiver",
    "desactiver",
    "désactive",
    "desactive",
    "désactivez",
    "desactivez",
    "augmenter",
    "augmente",
    "augmentez",
    "réduire",
    "reduire",
    "réduis",
    "reduis",
    "réduisez",
    "reduisez",
    "centrer",
    "centre",
    "centrez",
    "aligner",
    "aligne",
    "alignez",
    "colorier",
    "colorie",
    "coloriez",
    "passer",
    "passe",
    "passez",
    "retirer",
    "retire",
    "retirez",
    "enlever",
    "enlève",
    "enleve",
    "enlevez",
    "inclure",
    "inclus",
    "incluez",
    "exclure",
    "exclus",
    "excluez",
    "remplacer",
    "remplace",
    "remplacez",
    "implémenter",
    "implementer",
    "implémente",
    "implemente",
    "corriger",
    "corrige",
    "corrigez",
    "utiliser",
    "utilise",
    "utilisez",
    "rendre",
    "rend",
    "permettre",
    "permet",
    "faciliter",
    "facilite",
    "add",
    "remove",
    "set",
    "show",
    "hide",
    "make",
    "enable",
    "disable",
    "update",
    "fix",
    "move",
    "open",
    "close",
    "toggle",
    "implement",
];

const COMMA_LIST_PROSE_STARTERS: &[&str] = &[
    "je",
    "tu",
    "il",
    "elle",
    "on",
    "nous",
    "vous",
    "ils",
    "elles",
    "ce",
    "c'",
    "c'est",
    "bonjour",
    "salut",
    "merci",
    "comment",
    "pourquoi",
    "est-ce",
    "peut-être",
    "peut",
    "puis",
    "ensuite",
    "car",
    "parce",
    "mais",
    "donc",
    "or",
    "ni",
    "when",
    "if",
    "the",
    "and",
    "but",
    "so",
    "because",
];

fn try_format_comma_separated_list(text: &str) -> Option<String> {
    if text.contains('\n') {
        return None;
    }

    let segments = split_comma_list_segments(text);
    if segments.len() < 2 || segments.len() > 12 {
        return None;
    }
    if !comma_separated_segments_are_list(&segments) {
        return None;
    }

    let numbered_items: Vec<String> = segments
        .iter()
        .enumerate()
        .map(|(index, segment)| format!("{}. {}", index + 1, capitalize_first_char(segment)))
        .collect();
    Some(numbered_items.join("\n"))
}

fn split_comma_list_segments(text: &str) -> Vec<String> {
    let normalized = replace_phrase_ci(text, " virgule ", ", ");
    normalized
        .split(", ")
        .map(trim_list_segment)
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn trim_list_segment(segment: &str) -> String {
    segment
        .trim()
        .trim_end_matches(['.', '!', '?'])
        .trim()
        .to_string()
}

fn comma_list_first_word(segment: &str) -> Option<String> {
    segment.split_whitespace().next().map(|word| {
        word.trim_end_matches(['.', '!', '?', ',', ';', ':'])
            .to_ascii_lowercase()
    })
}

fn comma_separated_segments_are_list(segments: &[String]) -> bool {
    for segment in segments {
        if segment.is_empty() || segment.contains('?') {
            return false;
        }
        if segment.split_whitespace().count() > 12 {
            return false;
        }
    }

    let imperative_count = segments
        .iter()
        .filter(|segment| {
            comma_list_first_word(segment)
                .is_some_and(|word| COMMA_LIST_IMPERATIVE_STARTERS.contains(&word.as_str()))
        })
        .count();
    if imperative_count >= 2 {
        return true;
    }

    segments.len() >= 3
        && segments.iter().all(|segment| {
            let words = segment.split_whitespace().count();
            (1..=4).contains(&words)
        })
        && segments.iter().all(|segment| {
            comma_list_first_word(segment)
                .is_none_or(|word| !COMMA_LIST_PROSE_STARTERS.contains(&word.as_str()))
        })
}

const IMPLICIT_LIST_INTRO_PHRASES: &[&str] =
    &[" pour ", " avec ", " comprenant ", " incluant ", " : "];

const IMPLICIT_LIST_ITEM_BLOCKWORDS: &[&str] = &[
    "et",
    "ou",
    "mais",
    "donc",
    "puis",
    "aussi",
    "encore",
    "très",
    "tres",
    "demain",
    "aujourd'hui",
    "aujourdhui",
    "hier",
    "maintenant",
    "toujours",
    "jamais",
    "bien",
    "mal",
    "vite",
    "peu",
    "plus",
    "moins",
    "comme",
    "chez",
    "dans",
    "sur",
    "sous",
    "sans",
    "pour",
    "avec",
    "the",
    "and",
    "or",
    "de",
    "du",
    "des",
    "le",
    "la",
    "les",
    "un",
    "une",
    "ce",
    "cette",
    "mon",
    "ton",
    "son",
    "mes",
    "tes",
    "ses",
    "notre",
    "votre",
    "leur",
    "very",
    "really",
];

const IMPLICIT_LIST_VERB_BLOCKWORDS: &[&str] = &[
    "acheter", "aller", "venir", "faire", "être", "etre", "avoir", "courir", "marcher", "parler",
    "dire", "voir", "prendre", "mettre", "passer", "devoir", "pouvoir", "vouloir", "savoir",
    "falloir", "donner", "trouver", "demander", "rester",
];

fn try_format_implicit_noun_list(text: &str) -> Option<String> {
    if text.contains('\n') || text.contains(',') || text.contains(';') {
        return None;
    }

    let lower = text.to_lowercase();
    let (trigger_pos, trigger_len) = IMPLICIT_LIST_INTRO_PHRASES
        .iter()
        .filter_map(|phrase| lower.rfind(phrase).map(|pos| (pos, phrase.len())))
        .max_by_key(|(pos, _)| *pos)?;

    let intro = text[..trigger_pos + trigger_len].trim_end();
    let remainder = text[trigger_pos + trigger_len..].trim();
    if remainder.is_empty() {
        return None;
    }

    let item_tokens = split_implicit_list_items(remainder)?;
    if !implicit_list_items_are_valid(&item_tokens) {
        return None;
    }

    let numbered_items: Vec<String> = item_tokens
        .iter()
        .enumerate()
        .map(|(index, item)| format!("{}. {}", index + 1, capitalize_first_char(item)))
        .collect();
    Some(format_list_output(
        intro,
        &numbered_items,
        ListStyle::Numbered,
    ))
}

fn split_implicit_list_items(remainder: &str) -> Option<Vec<String>> {
    let parts: Vec<&str> = if remainder.contains(" et ") {
        remainder.split(" et ").collect()
    } else {
        remainder.split_whitespace().collect()
    };

    let items: Vec<String> = parts
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect();

    if items.len() < 3 || items.len() > 12 {
        return None;
    }
    Some(items)
}

fn implicit_list_items_are_valid(items: &[String]) -> bool {
    items.iter().all(|item| {
        let words: Vec<&str> = item.split_whitespace().collect();
        if words.is_empty() || words.len() > 4 {
            return false;
        }
        words.iter().all(|word| {
            let lower = word.to_ascii_lowercase();
            if lower.len() < 2 {
                return false;
            }
            if IMPLICIT_LIST_ITEM_BLOCKWORDS.contains(&lower.as_str()) {
                return false;
            }
            if IMPLICIT_LIST_VERB_BLOCKWORDS.contains(&lower.as_str()) {
                return false;
            }
            !word
                .chars()
                .any(|ch| matches!(ch, '.' | '!' | '?' | ',' | ';' | ':'))
        })
    })
}

fn try_format_bullet_list(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let marker = " tiret ";
    let mut positions = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = lower[search_from..].find(marker) {
        let start = search_from + rel;
        positions.push(start);
        search_from = start + marker.len();
    }
    if positions.len() < 2 {
        return None;
    }

    let first = positions[0];
    let intro = text[..first].trim_end();
    let mut items = Vec::new();
    for (index, pos) in positions.iter().enumerate() {
        let content_start = pos + marker.len();
        let content_end = positions.get(index + 1).copied().unwrap_or(text.len());
        let item = text[content_start..content_end].trim();
        if !item.is_empty() {
            items.push(capitalize_first_char(item));
        }
    }
    if items.len() < 2 {
        return None;
    }

    Some(format_list_output(intro, &items, ListStyle::Bullet))
}

fn try_format_numbered_list(text: &str) -> Option<String> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.len() < 4 {
        return None;
    }

    let mut markers = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        if let Some(number) = parse_list_marker(tokens[index], tokens.get(index + 1)) {
            markers.push((index, number));
            index += marker_token_span(tokens[index], tokens.get(index + 1));
            continue;
        }
        index += 1;
    }

    if markers.len() < 2 {
        return None;
    }

    for window in markers.windows(2) {
        if window[1].1 != window[0].1 + 1 {
            return None;
        }
    }

    let intro_end = markers[0].0;
    let intro = tokens[..intro_end].join(" ");
    let mut items = Vec::new();
    for (marker_index, (token_index, number)) in markers.iter().enumerate() {
        let content_start =
            token_index + marker_token_span(tokens[*token_index], tokens.get(token_index + 1));
        let content_end = markers
            .get(marker_index + 1)
            .map(|(next_index, _)| *next_index)
            .unwrap_or(tokens.len());
        if content_start >= content_end {
            continue;
        }
        let item = tokens[content_start..content_end].join(" ");
        if !item.is_empty() {
            items.push((*number, capitalize_first_char(&item)));
        }
    }

    if items.len() < 2 {
        return None;
    }

    Some(format_list_output(
        &intro,
        &items
            .iter()
            .map(|(number, item)| format!("{number}. {item}"))
            .collect::<Vec<_>>(),
        ListStyle::Numbered,
    ))
}

enum ListStyle {
    Bullet,
    Numbered,
}

fn format_list_output(intro: &str, items: &[String], style: ListStyle) -> String {
    let intro = intro.trim();
    let mut lines = Vec::new();
    if !intro.is_empty() {
        let intro_line = if intro.ends_with(':') {
            capitalize_first_char(intro)
        } else {
            format!("{} :", capitalize_first_char(intro))
        };
        lines.push(intro_line);
    }

    for item in items {
        match style {
            ListStyle::Bullet => lines.push(format!("- {item}")),
            ListStyle::Numbered => lines.push(item.clone()),
        }
    }

    lines.join("\n")
}

fn marker_token_span(current: &str, next: Option<&&str>) -> usize {
    let current_lower = current.to_ascii_lowercase();
    if current_lower == "numéro" || current_lower == "numero" {
        return if next.is_some() { 2 } else { 1 };
    }
    if matches!(
        current_lower.as_str(),
        "premièrement"
            | "premierement"
            | "deuxièmement"
            | "deuxiemement"
            | "troisièmement"
            | "troisiemement"
            | "quatrièmement"
            | "quatriemement"
            | "cinquièmement"
            | "cinquiemement"
    ) {
        return 1;
    }
    if current.ends_with('.') || current.parse::<u32>().is_ok() {
        return 1;
    }
    if current_lower == "point" {
        return if next.is_some() { 2 } else { 1 };
    }
    1
}

fn parse_list_marker(token: &str, next: Option<&&str>) -> Option<u32> {
    let token_lower = token.to_ascii_lowercase();
    if let Ok(number) = token.trim_end_matches('.').parse::<u32>() {
        return Some(number);
    }
    if token_lower == "numéro" || token_lower == "numero" {
        return next.and_then(|word| parse_spoken_cardinal(word));
    }
    if token_lower == "point" {
        return next.and_then(|word| parse_spoken_cardinal(word));
    }
    parse_ordinal_marker(&token_lower)
}

fn parse_ordinal_marker(token: &str) -> Option<u32> {
    match token {
        "premièrement" | "premierement" | "premier" | "premiere" => Some(1),
        "deuxièmement" | "deuxiemement" | "deuxième" | "deuxieme" => Some(2),
        "troisièmement" | "troisiemement" | "troisième" | "troisieme" => Some(3),
        "quatrièmement" | "quatriemement" | "quatrième" | "quatrieme" => Some(4),
        "cinquièmement" | "cinquiemement" | "cinquième" | "cinquieme" => Some(5),
        "sixièmement" | "sixiemement" | "sixième" | "sixieme" => Some(6),
        "septièmement" | "septiemement" | "septième" | "septieme" => Some(7),
        "huitièmement" | "huitiemement" | "huitième" | "huitieme" => Some(8),
        "neuvièmement" | "neuviemement" | "neuvième" | "neuvieme" => Some(9),
        "dixièmement" | "dixiemement" | "dixième" | "dixieme" => Some(10),
        _ => None,
    }
}

fn parse_spoken_cardinal(word: &str) -> Option<u32> {
    match word.to_ascii_lowercase().as_str() {
        "un" | "une" | "1" => Some(1),
        "deux" | "2" => Some(2),
        "trois" | "3" => Some(3),
        "quatre" | "4" => Some(4),
        "cinq" | "5" => Some(5),
        "six" | "6" => Some(6),
        "sept" | "7" => Some(7),
        "huit" | "8" => Some(8),
        "neuf" | "9" => Some(9),
        "dix" | "10" => Some(10),
        _ => word.parse().ok(),
    }
}

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
    text.lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
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
    text.lines()
        .map(normalize_punctuation_spacing_line)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn normalize_punctuation_spacing_line(text: &str) -> String {
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
        assert!(validate_cleanup_output("", "   ").is_err());
        assert!(validate_cleanup_output("   ", "   ").is_err());
    }

    #[test]
    fn falls_back_when_model_returns_only_thinking_block() {
        let raw = "euh bonjour";
        let model_output = format!("{THINK_OPEN}planning only{THINK_CLOSE}");
        let out = validate_cleanup_output(raw, &model_output).unwrap();
        assert_eq!(out, post_process_transcript(raw));
    }

    #[test]
    fn falls_back_when_model_returns_empty_quotes() {
        let raw = "euh bonjour";
        let out = validate_cleanup_output(raw, "\"\"").unwrap();
        assert_eq!(out, post_process_transcript(raw));
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

    #[test]
    fn join_with_short_pause_inserts_comma() {
        let joined = join_transcript_segments_with_pauses(&[
            ("je vais au magasin".to_string(), 0),
            ("ensuite je rentre".to_string(), 400),
        ]);
        assert_eq!(joined, "je vais au magasin, ensuite je rentre");
    }

    #[test]
    fn join_with_long_pause_inserts_period_and_capitalizes() {
        let joined = join_transcript_segments_with_pauses(&[
            ("bonjour".to_string(), 0),
            ("comment allez-vous".to_string(), 900),
        ]);
        assert_eq!(joined, "bonjour. Comment allez-vous");
    }

    #[test]
    fn join_respects_existing_whisper_punctuation() {
        let joined = join_transcript_segments_with_pauses(&[
            ("Bonjour.".to_string(), 0),
            ("Comment allez-vous".to_string(), 900),
        ]);
        assert_eq!(joined, "Bonjour. Comment allez-vous");
    }

    #[test]
    fn strips_inline_hesitation_fillers() {
        let out = post_process_transcript("je vais euh au magasin");
        assert_eq!(out, "Je vais au magasin");
    }

    #[test]
    fn preserves_benjamin_from_inline_filler_pass() {
        let out = post_process_transcript("contactez benjamin demain");
        assert_eq!(out, "Contactez benjamin demain");
    }

    #[test]
    fn formats_numbered_spoken_list() {
        let out = format_spoken_lists("aller au magasin pour 1 pommes 2 bananes 3 oranges");
        assert!(out.contains("1. Pommes"));
        assert!(out.contains("2. Bananes"));
        assert!(out.contains("3. Oranges"));
        assert!(out.contains('\n'));
    }

    #[test]
    fn formats_bullet_spoken_list() {
        let out = format_spoken_lists("courses tiret pommes tiret bananes tiret oranges");
        assert!(out.contains("Courses :"));
        assert!(out.contains("- Pommes"));
        assert!(out.contains("- Bananes"));
        assert!(out.contains("- Oranges"));
    }

    #[test]
    fn post_process_preserves_list_newlines() {
        let out = post_process_transcript("aller au magasin pour 1 pommes 2 bananes");
        assert!(out.contains('\n'));
    }

    #[test]
    fn find_latest_frozen_boundary_requires_sentence_end() {
        let segments = vec![
            ("phrase incomplète".to_string(), 0),
            ("suite".to_string(), 2000),
        ];
        assert_eq!(find_latest_frozen_boundary(&segments), None);
    }

    #[test]
    fn fits_llm_cleanup_budget_accepts_short_text() {
        assert!(fits_llm_cleanup_budget(
            "Bonjour, ceci est une phrase courte."
        ));
    }

    #[test]
    fn fits_llm_cleanup_budget_rejects_very_long_text() {
        let long = "mot ".repeat(4000);
        assert!(!fits_llm_cleanup_budget(&long));
    }

    #[test]
    fn formats_implicit_noun_list_after_pour() {
        let out = format_spoken_lists("aller au magasin pour pommes bananes oranges");
        assert!(out.contains("1. Pommes"));
        assert!(out.contains("2. Bananes"));
        assert!(out.contains("3. Oranges"));
        assert!(out.contains('\n'));
    }

    #[test]
    fn formats_implicit_list_with_et() {
        let out = format_spoken_lists("courses pour lait et oeufs et pain");
        assert!(out.contains("1. Lait"));
        assert!(out.contains('\n'));
    }

    #[test]
    fn implicit_list_skips_prose_without_enough_items() {
        let out = format_spoken_lists("aller au magasin pour demain");
        assert_eq!(out, "aller au magasin pour demain");
    }

    #[test]
    fn looks_like_implicit_enumeration_detects_shopping_list() {
        assert!(looks_like_implicit_enumeration(
            "aller au magasin pour pommes bananes oranges"
        ));
    }

    #[test]
    fn formats_comma_separated_feature_list() {
        let out =
            format_spoken_lists("ajouter un bouton, mettre un fond blanc, clignoter la page.");
        assert!(out.contains("1. Ajouter un bouton"));
        assert!(out.contains("2. Mettre un fond blanc"));
        assert!(out.contains("3. Clignoter la page"));
        assert!(out.contains('\n'));
    }

    #[test]
    fn formats_comma_separated_feature_list_after_oral_virgule() {
        let out = post_process_transcript(
            "ajouter un bouton virgule mettre un fond blanc virgule clignoter la page",
        );
        assert!(out.contains("1. Ajouter un bouton"));
        assert!(out.contains("2. Mettre un fond blanc"));
        assert!(out.contains("3. Clignoter la page"));
    }

    #[test]
    fn comma_feature_list_skips_conversational_commas() {
        let out = format_spoken_lists("Bonjour, comment allez-vous");
        assert_eq!(out, "Bonjour, comment allez-vous");
    }

    #[test]
    fn looks_like_implicit_enumeration_detects_comma_feature_list() {
        assert!(looks_like_implicit_enumeration(
            "ajouter un bouton, mettre un fond blanc, clignoter la page"
        ));
    }

    #[test]
    fn validate_output_recovers_comma_feature_list_from_flattening() {
        let raw = "1. Ajouter un bouton\n2. Mettre un fond blanc\n3. Clignoter la page";
        let flattened = "Ajouter un bouton, mettre un fond blanc, clignoter la page.";
        let out = validate_cleanup_output(raw, flattened).unwrap();
        assert!(out.contains('\n'));
        assert!(out.contains("1. Ajouter un bouton"));
    }

    #[test]
    fn validate_output_recovers_llm_merged_numbered_feature_list() {
        let raw = "1. Ajouter un bouton\n2. Mettre un fond blanc\n3. Clignoter la page";
        let merged = "1. Ajouter un bouton, mettre un fond blanc, clignoter la page";
        let out = validate_cleanup_output(raw, merged).unwrap();
        assert!(out.contains('\n'));
        assert!(out.contains("2. Mettre un fond blanc"));
        assert!(out.contains("3. Clignoter la page"));
    }

    #[test]
    fn recover_merged_numbered_list_without_multiline_raw() {
        let merged = "1. Ajouter un bouton, mettre un fond blanc, clignoter la page";
        let out = validate_cleanup_output(merged, merged).unwrap();
        assert!(out.contains('\n'));
        assert!(out.contains("3. Clignoter la page"));
    }

    #[test]
    fn validate_output_restores_list_when_llm_flattens_to_commas() {
        let raw = "Aller au magasin pour:\n1. Pommes\n2. Bananes\n3. Oranges";
        let flattened = "Aller au magasin pour pommes, bananes, oranges.";
        let out = validate_cleanup_output(raw, flattened).unwrap();
        assert!(out.contains('\n'));
        assert!(out.contains("1. Pommes"));
    }

    #[test]
    fn validate_output_recovers_implicit_list_from_comma_flattening() {
        let raw = "Aller au magasin pour pommes bananes oranges";
        let flattened = "Aller au magasin pour pommes, bananes, oranges.";
        let out = validate_cleanup_output(raw, flattened).unwrap();
        assert!(out.contains('\n'));
        assert!(out.contains("1. Pommes"));
    }

    #[test]
    fn cleanup_user_message_hints_implicit_enumeration() {
        let msg =
            build_cleanup_user_message("aller au magasin pour pommes bananes oranges").unwrap();
        assert!(msg.contains("liste numérotée"));
    }
}
