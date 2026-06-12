# Onboarding tutorial redesign

**Date:** 2026-06-12  
**Status:** Implemented

## Problem

The first-launch tutorial used an in-app button calling `toggle_dictation` instead of the global hotkey. Mic probe suspended the hotkey and could leave it disabled. Transcript UI (`CodeWindow`) was disconnected from real text injection.

## Solution

Four-step wizard with a **practice field** on step 3:

1. Welcome — Whisper model download
2. Microphone — level meter + permission prompt
3. Practice — focused textarea + hotkey coach; user must dictate via **global shortcut only**
4. Done — `MainHotkeyGuide` recap

### Backend

`prepare_onboarding_dictation` command:

- Stops mic probe and resumes global hotkey
- Stops active pipeline if needed (2s timeout)
- Called on practice step enter/exit, onboarding complete, and hook cleanup

### Frontend

- `useOnboardingPractice` — practice phase state machine
- `OnboardingPracticeField` — uncontrolled textarea synced after injection
- `OnboardingHotkeyCoach` — visual feedback per pipeline phase (no motion on key press)
- Continue on step 3 disabled until `practiceText` is non-empty

### Motion

- Step slides: existing `OnboardingStepTransition`
- Coach error: `animate-onboarding-coach-shake` (160ms)
- Success message: `successPopVariants` with `prefers-reduced-motion` guard
- Recording: green border on practice field (CSS transition only)

## Acceptance criteria

1. Step 3 starts recording via Alt+Space without UI button
2. Overlay pill visible while recording
3. Dictated text appears in practice textarea
4. Hotkey works after mic step without manual stop
5. Hotkey remains active when navigating steps
6. Hotkey works in external apps after onboarding
7. Reduced motion disables shake/slide embellishments
