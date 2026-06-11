//! Runtime expansion of snippet template variables (offline, deterministic).

const TOKEN_DATE: &str = "{{date}}";
const TOKEN_CLIPBOARD: &str = "{{clipboard}}";
const TOKEN_NOM: &str = "{{nom}}";

const MONTHS_FR: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SnippetVariableContext {
    pub user_name: String,
    pub clipboard: Option<String>,
    /// When set, used instead of the local calendar date (tests).
    pub date_override: Option<String>,
}

impl SnippetVariableContext {
    pub fn from_user_name(user_name: String) -> Self {
        Self {
            user_name,
            clipboard: None,
            date_override: None,
        }
    }

    pub fn with_clipboard(mut self, clipboard: Option<String>) -> Self {
        self.clipboard = clipboard;
        self
    }
}

/// Expands `{{date}}`, `{{clipboard}}`, and `{{nom}}` in snippet content.
pub fn expand_snippet_variables(content: &str, ctx: &SnippetVariableContext) -> String {
    if !content.contains("{{") {
        return content.to_owned();
    }

    let date_value = ctx
        .date_override
        .clone()
        .unwrap_or_else(format_local_date_french);
    let clipboard_value = ctx.clipboard.as_deref().unwrap_or("");
    let nom_value = ctx.user_name.as_str();

    content
        .replace(TOKEN_DATE, &date_value)
        .replace(TOKEN_CLIPBOARD, clipboard_value)
        .replace(TOKEN_NOM, nom_value)
}

pub fn format_local_date_french() -> String {
    use chrono::Datelike;

    let now = chrono::Local::now();
    let month_name = MONTHS_FR
        .get(now.month0() as usize)
        .copied()
        .unwrap_or("?");
    format!("{} {month_name} {}", now.day(), now.year())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(user_name: &str, clipboard: Option<&str>, date: Option<&str>) -> SnippetVariableContext {
        SnippetVariableContext {
            user_name: user_name.to_string(),
            clipboard: clipboard.map(str::to_string),
            date_override: date.map(str::to_string),
        }
    }

    #[test]
    fn expands_all_tokens() {
        let result = expand_snippet_variables(
            "Bonjour {{nom}}, copie: {{clipboard}}, le {{date}}.",
            &ctx("Alice", Some("ligne"), Some("5 mars 2026")),
        );
        assert_eq!(result, "Bonjour Alice, copie: ligne, le 5 mars 2026.");
    }

    #[test]
    fn empty_nom_and_clipboard() {
        let result = expand_snippet_variables(
            "{{nom}}|{{clipboard}}",
            &ctx("", None, Some("1 janvier 2026")),
        );
        assert_eq!(result, "|");
    }

    #[test]
    fn leaves_plain_text_unchanged() {
        let result = expand_snippet_variables("Hello world", &ctx("Bob", None, None));
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn date_token_is_case_sensitive() {
        let result = expand_snippet_variables(
            "{{DATE}}",
            &ctx("", None, Some("2 février 2026")),
        );
        assert_eq!(result, "{{DATE}}");
    }
}
