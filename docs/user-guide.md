# User guide

## Start dictation

- Default global shortcut: **Alt + Space** (toggle mode).
- Hold the key for more than 400 ms for **push-to-talk** mode (release to transcribe).
- An overlay shows the state: listening, processing, injection.

## Onboarding

On first launch, the assistant walks you through:

1. Granting microphone access.
2. Testing a short dictation.
3. Confirming text is injected into the active application.

## Settings

Open the main window from the system tray icon.

### General

- **UI language**: French or English (Settings → General). On first launch, the language follows the system locale when recognized.
- **Dictation language**: French, English, or automatic detection (Whisper). Independent of the UI language.
- **AI auto-edits**: removes fillers, fixes punctuation, and applies light rephrasing (local Qwen model).
- **Learn corrections**: enriches the personal dictionary when you edit injected text.

### Models

- **Whisper**: `small` (fast) or `distil-fr-dec16` (better French).
- **LLM**: Qwen3 0.6B / 1.7B / 4B depending on latency vs. quality.
- **Backend**: automatic (Vulkan GPU when available) or CPU only.

### Shortcuts

Change the global shortcut; Escape cancels capture.

### Advanced

- **Automatic updates**: opt-in, checks GitHub on startup.
- **Launch at startup**: tray icon on Windows boot.
- **Inference backend**: force CPU if needed.

## Features

| Feature | Access |
|---------|--------|
| Personal dictionary | Main window |
| Voice snippets | Main window |
| Dictation history | Main window |
| Insights (latency, words/min) | Insights tab |
| Per-app tone | Context rules in settings |

## Tray

Left click: open settings. Menu: dictation, auto-start, quit.

## Privacy

All transcription and AI post-processing run **locally**. Only model downloads (first use) and updates (if enabled) use the network.
