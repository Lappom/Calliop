use crate::store::Snippet;

const TRAILING_PUNCTUATION: [char; 6] = ['.', ',', ';', ':', '!', '?'];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShieldedSnippet {
    pub token: String,
    pub content: String,
}

fn shield_token(index: usize) -> String {
    format!("⟦CALLIOP{index}⟧")
}

/// Replaces matched snippet triggers with opaque tokens so LLM cleanup cannot alter
/// the expanded content. Returns the shielded text and the token-to-content map.
pub fn shield_snippet_triggers(text: &str, snippets: &[Snippet]) -> (String, Vec<ShieldedSnippet>) {
    if snippets.is_empty() || text.is_empty() {
        return (text.to_owned(), Vec::new());
    }

    let mut ordered: Vec<&Snippet> = snippets.iter().collect();
    ordered.sort_by_key(|snippet| std::cmp::Reverse(normalize_for_match(&snippet.trigger)));

    let mut result = String::with_capacity(text.len());
    let mut shields = Vec::new();
    let mut search_start = 0_usize;
    let mut next_index = 0_usize;

    while search_start < text.len() {
        let slice = &text[search_start..];
        let mut best_match: Option<(usize, usize, Option<char>, &Snippet)> = None;

        for snippet in &ordered {
            let Some((rel_start, rel_end, trailing)) = find_first_match(slice, &snippet.trigger)
            else {
                continue;
            };

            let should_take = match &best_match {
                None => true,
                Some((best_start, best_end, _, best_snippet)) => {
                    rel_start < *best_start
                        || (rel_start == *best_start
                            && (rel_end - rel_start) > (best_end - best_start))
                        || (rel_start == *best_start
                            && (rel_end - rel_start) == (best_end - best_start)
                            && normalize_for_match(&snippet.trigger).len()
                                > normalize_for_match(&best_snippet.trigger).len())
                }
            };

            if should_take {
                best_match = Some((rel_start, rel_end, trailing, snippet));
            }
        }

        let Some((rel_start, rel_end, trailing, snippet)) = best_match else {
            result.push_str(slice);
            break;
        };

        result.push_str(&slice[..rel_start]);
        let token = shield_token(next_index);
        next_index += 1;
        shields.push(ShieldedSnippet {
            token: token.clone(),
            content: snippet.content.clone(),
        });
        result.push_str(&token);
        if let Some(punctuation) = trailing {
            result.push(punctuation);
        }
        search_start += rel_end;
    }

    (result, shields)
}

pub fn unshield_snippets(text: &str, shields: &[ShieldedSnippet]) -> String {
    let mut result = text.to_string();
    for shield in shields {
        result = result.replace(&shield.token, &shield.content);
    }
    result
}

/// Applies LLM output while preserving configured snippet content.
pub fn finalize_llm_with_snippets(
    cleaned: &str,
    shields: &[ShieldedSnippet],
    full_text: &str,
    snippets: &[Snippet],
) -> String {
    if shields.is_empty() {
        return cleaned.to_string();
    }
    if shields.iter().all(|shield| cleaned.contains(&shield.token)) {
        return unshield_snippets(cleaned, shields);
    }

    let from_cleaned = apply_snippets(cleaned, snippets);
    if from_cleaned != cleaned {
        return from_cleaned;
    }
    apply_snippets(full_text, snippets)
}

/// Expands voice snippet triggers into their full content (deterministic, offline).
pub fn apply_snippets(text: &str, snippets: &[Snippet]) -> String {
    if snippets.is_empty() || text.is_empty() {
        return text.to_owned();
    }

    let mut ordered: Vec<&Snippet> = snippets.iter().collect();
    ordered.sort_by_key(|snippet| std::cmp::Reverse(normalize_for_match(&snippet.trigger)));

    let mut result = String::with_capacity(text.len());
    let mut search_start = 0_usize;

    while search_start < text.len() {
        let slice = &text[search_start..];
        let mut best_match: Option<(usize, usize, Option<char>, &Snippet)> = None;

        for snippet in &ordered {
            let Some((rel_start, rel_end, trailing)) = find_first_match(slice, &snippet.trigger)
            else {
                continue;
            };

            let should_take = match &best_match {
                None => true,
                Some((best_start, best_end, _, best_snippet)) => {
                    rel_start < *best_start
                        || (rel_start == *best_start
                            && (rel_end - rel_start) > (best_end - best_start))
                        || (rel_start == *best_start
                            && (rel_end - rel_start) == (best_end - best_start)
                            && normalize_for_match(&snippet.trigger).len()
                                > normalize_for_match(&best_snippet.trigger).len())
                }
            };

            if should_take {
                best_match = Some((rel_start, rel_end, trailing, snippet));
            }
        }

        let Some((rel_start, rel_end, trailing, snippet)) = best_match else {
            result.push_str(slice);
            break;
        };

        result.push_str(&slice[..rel_start]);
        result.push_str(&snippet.content);
        if let Some(punctuation) = trailing {
            result.push(punctuation);
        }
        search_start += rel_end;
    }

    result
}

