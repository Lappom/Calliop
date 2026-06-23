# Contribuer à Calliop

Merci de votre intérêt pour Calliop. Ce projet vise une dictée vocale **100 % locale** — merci de respecter cette contrainte dans vos contributions.

## Workflow

1. Créer une branche depuis `main` : `feature/nom-court`, `fix/…`, `chore/…` ou `docs/…` (voir [docs/branch-strategy.md](docs/branch-strategy.md))
2. Une feature = une branche = une PR
3. Surligner une section dans [PLAN.md](PLAN.md) (ex. `### Phase 1 — …`) puis lancer **`/implement`** dans Cursor
4. Ouvrir une pull request — le template GitHub guide le plan de test
5. Supprimer la branche après merge

`main` est protégée : merge via PR avec CI verte (`frontend`, `rust`).

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
cargo build -p calliop-llm-worker
cargo run --release --bin benchmark-stt -- ../benchmarks/corpus/fr.json --cpu
```

## Releases GitHub

1. Mettre à jour `releases/v{version}.md` et les versions dans `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`.
2. Configurer les **secrets de dépôt** (Settings → Secrets and variables → Actions → *Repository secrets*, pas Environment secrets) :
   - `TAURI_SIGNING_PRIVATE_KEY` : contenu intégral de `src-tauri/.tauri/calliop.key`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` : mot de passe utilisé à la génération de la clé (obligatoire si la clé est chiffrée)
3. Vérifier l’alignement clé publique : `node scripts/verify-updater-pubkey.mjs` (`tauri.conf.json` doit correspondre à `calliop.key.pub`).
4. Pousser un tag : `git tag v0.1.2 && git push origin v0.1.2`
5. Le workflow [`.github/workflows/release.yml`](.github/workflows/release.yml) crée un brouillon de release avec les installateurs et `latest.json` pour l’auto-update.

Clé publique commitée : `src-tauri/.tauri/calliop.key.pub`.

## Conventions

- Lisez les règles dans [`.cursor/rules/`](.cursor/rules/) avant de coder
- Commentaires de code en **anglais**
- UI utilisateur en **français** (v1)
- Pas d'appels réseau pour le core (STT, LLM, injection) — modèles téléchargés séparément en Phase 1+

## Architecture

Le pipeline audio est documenté dans `.cursor/rules/audio-pipeline.mdc`. Chaque module Rust (`audio`, `stt`, `llm`, `inject`, `hotkey`, `store`, `pipeline`) a sa responsabilité — ne mélangez pas les concerns.

## Questions

Consultez [PLAN.md](PLAN.md) pour la roadmap et les décisions actées.
