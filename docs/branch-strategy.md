# Stratégie de branches

## Branches permanentes

| Branche | Rôle | Protection |
| ------- | ---- | ---------- |
| `main` | Code stable, releases depuis tags `v*` | PR obligatoire, CI verte, pas de force-push |

Il n'y a pas de branche `develop` : le flux est **trunk-based** sur `main`.

## Branches de travail

| Préfixe | Usage | Exemple |
| ------- | ----- | ------- |
| `feature/` | Nouvelle fonctionnalité ou lot de travail | `feature/phase-3b-dictionary` |
| `fix/` | Correction de bug | `fix/inject-clipboard-restore` |
| `chore/` | CI, dépendances, tooling | `chore/dependabot-rust` |
| `docs/` | Documentation seule | `docs/installation-macos` |

Règles :

1. Toujours créer depuis `main` à jour : `git fetch origin && git checkout -b feature/ma-feature origin/main`
2. Une branche = un objectif clair = une PR
3. Rebaser ou merger `main` régulièrement pour limiter les conflits
4. Supprimer la branche distante après merge

## Releases

Les releases sont des **tags annotés** sur `main` :

```powershell
git tag -a v0.1.9 -m "v0.1.9"
git push origin v0.1.9
```

Le workflow `.github/workflows/release.yml` publie les installateurs et `latest.json`.

## Branches historiques

Les branches `feature/phase-*` datent du développement initial. Une fois mergées dans `main`, elles peuvent être supprimées :

```powershell
# Local
git branch -d feature/phase-1-mvp

# Distant (après vérification que main contient le travail)
git push origin --delete feature/phase-1-mvp
```

## Contributeurs externes

1. Fork du dépôt
2. Branche `feature/` ou `fix/` sur le fork
3. PR vers `main` du dépôt upstream
4. CI doit passer avant merge

Voir [CONTRIBUTING.md](../CONTRIBUTING.md) pour l'environnement de dev et les vérifications.
