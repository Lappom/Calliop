# Dictation cleanup quality — design

**Date:** 2026-06-12  
**Status:** Implemented

## Problem

Users reported poorly formatted injected text after dictation: fake domains (`statistique.a`, `comme.com`), glued punctuation (`affichage.sur`, `bar affiché.3`), STT errors, and raw unprocessed speech despite **auto-edit** being enabled.

## Root causes

1. **`collapse_spaced_dots_in_identifiers`** collapsed any `token . token` into a fake domain, including French prose (`statistique . a` → `statistique.a`).
2. **`normalize_punctuation_spacing`** preserved glued `.` before letters/digits when both sides looked like identifiers, without checking sentence context.
3. **LLM silent fallback** — when `auto_edit=true` but the sidecar was not loaded (timeout, validation failure), injection used deterministic post-processing only with no user feedback (`llmMs = 0`).
4. **LLM preload** skipped on minimized startup even when RAM was sufficient and auto-edit was on.

## Solution

### A. Deterministic post-processing (`calliop-prompt`)

- Added `should_collapse_dot(left, right, preceding)` with:
  - TLD allowlist (`com`, `fr`, `org`, …)
  - French function-word blocklist for the right token (`a`, `sur`, `des`, …)
  - Common-word blocklist for the left token (`comme`, `statistique`, …)
  - Email context (`@` in preceding text)
  - Version numbers (`v1.2`)
- `normalize_punctuation_spacing` splits glued `.` when collapse is not warranted.
- Regression tests for reported cases and email dictation.

### B. LLM observability

- Extended `LatencyMetricsEvent` with `llmStatus` (`applied` | `skipped` | `failed` | `disabled`) and optional `llmSkipReason` (`not_loaded`, `timeout`, `validation_failed`, `worker_error`).
- Emits `llm-skipped` when auto-edit did not apply.
- Frontend: `LlmSkipToast`, translated labels in `LatencySummary`, locale strings FR/EN.

### C. LLM preload

- When starting minimized with sufficient RAM and `auto_edit=true` (and not `low_power_mode`), preload LLM alongside Whisper.

## Out of scope (phase 2)

- VAD mid-word healing heuristics
- Raising default LLM model tier
- Pre-injection text preview UI

## Validation

- `cargo test -p calliop-prompt` — 43 tests including new regression cases
- `cargo test` profile minimized preload
- `pnpm typecheck` — frontend types for new metrics fields
