# Installation

Calliop v1 cible **Windows 10/11**. Les builds macOS et Linux sont publiés en **expérimental** (non signés, non testés pour la v1).

## Télécharger

1. Ouvrez la page [Releases GitHub](https://github.com/Lappom/Calliop/releases).
2. Téléchargez l’artefact pour votre plateforme :
   - **Windows (recommandé)** : `Calliop_*_x64-setup.exe` (installateur NSIS) ou `.msi`
   - **macOS (expérimental)** : `.dmg`
   - **Linux (expérimental)** : `.AppImage` ou `.deb`

## Prérequis Windows

- [WebView2 Runtime Evergreen](https://developer.microsoft.com/microsoft-edge/webview2/) (souvent déjà installé avec Windows 11 ou Edge)
- Microphone
- Connexion internet **uniquement au premier lancement** pour télécharger les modèles Whisper/LLM (~500 Mo à ~2 Go selon les réglages)

## Installer (Windows)

### Installateur NSIS (`.exe`)

1. Lancez `Calliop_*_x64-setup.exe`.
2. Si **SmartScreen** affiche « Windows a protégé votre PC » : cliquez sur **Informations complémentaires**, puis **Exécuter quand même**.  
   Les binaires v1 ne sont **pas signés** (comportement attendu).
3. Suivez l’assistant ; l’installation se fait pour l’utilisateur courant.
4. Lancez Calliop depuis le menu Démarrer ou le raccourci bureau.

### Installateur MSI

Même procédure ; utile pour un déploiement silencieux en entreprise (`msiexec /i Calliop_*.msi`).

## Premier lancement (< 5 min)

1. **Onboarding** : autorisez le micro, testez une courte dictée.
2. **Téléchargement modèle** : Whisper `small` par défaut (~466 Mo). La barre de progression s’affiche dans l’app.
3. **Dictée** : raccourci par défaut **Alt + Espace** (toggle).
4. Optionnel : activez les auto-edits IA dans Paramètres → le modèle LLM se télécharge à la demande.

Les modèles sont stockés dans `%APPDATA%\com.calliop.app\models\` — pas dans l’installateur.

## Mises à jour

Dans **Paramètres → Avancé**, vous pouvez activer **Mises à jour automatiques** (désactivé par défaut). L’app vérifie alors les releases GitHub signées au démarrage.

Sinon, téléchargez manuellement la dernière release.

## Désinstallation

- **Paramètres Windows** → Applications → Calliop → Désinstaller  
- Les modèles et la configuration SQLite dans `%APPDATA%\com.calliop.app\` peuvent être supprimés manuellement si vous souhaitez libérer de l’espace.
