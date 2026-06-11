# Guide utilisateur

## Démarrer une dictée

- Raccourci global par défaut : **Alt + Espace** (mode toggle).
- Maintenez la touche plus de 400 ms pour le mode **push-to-talk** (relâcher pour transcrire).
- Un overlay indique l’état : écoute, traitement, injection.

## Onboarding

Au premier lancement, l’assistant vous guide pour :

1. Autoriser l’accès au microphone.
2. Tester une dictée courte.
3. Valider que le texte est injecté dans l’application active.

## Paramètres

Ouvrez la fenêtre principale depuis l’icône de la barre des tâches.

### Général

- **Langue de l'interface** : français ou anglais (Paramètres → Général). Au premier lancement, la langue suit celle du système si elle est reconnue.
- **Langue de dictée** : français, anglais ou détection automatique (Whisper). Indépendante de la langue de l'interface.
- **Auto-edits IA** : nettoie fillers, ponctuation et reformulation légère (modèle local Qwen).
- **Apprentissage des corrections** : enrichit le dictionnaire quand vous corrigez le texte injecté.

### Modèles

- **Whisper** : `small` (rapide) ou `distil-fr-dec16` (meilleur français).
- **LLM** : Qwen3 0.6B / 1.7B / 4B selon latence vs qualité.
- **Backend** : automatique (GPU Vulkan si disponible) ou CPU uniquement.

### Raccourcis

Modifiez le raccourci global ; Échap annule la capture.

### Avancé

- **Mises à jour automatiques** : opt-in, vérifie GitHub au démarrage.
- **Lancer au démarrage** : icône tray au boot Windows.
- **Backend d’inférence** : forcer CPU si besoin.

## Fonctionnalités

| Fonction | Accès |
|----------|--------|
| Dictionnaire personnel | Fenêtre principale |
| Snippets vocaux | Fenêtre principale |
| Historique des dictées | Fenêtre principale |
| Insight (latence, mots/min) | Onglet Insight |
| Ton par application | Règles de contexte dans les paramètres |

## Tray

Clic gauche : ouvrir les paramètres. Menu : dictée, démarrage auto, quitter.

## Confidentialité

Toute la transcription et le post-traitement IA s’exécutent **localement**. Seuls le téléchargement des modèles (premier usage) et les mises à jour (si activées) utilisent le réseau.
