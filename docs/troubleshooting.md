# Troubleshooting

## Installer blocked by SmartScreen

v1 binaries are unsigned. Use **More info → Run anyway**. For enterprise deployment, plan an exception or code signing certificate.

## App won't start (black screen)

Install or repair [WebView2 Evergreen](https://developer.microsoft.com/microsoft-edge/webview2/).

## Microphone not working

1. Check Windows permissions: **Settings → Privacy → Microphone**.
2. In Calliop, rerun the microphone test from onboarding.
3. Close other apps that may be holding the microphone.

## Dictation text is not injected

1. Place the cursor in a text field (Notepad is a good test).
2. Check that the global shortcut does not conflict with another app.
3. Some secured apps (games, password fields) may block injection — Calliop falls back to the clipboard.

## Model download stuck

- Check your internet connection and any proxy settings.
- Models are hosted on Hugging Face (fallback if GitHub Releases is unavailable).
- Disk space: allow at least 2 GB for Whisper + LLM.

## High latency

1. Use Whisper `small` and LLM Qwen3 0.6B.
2. Settings → Advanced → **CPU only** if Vulkan GPU causes issues.
3. Close memory-heavy apps (16 GB RAM recommended).

## Updates

If automatic updates fail:

1. Confirm the option is enabled in Settings → Advanced.
2. Download manually from [GitHub Releases](https://github.com/Lappom/Calliop/releases).

## Logs

Launch Calliop from a terminal to see `stderr` messages:

```powershell
& "$env:LOCALAPPDATA\Programs\Calliop\Calliop.exe"
```

In development: `pnpm tauri:dev`.

## User data

Configuration and history: `%APPDATA%\com.calliop.app\`  
Models: `%APPDATA%\com.calliop.app\models\`
