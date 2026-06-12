#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserError {
    MicProbeActiveBeforeDictation,
    UnsupportedSttLanguage,
    InvalidMatchType,
    InvalidTone,
    UnknownWhisperModel,
    CannotDeleteAutoWhisperModel,
    CannotDeleteActiveWhisperModel,
    UnknownLlmModel,
    LlmEngineLoadFailed,
    LlmModelCorrupt,
    CannotDeleteAutoLlmModel,
    CannotDeleteActiveLlmModel,
    InvalidModelKind,
    InvalidInferenceBackend,
    DictionaryWordEmpty,
    DictionaryWordInvalid,
    DictionaryMisspellingInvalid,
    DictionaryMisspellingSame,
    DictionaryMisspellingExists,
    DictionaryWordExists,
    DictionaryWordNotFound,
    SnippetTriggerEmpty,
    SnippetTriggerTooShort,
    SnippetContentEmpty,
    SnippetNotFound,
    InvalidSnippetJson,
    SnippetImportEmpty,
    SnippetImportNoValid,
    AppContextPatternTooShort,
    AppContextRuleNotFound,
    DictationNotFound,
    MicProbeAlreadyActive,
    DictationActiveMicProbe,
}

impl UserError {
    pub fn as_code(self) -> &'static str {
        match self {
            Self::MicProbeActiveBeforeDictation => "MIC_PROBE_ACTIVE_BEFORE_DICTATION",
            Self::UnsupportedSttLanguage => "UNSUPPORTED_STT_LANGUAGE",
            Self::InvalidMatchType => "INVALID_MATCH_TYPE",
            Self::InvalidTone => "INVALID_TONE",
            Self::UnknownWhisperModel => "UNKNOWN_WHISPER_MODEL",
            Self::CannotDeleteAutoWhisperModel => "CANNOT_DELETE_AUTO_WHISPER",
            Self::CannotDeleteActiveWhisperModel => "CANNOT_DELETE_ACTIVE_WHISPER",
            Self::UnknownLlmModel => "UNKNOWN_LLM_MODEL",
            Self::LlmEngineLoadFailed => "LLM_ENGINE_LOAD_FAILED",
            Self::LlmModelCorrupt => "LLM_MODEL_CORRUPT",
            Self::CannotDeleteAutoLlmModel => "CANNOT_DELETE_AUTO_LLM",
            Self::CannotDeleteActiveLlmModel => "CANNOT_DELETE_ACTIVE_LLM",
            Self::InvalidModelKind => "INVALID_MODEL_KIND",
            Self::InvalidInferenceBackend => "INVALID_INFERENCE_BACKEND",
            Self::DictionaryWordEmpty => "WORD_EMPTY",
            Self::DictionaryWordInvalid => "WORD_TOO_LONG",
            Self::DictionaryMisspellingInvalid => "MISSPELLING_INVALID",
            Self::DictionaryMisspellingSame => "MISSPELLING_SAME_AS_WORD",
            Self::DictionaryMisspellingExists => "MISSPELLING_ALREADY_REGISTERED",
            Self::DictionaryWordExists => "WORD_ALREADY_IN_DICTIONARY",
            Self::DictionaryWordNotFound => "WORD_NOT_FOUND",
            Self::SnippetTriggerEmpty => "SNIPPET_TRIGGER_EMPTY",
            Self::SnippetTriggerTooShort => "SNIPPET_TRIGGER_TOO_SHORT",
            Self::SnippetContentEmpty => "SNIPPET_CONTENT_EMPTY",
            Self::SnippetNotFound => "SNIPPET_NOT_FOUND",
            Self::InvalidSnippetJson => "INVALID_JSON",
            Self::SnippetImportEmpty => "SNIPPET_FILE_EMPTY",
            Self::SnippetImportNoValid => "SNIPPET_IMPORT_NO_VALID",
            Self::AppContextPatternTooShort => "PATTERN_TOO_SHORT",
            Self::AppContextRuleNotFound => "RULE_NOT_FOUND",
            Self::DictationNotFound => "DICTATION_NOT_FOUND",
            Self::MicProbeAlreadyActive => "MIC_PROBE_ALREADY_ACTIVE",
            Self::DictationActiveMicProbe => "DICTATION_ACTIVE_BEFORE_MIC_PROBE",
        }
    }
}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_code())
    }
}

pub fn user_error_string(err: UserError) -> String {
    err.as_code().into()
}
