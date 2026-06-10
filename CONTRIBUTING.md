# Contribuer à Calliop

Merci de votre intérêt pour Calliop. Ce projet vise une dictée vocale **100 % locale** — merci de respecter cette contrainte dans vos contributions.

## Workflow

1. Créer une branche depuis `main` : `feature/nom-court`
2. Une feature = une branche = une session de dev
3. Surligner une section dans [PLAN.md](PLAN.md) (ex. `### Phase 1 — …`) puis lancer **`/implement`** dans Cursor
4. Ouvrir une pull request avec une description claire et un plan de test

## Environnement de dev

```powershell
pnpm install
pnpm tauri:dev
```

Sur Windows, `tauri:dev` garantit que CMake 4.x est disponible et compile le sidecar LLM (`calliop-llm-worker`).

## Vérifications avant PR

```powershell
# Frontend
pnpm typecheck
pnpm lint

# Rust (depuis src-tauri/)
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Tests manuels par module (depuis `src-tauri/`) :

```powershell
cargo run --bin test-audio -- record 3s output.wav
cargo run --bin test-stt -- output.wav
cargo run --bin test-inject -- "Hello world"
cargo run --bin test-llm -- "euh bonjour donc voilà"
cargo build --features llm-worker --bin calliop-llm-worker
```

## Conventions

- Lisez les règles dans [`.cursor/rules/`](.cursor/rules/) avant de coder
- Commentaires de code en **anglais**
- UI utilisateur en **français** (v1)
- Pas d'appels réseau pour le core (STT, LLM, injection) — modèles téléchargés séparément en Phase 1+

## Architecture

Le pipeline audio est documenté dans `.cursor/rules/audio-pipeline.mdc`. Chaque module Rust (`audio`, `stt`, `llm`, `inject`, `hotkey`, `store`, `pipeline`) a sa responsabilité — ne mélangez pas les concerns.

## Questions

Consultez [PLAN.md](PLAN.md) pour la roadmap et les décisions actées.
