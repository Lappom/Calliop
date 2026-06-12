# Live editing during dictation — design

**Date:** 2026-06-12  
**Status:** Approved

## Problem

Calliop should match Wispr Flow-style intelligent dictation: remove hesitation fillers mid-sentence, format spoken lists, infer punctuation from pauses, resolve self-corrections, and pipeline LLM cleanup during recording so final injection feels instant.

## Decisions

| Topic | Decision |
|---|---|
| Real-time display | No cleaned overlay preview; pipeline LLM during recording only |
| Deterministic (no LLM) | Inline fillers, pause punctuation, spoken lists |
| LLM-only | Self-corrections (« à 14h en fait 15h ») |
| Lists | Numbered + bullet (`tiret`) |
| Frozen boundary | Sentence end + pause ≥ 1.5 s before next segment |

## Architecture

1. **VAD** exposes `SpeechSegment { samples, leading_silence_ms }`.
2. **`join_transcript_segments_with_pauses`** inserts comma/period from pause thresholds; Whisper punctuation wins when present.
3. **`post_process_transcript`** extended: inline fillers, spoken lists, preserved newlines.
4. **Streaming worker** detects frozen boundaries and starts background LLM on prefix.
5. **Stop** merges cleaned prefix + quick LLM tail (or full fallback).

## Thresholds

- Pause < 700 ms → comma between segments
- Pause ≥ 700 ms → period + capitalize next segment
- Frozen boundary → pause ≥ 1500 ms after sentence-ending segment

## Out of scope

- Progressive injection into target apps
- Cleaned text preview in overlay
- Voice tone detection for punctuation

## Validation

- `cargo test -p calliop-prompt`
- `cargo test` (audio VAD, orchestrator boundary tests)
- `pnpm typecheck`
