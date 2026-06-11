//! Word Error Rate (WER) helpers for STT benchmarks.

/// Normalize text for WER: lowercase, strip punctuation, collapse whitespace.
pub fn normalize_for_wer(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_space = false;

    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '\'' {
            out.push(ch.to_ascii_lowercase());
            last_space = false;
        } else if !last_space && !out.is_empty() {
            out.push(' ');
            last_space = true;
        }
    }

    out.trim().to_string()
}

/// Split normalized text into word tokens.
pub fn tokenize_for_wer(text: &str) -> Vec<String> {
    normalize_for_wer(text)
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

/// Levenshtein distance at word level.
pub fn word_edit_distance(reference: &[String], hypothesis: &[String]) -> usize {
    let rows = reference.len() + 1;
    let cols = hypothesis.len() + 1;
    if rows == 1 {
        return hypothesis.len();
    }
    if cols == 1 {
        return reference.len();
    }

    let mut prev: Vec<usize> = (0..cols).collect();
    let mut curr = vec![0; cols];

    for i in 1..rows {
        curr[0] = i;
        for j in 1..cols {
            let cost = if reference[i - 1] == hypothesis[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[cols - 1]
}

/// WER as a fraction in [0, 1+]; 0 = perfect match.
pub fn word_error_rate(reference: &str, hypothesis: &str) -> f64 {
    let ref_tokens = tokenize_for_wer(reference);
    if ref_tokens.is_empty() {
        return if tokenize_for_wer(hypothesis).is_empty() {
            0.0
        } else {
            1.0
        };
    }

    let hyp_tokens = tokenize_for_wer(hypothesis);
    let edits = word_edit_distance(&ref_tokens, &hyp_tokens);
    edits as f64 / ref_tokens.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_match_is_zero_wer() {
        assert_eq!(word_error_rate("Bonjour le monde", "bonjour le monde"), 0.0);
    }

    #[test]
    fn one_substitution_wer() {
        let wer = word_error_rate("bonjour le monde", "bonjour la monde");
        assert!((wer - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn punctuation_is_ignored() {
        assert_eq!(
            word_error_rate("Bonjour, le monde!", "bonjour le monde"),
            0.0
        );
    }
}
