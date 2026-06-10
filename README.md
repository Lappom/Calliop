# Calliop

Dictée vocale universelle, **100 % locale** — clone open source de [Wispr Flow](https://wisprflow.ai/).

Calliop permet de dicter dans n'importe quelle application via un raccourci global, avec transcription locale et post-traitement IA optionnel. Aucune dépendance cloud pour le fonctionnement core.

## Prérequis (Windows v1)

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 20+
- [pnpm](https://pnpm.io/installation)
- [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) (Runtime Evergreen)
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) avec workload « Desktop development with C++ »

## Démarrage rapide

```powershell
pnpm install
pnpm tauri dev
```

Une fenêtre vide **Calliop** s'ouvre avec une icône dans la barre des tâches (system tray).

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

Les binaires Windows v1 ne sont **pas signés** (pas de certificat code signing). Windows SmartScreen peut afficher un avertissement à l'installation — comportement attendu en développement.

## Licence

AGPL-3.0 — voir [LICENSE](LICENSE).
