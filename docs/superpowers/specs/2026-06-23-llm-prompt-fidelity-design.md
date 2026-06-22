# LLM prompt fidelity — design

**Date:** 2026-06-23  
**Status:** Implemented

## Problem

Users reported the LLM auto-edit was too aggressive: reformulating style, removing words, or changing meaning despite acceptable raw transcripts.

## Decision

**Option A — targeted prompt edit** (moderate fidelity mode):

- Add a golden rule at the top: stay faithful to user words; no style reformulation, no added content, no information removal except fillers and explicitly abandoned fragments.
- Remove « Reformule légèrement si nécessaire pour améliorer la fluidité ».
- Soften short-sentence handling: link fragments with punctuation without rewriting words.
- Keep fillers removal, self-corrections (« 14h en fait 15h »), oral punctuation commands, and list formatting rules.

## Out of scope

- Code guard rejecting over-truncated LLM output
- Full prompt restructure or few-shot examples
- Default model tier change

## Validation

- `cargo test -p calliop-prompt`
- Manual dictation: fillers removed, wording preserved, obvious self-corrections still applied
