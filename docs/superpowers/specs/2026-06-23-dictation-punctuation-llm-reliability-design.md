# Dictation punctuation and LLM reliability — design

**Date:** 2026-06-23  
**Status:** Implemented

## Problem

Users reported telegraphic injected text (many short sentences, ellipsis-like fragmentation) and frequent LLM auto-edit skips despite `auto_edit` being enabled.

## Root causes

1. **Aggressive pause punctuation** — any VAD pause ≥ 700 ms inserted a period between STT segments before LLM cleanup.
2. **LLM skip fallback** — when cleanup failed (`not_loaded`, `timeout`, `validation_failed`, `worker_error`), injection used the period-heavy joined transcript.
3. **Same join path for LLM input and display** — the sidecar received text already split into many sentence boundaries.

## Solution

### A. Three-tier pause punctuation (verbatim / live display)

In `calliop-prompt`:

| Pause before segment | Behavior |
|---|---|
| < 400 ms | space |
| 400–1199 ms | comma |
| ≥ 1200 ms | period + capitalize |

Constants: `PAUSE_COMMA_MIN_MS = 400`, `PAUSE_PERIOD_THRESHOLD_MS = 1200`.

### B. Softer LLM join path

`join_transcript_segments_for_llm()`:

| Pause | Behavior |
|---|---|
| < 400 ms | space |
| 400–1499 ms | comma |
| ≥ 1500 ms | period |

Used in `build_llm_ready_text()`, auto-edit `snippet_fallback`, and pipelined tail fallback.

### C. LLM reliability

- `LLM_CLEANUP_TIMEOUT`: 45 s → 60 s (max 120 s unchanged).
- `LLM_ENGINE_WAIT_TIMEOUT`: 60 s for cold-start sidecar wait.
- Structured `eprintln!` on chunk and session LLM skips.
- `SYSTEM_PROMPT`: merge consecutive very short sentences when hesitations/pauses fragmented the transcript.

### D. Observability

`LatencySummary` shows `Ignorée — <reason>` when `llmStatus` is `skipped` or `failed`.

## Out of scope

- User-configurable pause thresholds in Settings UI
- Default LLM model tier change
- Standalone `merge_telegraphic_runs` heuristic

## Validation

- `cargo test -p calliop-prompt`
- `cargo test -p calliop -- orchestrator`
- `pnpm typecheck`
- Manual: natural pauses ~0.8 s → commas not periods; pause ≥ 1.5 s → periods preserved
