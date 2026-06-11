# Installation

Calliop v1 targets **Windows 10/11**. macOS and Linux builds are published as **experimental** (unsigned, not tested for v1).

## Download

1. Open the [GitHub Releases](https://github.com/Lappom/Calliop/releases) page.
2. Download the artifact for your platform:
   - **Windows (recommended)**: `Calliop_*_x64-setup.exe` (NSIS installer) or `.msi`
   - **macOS (experimental)**: `.dmg`
   - **Linux (experimental)**: `.AppImage` or `.deb`

## Windows prerequisites

- [WebView2 Runtime Evergreen](https://developer.microsoft.com/microsoft-edge/webview2/) (often already installed with Windows 11 or Edge)
- Microphone
- Internet connection **only on first launch** to download Whisper/LLM models (~500 MB to ~2 GB depending on settings)

## Install (Windows)

### NSIS installer (`.exe`)

1. Run `Calliop_*_x64-setup.exe`.
2. If **SmartScreen** shows "Windows protected your PC": click **More info**, then **Run anyway**.  
   v1 binaries are **unsigned** (expected behavior).
3. Follow the wizard; installation is per-user.
4. Launch Calliop from the Start menu or desktop shortcut.

### MSI installer

Same procedure; useful for silent enterprise deployment (`msiexec /i Calliop_*.msi`).

## First launch (< 5 min)

1. **Onboarding**: grant microphone access, test a short dictation.
2. **Model download**: Whisper `small` by default (~466 MB). Progress is shown in the app.
3. **Dictation**: default shortcut **Alt + Space** (toggle).
4. Optional: enable AI auto-edits in Settings → the LLM model downloads on demand.

Models are stored in `%APPDATA%\com.calliop.app\models\` — not bundled in the installer.

## Updates

In **Settings → Advanced**, you can enable **Automatic updates** (disabled by default). The app then checks for signed GitHub releases on startup.

Otherwise, download the latest release manually.

## Uninstall

- **Windows Settings** → Apps → Calliop → Uninstall  
- Models and SQLite configuration in `%APPDATA%\com.calliop.app\` can be removed manually to free disk space.
