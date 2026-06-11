//! Pure logic for locating injected text regions via anchor words.

pub const ANCHOR_WORD_COUNT: usize = 5;
pub const MAX_REGION_FACTOR: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InjectionAnchors {
    pub prefix: String,
    pub suffix: String,
}

pub fn find_injection_bounds(document: &str, injected: &str) -> Option<(usize, usize)> {
    if injected.is_empty() {
        return None;
    }

    let normalized_doc = normalize_whitespace(document);
    let normalized_injected = normalize_whitespace(injected);

    if let Some(start) = normalized_doc.rfind(&normalized_injected) {
        return map_normalized_span_to_original(
            document,
            &normalized_doc,
            start,
            normalized_injected.len(),
        );
    }

    document
        .rfind(injected)
        .map(|start| (start, start + injected.len()))
}

pub fn build_anchors(document: &str, injected: &str) -> Option<InjectionAnchors> {
    let (start, end) = find_injection_bounds(document, injected)?;
    let before = &document[..start];
    let after = &document[end..];

    Some(InjectionAnchors {
        prefix: last_n_words(before, ANCHOR_WORD_COUNT),
        suffix: first_n_words(after, ANCHOR_WORD_COUNT),
    })
}

pub fn extract_region(
    document: &str,
    anchors: &InjectionAnchors,
    max_region_len: usize,
) -> Option<String> {
    if anchors.prefix.is_empty() && anchors.suffix.is_empty() {
        let trimmed = document.trim();
        return if trimmed.len() <= max_region_len {
            Some(trimmed.to_string())
        } else {
            None
        };
    }

    if anchors.prefix.is_empty() {
        return extract_region_after_prefix(document, 0, anchors, max_region_len);
    }

    let mut last_region = None;
    let mut search_from = 0;
    while let Some(rel) = document[search_from..].find(&anchors.prefix) {
        let prefix_end = search_from + rel + anchors.prefix.len();
        if let Some(region) =
            extract_region_after_prefix(document, prefix_end, anchors, max_region_len)
        {
            last_region = Some(region);
        }
        search_from = search_from + rel + 1;
    }

    last_region
}

fn extract_region_after_prefix(
    document: &str,
    prefix_end: usize,
    anchors: &InjectionAnchors,
    max_region_len: usize,
) -> Option<String> {
    if prefix_end > document.len() {
        return None;
    }

    let after_prefix = &document[prefix_end..];
    let region = if anchors.suffix.is_empty() {
        after_prefix.trim().to_string()
    } else {
        let suffix_pos = after_prefix.find(&anchors.suffix)?;
        after_prefix[..suffix_pos].trim().to_string()
    };

    if region.is_empty() || region.len() > max_region_len {
        return None;
    }

    Some(region)
}

pub fn is_stabilized(previous: &str, current: &str, stable_reads: u32) -> bool {
    stable_reads >= 2 && !previous.is_empty() && previous == current
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn map_normalized_span_to_original(
    original: &str,
    normalized: &str,
    norm_start: usize,
    norm_len: usize,
) -> Option<(usize, usize)> {
    if norm_start + norm_len > normalized.len() {
        return None;
    }

    let target = &normalized[norm_start..norm_start + norm_len];
    let mut norm_pos = 0usize;
    let mut orig_start = None;
    let mut orig_end = None;
    let mut in_word = false;

    for (idx, ch) in original.char_indices() {
        if ch.is_whitespace() {
            if in_word {
                in_word = false;
                if norm_pos == norm_start + norm_len {
                    orig_end = Some(idx);
                    break;
                }
            }
            if norm_pos < normalized.len() && normalized.as_bytes().get(norm_pos) == Some(&b' ') {
                norm_pos += 1;
            }
            continue;
        }

        if !in_word {
            in_word = true;
            if norm_pos == norm_start {
                orig_start = Some(idx);
            }
        }

        if norm_pos < normalized.len() && normalized[norm_pos..].starts_with(ch) {
            norm_pos += ch.len_utf8();
        }

        if norm_pos == norm_start + norm_len {
            orig_end = Some(idx + ch.len_utf8());
            break;
        }
    }

    if orig_end.is_none() && norm_pos == norm_start + norm_len {
        orig_end = Some(original.len());
    }

    let start = orig_start?;
    let end = orig_end?;
    let extracted = normalize_whitespace(&original[start..end]);
    if extracted == target {
        Some((start, end))
    } else {
        None
    }
}

fn first_n_words(text: &str, count: usize) -> String {
    text.split_whitespace()
        .take(count)
        .collect::<Vec<_>>()
        .join(" ")
}

fn last_n_words(text: &str, count: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }
    let start = words.len().saturating_sub(count);
    words[start..].join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_injection_bounds_locates_substring() {
        let doc = "Hello world Calliop test";
        let bounds = find_injection_bounds(doc, "Calliop").expect("bounds");
        assert_eq!(bounds, (12, 19));
    }

    #[test]
    fn find_injection_bounds_prefers_last_occurrence() {
        let doc = "Calliop is great. I love Calliop!";
        let bounds = find_injection_bounds(doc, "Calliop").expect("bounds");
        assert_eq!(&doc[bounds.0..bounds.1], "Calliop");
        assert_eq!(bounds.0, doc.rfind("Calliop").expect("last match"));
    }

    #[test]
    fn find_injection_bounds_handles_accented_whitespace_normalization() {
        let doc = "Bonjour   à   tous,  ceci est un test.";
        let injected = "à tous";
        let bounds = find_injection_bounds(doc, injected).expect("bounds");
        let matched = &doc[bounds.0..bounds.1];
        assert_eq!(normalize_whitespace(matched), "à tous");
    }

    #[test]
    fn build_anchors_captures_surrounding_words() {
        let doc = "one two three Calliop four five six";
        let anchors = build_anchors(doc, "Calliop").expect("anchors");
        assert_eq!(anchors.prefix, "one two three");
        assert_eq!(anchors.suffix, "four five six");
    }

    #[test]
    fn extract_region_prefers_last_valid_prefix_match() {
        let anchors = build_anchors(
            "one two three Calliop four five six",
            "Calliop",
        )
        .expect("anchors");
        let doc = "one two three noise one two three Calliope four five six";
        let region = extract_region(doc, &anchors, 64).expect("region");
        assert_eq!(region, "Calliope");
    }

    #[test]
    fn extract_region_returns_corrected_segment() {
        let doc = "one two three Calliope four five six";
        let anchors =
            build_anchors("one two three Calliop four five six", "Calliop").expect("anchors");
        let region = extract_region(doc, &anchors, 64).expect("region");
        assert_eq!(region, "Calliope");
    }

    #[test]
    fn extract_region_rejects_oversized_region() {
        let anchors = InjectionAnchors {
            prefix: String::new(),
            suffix: String::new(),
        };
        let doc = "x".repeat(100);
        assert!(extract_region(&doc, &anchors, 10).is_none());
    }

    #[test]
    fn is_stabilized_requires_two_matching_reads() {
        assert!(!is_stabilized("a", "a", 1));
        assert!(is_stabilized("a", "a", 2));
        assert!(!is_stabilized("a", "b", 2));
    }
}
