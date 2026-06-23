# Politique de sécurité

## Versions supportées

| Version | Supportée |
| ------- | --------- |
| Dernière release (`v*`) | Oui |
| Branche `main` | Développement uniquement |
| Versions antérieures | Non |

## Signaler une vulnérabilité

**Ne pas ouvrir d'issue publique** pour un problème de sécurité.

Envoyez un rapport privé via [GitHub Security Advisories](https://github.com/Lappom/Calliop/security/advisories/new) ou par e-mail à l'auteur du dépôt (voir le profil [Lappom](https://github.com/Lappom)).

Incluez si possible :

- Description du problème et impact
- Étapes de reproduction
- Version affectée et plateforme
- Correctif suggéré (optionnel)

## Délai de réponse

- Accusé de réception : sous 72 h
- Correctif ou plan d'action : selon la gravité, généralement sous 30 jours

## Périmètre

- Application desktop Tauri (injection, permissions, mise à jour auto)
- Sidecar LLM local et chargement de modèles
- Fuites de données audio/texte vers le réseau hors téléchargement de modèles / auto-update documentés

Hors périmètre : vulnérabilités dans des dépendances tierces déjà corrigées en amont, sauf si l'exposition dans Calliop est spécifique.
