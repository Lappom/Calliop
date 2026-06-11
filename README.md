# Calliop

Dictée vocale universelle, **100 % locale** — clone open source de [Wispr Flow](https://wisprflow.ai/).

Calliop permet de dicter dans n'importe quelle application via un raccourci global, avec transcription locale et post-traitement IA optionnel. Aucune dépendance cloud pour le fonctionnement core.

## Télécharger

Binaires Windows (NSIS / MSI) sur [GitHub Releases](https://github.com/Lappom/Calliop/releases). Guide : [docs/installation.md](docs/installation.md).

- [Guide utilisateur](docs/guide-utilisateur.md)
- [Dépannage](docs/depannage.md)
- [Benchmarks STT](docs/BENCHMARKS.md)

## Prérequis (développement, Windows v1)

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 20+
- [pnpm](https://pnpm.io/installation)
- [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) (Runtime Evergreen)
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) avec workload « Desktop development with C++ »
- [CMake](https://cmake.org/download/) 4.x (requis pour compiler `whisper-rs` ; `winget install Kitware.CMake`)

## Démarrage rapide

```powershell
pnpm install
pnpm tauri:dev
```

> **Note :** utilisez `pnpm tauri:dev` (et non `pnpm tauri dev`) sur Windows — ce script installe CMake 4.x si nécessaire pour compiler `whisper-rs`.

L'app **Calliop** s'ouvre avec une icône dans la barre des tâches. Au premier lancement, le modèle Whisper `small` est téléchargé (~466 Mo). Utilisez **Alt + Espace** pour démarrer / arrêter une dictée.

## Tests CLI (modules isolés)

Depuis `src-tauri/` :

```powershell
cargo run --bin test-audio -- record 3s output.wav
cargo run --bin test-stt -- output.wav
cargo run --bin test-inject -- "Bonjour, ceci est un test"
```

## Workflow Cursor

1. Ouvrir [PLAN.md](PLAN.md) et surligner la section à implémenter (ex. une phase)
2. Dans le chat Agent, taper **`/implement`**
3. L'agent exécute la section de manière optimisée (une phase à la fois)

Voir [CONTRIBUTING.md](CONTRIBUTING.md) pour le détail.

## Structure du projet

```
src-tauri/src/
  audio/       # capture, VAD
  stt/         # whisper bindings
  llm/         # post-processing local
  inject/      # text injection
  hotkey/      # global shortcuts
  store/       # SQLite
  pipeline/    # orchestration
src/
  components/  # overlay, settings, onboarding
  hooks/
  lib/
```

Voir [PLAN.md](PLAN.md) pour la roadmap complète.

## Distribution (v1)

Les binaires Windows v1 ne sont **pas signés** (pas de certificat code signing). Windows SmartScreen peut afficher un avertissement à l'installation — voir [docs/installation.md](docs/installation.md).

Build installateur local : `pnpm tauri build --features gpu` (artefacts dans `src-tauri/target/release/bundle/`).

## Licence

AGPL-3.0 — voir [LICENSE](LICENSE).
