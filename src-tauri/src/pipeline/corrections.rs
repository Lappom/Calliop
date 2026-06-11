use crate::store::DictionaryCorrectionRule;

use super::snippets::{find_first_match, normalize_for_match};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrectionRule {
    pub incorrect: String,
    pub replacement: String,
}

impl From<DictionaryCorrectionRule> for CorrectionRule {
    fn from(rule: DictionaryCorrectionRule) -> Self {
        Self {
            incorrect: rule.incorrect,
            replacement: rule.replacement,
        }
    }
}

/// Replaces dictionary misspellings with their configured replacements (deterministic, offline).
pub fn apply_corrections(text: &str, rules: &[CorrectionRule]) -> String {
    if rules.is_empty() || text.is_empty() {
        return text.to_owned();
    }

    let mut ordered: Vec<&CorrectionRule> = rules.iter().collect();
    ordered.sort_by_key(|rule| std::cmp::Reverse(normalize_for_match(&rule.incorrect)));

    let mut result = String::with_capacity(text.len());
    let mut search_start = 0_usize;

    while search_start < text.len() {
        let slice = &text[search_start..];
        let mut best_match: Option<(usize, usize, Option<char>, &CorrectionRule)> = None;

        for rule in &ordered {
            let Some((rel_start, rel_end, trailing)) = find_first_match(slice, &rule.incorrect)
            else {
                continue;
            };

            let should_take = match &best_match {
                None => true,
                Some((best_start, best_end, _, best_rule)) => {
                    rel_start < *best_start
                        || (rel_start == *best_start
                            && (rel_end - rel_start) > (best_end - best_start))
                        || (rel_start == *best_start
                            && (rel_end - rel_start) == (best_end - best_start)
                            && normalize_for_match(&rule.incorrect).len()
                                > normalize_for_match(&best_rule.incorrect).len())
                }
            };

            if should_take {
                best_match = Some((rel_start, rel_end, trailing, rule));
            }
        }

        let Some((rel_start, rel_end, trailing, rule)) = best_match else {
            result.push_str(slice);
            break;
        };

        result.push_str(&slice[..rel_start]);
        result.push_str(&rule.replacement);
        if let Some(punctuation) = trailing {
            result.push(punctuation);
        }
        search_start += rel_end;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(incorrect: &str, replacement: &str) -> CorrectionRule {
        CorrectionRule {
            incorrect: incorrect.to_string(),
            replacement: replacement.to_string(),
        }
    }

    #[test]
    fn replaces_simple_misspelling() {
        let rules = vec![rule("Caliope", "Calliop")];
        let result = apply_corrections("bonjour Caliope", &rules);
        assert_eq!(result, "bonjour Calliop");
    }

    #[test]
    fn replaces_case_insensitively() {
        let rules = vec![rule("caliope", "Calliop")];
        let result = apply_corrections("bonjour CALIOPE", &rules);
        assert_eq!(result, "bonjour Calliop");
    }

    #[test]
    fn replaces_accent_insensitive() {
        let rules = vec![rule("deja", "déjà")];
        let result = apply_corrections("Je l'ai déjà fait.", &rules);
        assert_eq!(result, "Je l'ai déjà fait.");
    }

    #[test]
    fn preserves_trailing_punctuation() {
        let rules = vec![rule("Caliope", "Calliop")];
        let result = apply_corrections("Caliope.", &rules);
        assert_eq!(result, "Calliop.");
    }

    #[test]
    fn ignores_partial_word_matches() {
        let rules = vec![rule("cal", "Calliop")];
        let result = apply_corrections("recalibrer", &rules);
        assert_eq!(result, "recalibrer");
    }

    #[test]
    fn replaces_multiple_occurrences() {
        let rules = vec![rule("Caliope", "Calliop")];
        let result = apply_corrections("Caliope et Caliope", &rules);
        assert_eq!(result, "Calliop et Calliop");
    }

    #[test]
    fn leaves_unmatched_text_unchanged() {
        let rules = vec![rule("Caliope", "Calliop")];
        let result = apply_corrections("bonjour tout le monde", &rules);
        assert_eq!(result, "bonjour tout le monde");
    }

    #[test]
    fn replaces_dotted_misspelling_with_symbol() {
        let rules = vec![rule("Arro.Baz.", "@")];
        let result = apply_corrections("Envoyez un Arro.Baz. maintenant", &rules);
        assert_eq!(result, "Envoyez un @ maintenant");
    }

    #[test]
    fn prefers_longer_incorrect_match() {
        let rules = vec![rule("call", "short"), rule("call io", "long")];
        let result = apply_corrections("call io", &rules);
        assert_eq!(result, "long");
    }
}
