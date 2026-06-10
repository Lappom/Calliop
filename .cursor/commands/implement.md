---
description: Implémenter de manière optimisée la section sélectionnée de PLAN.md (une phase à la fois).
---

# /implement — Implémenter une section du plan

Tu implémentes **uniquement** la section de [PLAN.md](PLAN.md) que l'utilisateur a sélectionnée (ou référencée avec `@PLAN.md`).

Réponds en **français**, de façon concise.

## Preflight

1. **Section cible** — Identifie le bloc exact : titre (`### Phase N — …`), objectif, checklist `- [ ]`, critère de done, sous-sections (`####`).
2. **Sélection manquante** — Si aucune sélection n'est fournie, demande à l'utilisateur de surligner une section dans `PLAN.md` puis de relancer `/implement`.
3. **Une phase à la fois** — Refuse d'implémenter plusieurs phases ou une section trop large. Propose de découper si nécessaire.
4. **Contexte obligatoire** — Lis avant de coder :
   - [PLAN.md](PLAN.md) section « Décisions prises » et « Conséquences sur le plan »
   - Toutes les règles pertinentes dans [.cursor/rules/](.cursor/rules/) (`offline-constraint`, `audio-pipeline`, `rust-ts-conventions`, `testing-strategy`)
   - [DESIGN.md](DESIGN.md) seulement si la section touche l'UI (overlay, settings, onboarding)
5. **État du repo** — `git status` : note la branche et les changements non commités. Recommande une branche `feature/phase-N-court-resume` si on est sur `main`.

Arrête avec un message clair si la section est déjà entièrement cochée `[x]` — propose la phase suivante.

## Plan (court)

Avant d'écrire du code, produis un **plan d'exécution en 5–10 puces** :

- Ordre des fichiers / modules à créer ou modifier
- Dépendances crates/npm à ajouter (si any)
- Commandes de validation prévues
- Ce qui est **hors scope** explicite (features des phases futures)

Pour une section simple (< 5 tâches, pas de décision d'architecture), enchaîne directement après ce plan court — **ne passe pas en mode Plan Cursor** sauf ambiguïté bloquante.

Pose **1–2 questions maximum** seulement si un choix change l'architecture (ex. lib STT, stratégie injection).

## Implémentation optimisée

Applique cet ordre :

1. **Structure** — Créer ou étendre les modules Rust (`src-tauri/src/{audio,stt,llm,inject,hotkey,store,pipeline}/`) et dossiers frontend (`src/components`, `hooks`, `lib`) selon le plan.
2. **Core Rust d'abord** — Logique métier, IPC Tauri, pas de lourde logique ML dans le webview.
3. **UI ensuite** — Composants React minimaux ; UI en français (v1).
4. **Tests** — Smoke tests unitaires + CLI bins si la section le demande (`test-audio`, etc.).
5. **Docs** — Mettre à jour README / CONTRIBUTING seulement si la section l'exige.

### Contraintes non négociables

- **100 % offline** pour le core — pas d'appels réseau temporaires (cf. `offline-constraint.mdc`)
- **Windows v1** — tester sur Windows ; ne pas bloquer l'architecture cross-platform
- **Scope minimal** — pas de refacto hors section, pas de features des phases suivantes
- **Commentaires** en anglais dans le code
- **Ne pas modifier** les fichiers de plan Cursor (`.cursor/plans/*.plan.md`)

## Vérification

Exécute les checks pertinents pour la section :

```powershell
# Frontend
pnpm typecheck
pnpm lint

# Rust (depuis src-tauri/)
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Si la section inclut une app runnable :

```powershell
pnpm tauri dev
# ou pnpm tauri build pour valider le bundle
```

Itère jusqu'à ce que les checks passent ou qu'un blocker environnement (WebView2, MSVC) soit identifié avec mitigation documentée.

## Clôture

1. **Cocher la checklist** — Dans [PLAN.md](PLAN.md), remplace `- [ ]` par `- [x]` pour les tâches réellement terminées de la section ciblée uniquement.
2. **Règles Cursor** — Si une décision d'architecture a été prise, mettre à jour le fichier `.cursor/rules/` concerné.
3. **Résumé** — Format :

```
## Résultat
- **Section** : Phase N — …
- **Statut** : done | partiel | bloqué
- **Fichiers clés** : …
- **Validations** : …
- **Prochaine étape** : …
```

4. **Commit** — Propose un message de commit ; ne commit **que** si l'utilisateur le demande explicitement.

## Anti-patterns (PLAN.md)

- ❌ Implémenter toute l'application en une fois
- ❌ Mélanger plusieurs phases dans la même session
- ❌ Ignorer les tests CLI / unitaires des modules Rust
- ❌ Ajouter des dépendances cloud « temporaires »