pub(crate) fn find_first_match(text: &str, trigger: &str) -> Option<(usize, usize, Option<char>)> {
    let trigger_norm = normalize_for_match(trigger);
    if trigger_norm.is_empty() {
        return None;
    }

    let trigger_chars: Vec<char> = trigger_norm.chars().collect();
    let text_chars: Vec<(usize, char)> = text.char_indices().collect();

    for (index, &(byte_start, _)) in text_chars.iter().enumerate() {
        if !is_word_boundary_before(text, byte_start) {
            continue;
        }

        let mut text_index = index;
        let mut trigger_index = 0;
        let mut matched = true;

        while trigger_index < trigger_chars.len() {
            while text_index < text_chars.len()
                && text_chars[text_index].1.is_whitespace()
                && (trigger_index == 0
                    || trigger_index > 0 && trigger_chars[trigger_index - 1].is_whitespace())
            {
                text_index += 1;
            }

            if text_index >= text_chars.len() {
                matched = false;
                break;
            }

            if trigger_chars[trigger_index].is_whitespace() {
                if !text_chars[text_index].1.is_whitespace() {
                    matched = false;
                    break;
                }
                trigger_index += 1;
                text_index += 1;
                continue;
            }

            let text_char = fold_char(text_chars[text_index].1);
            let trigger_char = trigger_chars[trigger_index];
            if text_char != trigger_char {
                matched = false;
                break;
            }

            trigger_index += 1;
            text_index += 1;
        }

        if !matched {
            continue;
        }

        let match_end_byte = if text_index < text_chars.len() {
            text_chars[text_index].0
        } else {
            text.len()
        };

        if match_end_byte < text.len() {
            let next_char = text[match_end_byte..].chars().next()?;
            if is_word_char(next_char) {
                continue;
            }
        }

        let trailing = if match_end_byte < text.len() {
            let next_char = text[match_end_byte..].chars().next()?;
            if TRAILING_PUNCTUATION.contains(&next_char) {
                Some((next_char, match_end_byte + next_char.len_utf8()))
            } else {
                None
            }
        } else {
            None
        };

        let end_byte = trailing.map(|(_, end)| end).unwrap_or(match_end_byte);
        return Some((byte_start, end_byte, trailing.map(|(ch, _)| ch)));
    }

    None
}

fn is_word_boundary_before(text: &str, byte_index: usize) -> bool {
    if byte_index == 0 {
        return true;
    }
    text[..byte_index]
        .chars()
        .next_back()
        .is_none_or(|ch| !is_word_char(ch))
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '\'' || ch == '’'
}

pub(crate) fn normalize_for_match(value: &str) -> String {
    value
        .split_whitespace()
        .map(|part| part.chars().map(fold_char).collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn fold_char(ch: char) -> char {
    match ch {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'ç' => 'c',
        'è' | 'é' | 'ê' | 'ë' => 'e',
        'ì' | 'í' | 'î' | 'ï' => 'i',
        'ñ' => 'n',
        'ò' | 'ó' | 'ô' | 'ö' => 'o',
        'ù' | 'ú' | 'û' | 'ü' => 'u',
        'ý' | 'ÿ' => 'y',
        'æ' => 'a',
        'œ' => 'o',
        other => other.to_ascii_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Snippet;

    fn snippet(trigger: &str, content: &str) -> Snippet {
        Snippet {
            id: 1,
            trigger: trigger.to_string(),
            content: content.to_string(),
            created_at: "now".into(),
        }
    }

    #[test]
    fn expands_trigger_case_insensitively() {
        let snippets = vec![snippet("mon calendrier", "https://calendly.com/me")];
        let result = apply_snippets("Voici Mon Calendrier pour réserver.", &snippets);
        assert_eq!(result, "Voici https://calendly.com/me pour réserver.");
    }

    #[test]
    fn preserves_trailing_punctuation() {
        let snippets = vec![snippet("mon calendrier", "https://calendly.com/me")];
        let result = apply_snippets("mon calendrier.", &snippets);
        assert_eq!(result, "https://calendly.com/me.");
    }

    #[test]
    fn prefers_longer_trigger() {
        let snippets = vec![
            snippet("calendrier", "short"),
            snippet("mon calendrier", "long"),
        ];
        let result = apply_snippets("mon calendrier", &snippets);
        assert_eq!(result, "long");
    }

    #[test]
    fn ignores_partial_word_matches() {
        let snippets = vec![snippet("cal", "expanded")];
        let result = apply_snippets("recalibrer", &snippets);
        assert_eq!(result, "recalibrer");
    }

    #[test]
    fn matches_accent_insensitive() {
        let snippets = vec![snippet("deja", "déjà vu")];
        let result = apply_snippets("Je l'ai déjà fait.", &snippets);
        assert_eq!(result, "Je l'ai déjà vu fait.");
    }

    #[test]
    fn leaves_unmatched_text_unchanged() {
        let snippets = vec![snippet("signature", "Cordialement")];
        let result = apply_snippets("bonjour tout le monde", &snippets);
        assert_eq!(result, "bonjour tout le monde");
    }

    #[test]
    fn shield_and_unshield_roundtrip() {
        let snippets = vec![snippet("mon calendrier", "https://calendly.com/me")];
        let (shielded, shields) =
            shield_snippet_triggers("Voici mon calendrier demain.", &snippets);
        assert!(shielded.contains("⟦CALLIOP0⟧"));
        assert_eq!(shields.len(), 1);
        let cleaned = shielded.replace("Voici", "Voici,");
        assert_eq!(
            unshield_snippets(&cleaned, &shields),
            "Voici, https://calendly.com/me demain."
        );
    }

    #[test]
    fn finalize_llm_with_snippets_falls_back_when_tokens_missing() {
        let snippets = vec![snippet("mon calendrier", "https://calendly.com/me")];
        let (_, shields) = shield_snippet_triggers("mon calendrier", &snippets);
        let result =
            finalize_llm_with_snippets("Mon agenda", &shields, "mon calendrier", &snippets);
        assert_eq!(result, "https://calendly.com/me");
    }
}
